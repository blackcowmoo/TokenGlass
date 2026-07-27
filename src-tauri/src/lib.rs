use serde::Serialize;
use serde_json::Value;
use std::{
    collections::HashMap,
    sync::{mpsc, Arc, Mutex},
    time::Duration,
};
use tauri_plugin_shell::{
    process::{CommandChild, CommandEvent},
    ShellExt,
};

const OPENAI_API_BASE: &str = "https://api.openai.com/v1/organization";

struct RunningAppServer {
    _child: CommandChild,
    next_id: u64,
    waiters: Arc<Mutex<HashMap<u64, mpsc::Sender<Value>>>>,
}

#[derive(Default)]
struct CodexAppServer {
    server: Mutex<Option<RunningAppServer>>,
}

impl CodexAppServer {
    fn start_if_needed(&self, app: &tauri::AppHandle) -> Result<(), String> {
        let mut server = self
            .server
            .lock()
            .map_err(|_| "Codex 연결 상태를 잠글 수 없습니다.".to_string())?;
        if server.is_some() {
            return Ok(());
        }

        let (mut receiver, child) = app
            .shell()
            .sidecar("codex")
            .map_err(|error| format!("번들된 Codex App Server를 찾을 수 없습니다: {error}"))?
            .arg("app-server")
            .spawn()
            .map_err(|error| format!("번들된 Codex App Server를 시작할 수 없습니다: {error}"))?;
        let waiters = Arc::new(Mutex::new(HashMap::<u64, mpsc::Sender<Value>>::new()));
        let reader_waiters = Arc::clone(&waiters);

        std::thread::spawn(move || {
            while let Some(event) = tauri::async_runtime::block_on(receiver.recv()) {
                let CommandEvent::Stdout(bytes) = event else {
                    continue;
                };
                let Ok(line) = String::from_utf8(bytes) else {
                    continue;
                };
                let Ok(message) = serde_json::from_str::<Value>(&line) else {
                    continue;
                };
                let Some(id) = message.get("id").and_then(Value::as_u64) else {
                    continue;
                };
                if let Ok(mut pending) = reader_waiters.lock() {
                    if let Some(sender) = pending.remove(&id) {
                        let _ = sender.send(message);
                    }
                }
            }
        });

        *server = Some(RunningAppServer {
            _child: child,
            next_id: 1,
            waiters,
        });
        drop(server);
        self.request("initialize", serde_json::json!({
            "clientInfo": { "name": "tokenglass", "title": "TokenGlass", "version": env!("CARGO_PKG_VERSION") }
        }))?;
        self.notify("initialized", serde_json::json!({}))
    }

    fn notify(&self, method: &str, params: Value) -> Result<(), String> {
        let mut server = self
            .server
            .lock()
            .map_err(|_| "Codex 연결 상태를 잠글 수 없습니다.".to_string())?;
        let running = server
            .as_mut()
            .ok_or("Codex App Server가 시작되지 않았습니다.")?;
        let message = serde_json::json!({ "method": method, "params": params });
        running
            ._child
            .write(format!("{message}\n").as_bytes())
            .map_err(|error| format!("Codex에 요청을 보낼 수 없습니다: {error}"))
    }

    fn request(&self, method: &str, params: Value) -> Result<Value, String> {
        let receiver = {
            let mut server = self
                .server
                .lock()
                .map_err(|_| "Codex 연결 상태를 잠글 수 없습니다.".to_string())?;
            let running = server
                .as_mut()
                .ok_or("Codex App Server가 시작되지 않았습니다.")?;
            let id = running.next_id;
            running.next_id += 1;
            let (sender, receiver) = mpsc::channel();
            running
                .waiters
                .lock()
                .map_err(|_| "Codex 응답 대기열을 잠글 수 없습니다.".to_string())?
                .insert(id, sender);
            let message = serde_json::json!({ "method": method, "id": id, "params": params });
            if let Err(error) = running._child.write(format!("{message}\n").as_bytes()) {
                if let Ok(mut pending) = running.waiters.lock() {
                    pending.remove(&id);
                }
                return Err(format!("Codex에 요청을 보낼 수 없습니다: {error}"));
            }
            receiver
        };
        let message = receiver
            .recv_timeout(Duration::from_secs(20))
            .map_err(|_| "Codex 응답 시간이 초과되었습니다.".to_string())?;
        if let Some(error) = message.get("error") {
            return Err(error
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("Codex 요청에 실패했습니다.")
                .to_string());
        }
        message
            .get("result")
            .cloned()
            .ok_or("Codex 응답에 결과가 없습니다.".to_string())
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ChatGptLogin {
    login_id: String,
    auth_url: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SubscriptionLimit {
    limit_id: String,
    label: Option<String>,
    used_percent: Option<f64>,
    window_duration_mins: Option<u64>,
    resets_at: Option<i64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DailyTokenUsage {
    start_date: String,
    tokens: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ChatGptSubscriptionUsage {
    email: Option<String>,
    plan_type: Option<String>,
    limits: Vec<SubscriptionLimit>,
    lifetime_tokens: Option<u64>,
    peak_daily_tokens: Option<u64>,
    daily_usage: Vec<DailyTokenUsage>,
}

#[tauri::command]
fn start_chatgpt_login(
    app: tauri::AppHandle,
    app_server: tauri::State<'_, CodexAppServer>,
) -> Result<ChatGptLogin, String> {
    app_server.start_if_needed(&app)?;
    let result = app_server.request(
        "account/login/start",
        serde_json::json!({
            "type": "chatgpt",
            "useHostedLoginSuccessPage": true,
            "appBrand": "chatgpt"
        }),
    )?;
    Ok(ChatGptLogin {
        login_id: result
            .get("loginId")
            .and_then(Value::as_str)
            .ok_or("로그인 ID를 받지 못했습니다.")?
            .to_string(),
        auth_url: result
            .get("authUrl")
            .and_then(Value::as_str)
            .ok_or("로그인 URL을 받지 못했습니다.")?
            .to_string(),
    })
}

#[tauri::command]
fn fetch_chatgpt_subscription_usage(
    app: tauri::AppHandle,
    app_server: tauri::State<'_, CodexAppServer>,
) -> Result<ChatGptSubscriptionUsage, String> {
    app_server.start_if_needed(&app)?;
    let account =
        app_server.request("account/read", serde_json::json!({ "refreshToken": true }))?;
    let account_info = account
        .get("account")
        .ok_or("ChatGPT에 먼저 로그인하세요.")?;
    if account_info.get("type").and_then(Value::as_str) != Some("chatgpt") {
        return Err("ChatGPT OAuth 로그인이 필요합니다.".to_string());
    }
    let rate_limits = app_server.request("account/rateLimits/read", serde_json::json!({}))?;
    let usage = app_server.request("account/usage/read", serde_json::json!({}))?;
    let mut limits = Vec::new();
    if let Some(items) = rate_limits
        .get("rateLimitsByLimitId")
        .and_then(Value::as_object)
    {
        for (limit_id, item) in items {
            let primary = item.get("primary");
            limits.push(SubscriptionLimit {
                limit_id: limit_id.clone(),
                label: item
                    .get("limitName")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                used_percent: primary
                    .and_then(|value| value.get("usedPercent"))
                    .and_then(Value::as_f64),
                window_duration_mins: primary
                    .and_then(|value| value.get("windowDurationMins"))
                    .and_then(Value::as_u64),
                resets_at: primary
                    .and_then(|value| value.get("resetsAt"))
                    .and_then(Value::as_i64),
            });
        }
    }
    if limits.is_empty() {
        if let Some(item) = rate_limits.get("rateLimits") {
            let primary = item.get("primary");
            limits.push(SubscriptionLimit {
                limit_id: item
                    .get("limitId")
                    .and_then(Value::as_str)
                    .unwrap_or("codex")
                    .to_string(),
                label: item
                    .get("limitName")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                used_percent: primary
                    .and_then(|value| value.get("usedPercent"))
                    .and_then(Value::as_f64),
                window_duration_mins: primary
                    .and_then(|value| value.get("windowDurationMins"))
                    .and_then(Value::as_u64),
                resets_at: primary
                    .and_then(|value| value.get("resetsAt"))
                    .and_then(Value::as_i64),
            });
        }
    }
    let daily_usage = usage
        .get("dailyUsageBuckets")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|bucket| DailyTokenUsage {
            start_date: bucket
                .get("startDate")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            tokens: bucket.get("tokens").and_then(Value::as_u64).unwrap_or(0),
        })
        .collect();

    Ok(ChatGptSubscriptionUsage {
        email: account_info
            .get("email")
            .and_then(Value::as_str)
            .map(str::to_owned),
        plan_type: account_info
            .get("planType")
            .and_then(Value::as_str)
            .map(str::to_owned),
        limits,
        lifetime_tokens: usage
            .pointer("/summary/lifetimeTokens")
            .and_then(Value::as_u64),
        peak_daily_tokens: usage
            .pointer("/summary/peakDailyTokens")
            .and_then(Value::as_u64),
        daily_usage,
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ModelUsage {
    name: String,
    tokens: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OpenAiUsage {
    total_billed: f64,
    today_usage: f64,
    input_tokens: u64,
    output_tokens: u64,
    models: Vec<ModelUsage>,
    period_start: i64,
    period_end: i64,
}

fn api_error(response: reqwest::Response) -> impl std::future::Future<Output = String> {
    async move {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        let message = serde_json::from_str::<Value>(&body)
            .ok()
            .and_then(|value| {
                value
                    .pointer("/error/message")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            })
            .unwrap_or(body);
        format!("OpenAI Usage API 요청 실패 ({status}): {message}")
    }
}

#[tauri::command]
async fn fetch_openai_usage(admin_key: String) -> Result<OpenAiUsage, String> {
    if admin_key.trim().is_empty() {
        return Err("OpenAI 조직 관리자 API 키를 입력하세요.".to_string());
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| "현재 시간을 확인할 수 없습니다.".to_string())?
        .as_secs() as i64;
    let utc_now = time::OffsetDateTime::from_unix_timestamp(now)
        .map_err(|_| "현재 시간을 변환할 수 없습니다.".to_string())?;
    let period_start = time::Date::from_calendar_date(utc_now.year(), utc_now.month(), 1)
        .map_err(|_| "이번 달 시작일을 계산할 수 없습니다.".to_string())?
        .with_time(time::Time::MIDNIGHT)
        .assume_utc()
        .unix_timestamp();
    let today_start =
        time::Date::from_calendar_date(utc_now.year(), utc_now.month(), utc_now.day())
            .map_err(|_| "오늘 시작일을 계산할 수 없습니다.".to_string())?
            .with_time(time::Time::MIDNIGHT)
            .assume_utc()
            .unix_timestamp();

    let client = reqwest::Client::new();
    let usage_response = client
        .get(format!("{OPENAI_API_BASE}/usage/completions"))
        .bearer_auth(admin_key.trim())
        .query(&[
            ("start_time", period_start.to_string()),
            ("end_time", now.to_string()),
            ("bucket_width", "1d".to_string()),
            ("limit", "31".to_string()),
            ("group_by", "model".to_string()),
        ])
        .send()
        .await
        .map_err(|error| format!("OpenAI에 연결할 수 없습니다: {error}"))?;
    if !usage_response.status().is_success() {
        return Err(api_error(usage_response).await);
    }
    let usage: Value = usage_response
        .json()
        .await
        .map_err(|error| format!("사용량 응답을 읽을 수 없습니다: {error}"))?;

    let costs_response = client
        .get(format!("{OPENAI_API_BASE}/costs"))
        .bearer_auth(admin_key.trim())
        .query(&[
            ("start_time", period_start.to_string()),
            ("end_time", now.to_string()),
            ("bucket_width", "1d".to_string()),
            ("limit", "31".to_string()),
        ])
        .send()
        .await
        .map_err(|error| format!("OpenAI에 연결할 수 없습니다: {error}"))?;
    if !costs_response.status().is_success() {
        return Err(api_error(costs_response).await);
    }
    let costs: Value = costs_response
        .json()
        .await
        .map_err(|error| format!("비용 응답을 읽을 수 없습니다: {error}"))?;

    let mut input_tokens = 0_u64;
    let mut output_tokens = 0_u64;
    let mut model_tokens = std::collections::BTreeMap::<String, u64>::new();
    for bucket in usage
        .get("data")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        for result in bucket
            .get("results")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let input = result
                .get("input_tokens")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            let output = result
                .get("output_tokens")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            input_tokens += input;
            output_tokens += output;
            let model = result
                .get("model")
                .and_then(Value::as_str)
                .unwrap_or("Other");
            *model_tokens.entry(model.to_string()).or_default() += input + output;
        }
    }

    let mut total_billed = 0.0_f64;
    let mut today_usage = 0.0_f64;
    for bucket in costs
        .get("data")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let is_today = bucket
            .get("start_time")
            .and_then(Value::as_i64)
            .unwrap_or(0)
            >= today_start;
        for result in bucket
            .get("results")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let amount = result
                .pointer("/amount/value")
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            total_billed += amount;
            if is_today {
                today_usage += amount;
            }
        }
    }

    let mut models: Vec<ModelUsage> = model_tokens
        .into_iter()
        .map(|(name, tokens)| ModelUsage { name, tokens })
        .collect();
    models.sort_by(|left, right| right.tokens.cmp(&left.tokens));

    Ok(OpenAiUsage {
        total_billed,
        today_usage,
        input_tokens,
        output_tokens,
        models,
        period_start,
        period_end: now,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(CodexAppServer::default())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            fetch_openai_usage,
            start_chatgpt_login,
            fetch_chatgpt_subscription_usage
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
