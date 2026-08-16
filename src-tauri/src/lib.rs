use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    sync::{mpsc, Arc, Mutex},
    time::Duration,
};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent,
};
use tauri_plugin_shell::{
    process::{CommandChild, CommandEvent},
    ShellExt,
};
use tauri_plugin_store::StoreExt;
use tokio::sync::Mutex as AsyncMutex;

const OPENAI_API_BASE: &str = "https://api.openai.com/v1/organization";
const OPENAI_USAGE_CACHE_TTL_SECONDS: i64 = 5 * 60;

struct RunningAppServer {
    _child: CommandChild,
    next_id: u64,
    waiters: Arc<Mutex<HashMap<u64, mpsc::Sender<Value>>>>,
}

#[derive(Default)]
struct CodexAppServer {
    server: Mutex<Option<RunningAppServer>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeDiagnostics {
    app_version: String,
    operating_system: String,
    architecture: String,
    test_mode: bool,
    sidecar_available: bool,
    sidecar_running: bool,
}

#[cfg(test)]
fn sanitize_diagnostic_text(value: &str) -> String {
    let mut redacted = value.to_string();
    for prefix in ["sk-", "Bearer "] {
        while let Some(start) = redacted.find(prefix) {
            let suffix = &redacted[start + prefix.len()..];
            let end = suffix
                .find(|character: char| {
                    character.is_whitespace() || matches!(character, ',' | ';' | '"')
                })
                .unwrap_or(suffix.len());
            redacted.replace_range(start..start + prefix.len() + end, "[redacted]");
        }
    }
    redacted
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

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ModelUsage {
    name: String,
    tokens: u64,
}

#[derive(Clone, Serialize)]
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

#[derive(Clone)]
struct CachedOpenAiUsage {
    key_fingerprint: String,
    usage: OpenAiUsage,
    fetched_at: i64,
    generation: u64,
}

#[derive(Default)]
struct OpenAiUsageState {
    cache: AsyncMutex<Option<CachedOpenAiUsage>>,
    refresh_gate: AsyncMutex<()>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OpenAiUsageSnapshot {
    usage: OpenAiUsage,
    fetched_at: i64,
    source: String,
    stale: bool,
    refresh_error: Option<String>,
}

fn openai_key_fingerprint(admin_key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(admin_key.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn current_unix_timestamp() -> Result<i64, String> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| "현재 시간을 확인할 수 없습니다.".to_string())
        .map(|duration| duration.as_secs() as i64)
}

fn cache_is_fresh(entry: &CachedOpenAiUsage, now: i64) -> bool {
    now.saturating_sub(entry.fetched_at) < OPENAI_USAGE_CACHE_TTL_SECONDS
}

fn cache_matches_key(entry: &CachedOpenAiUsage, key_fingerprint: &str) -> bool {
    entry.key_fingerprint == key_fingerprint
}

fn should_return_fresh_cache(
    entry: &CachedOpenAiUsage,
    key_fingerprint: &str,
    force_refresh: bool,
    now: i64,
) -> bool {
    cache_matches_key(entry, key_fingerprint) && !force_refresh && cache_is_fresh(entry, now)
}

fn refresh_completed_while_waiting(
    entry: &CachedOpenAiUsage,
    key_fingerprint: &str,
    observed_generation: Option<u64>,
) -> bool {
    cache_matches_key(entry, key_fingerprint)
        && observed_generation
            .map(|generation| entry.generation > generation)
            .unwrap_or(false)
}

fn usage_snapshot(
    entry: &CachedOpenAiUsage,
    source: &str,
    stale: bool,
    refresh_error: Option<String>,
) -> OpenAiUsageSnapshot {
    OpenAiUsageSnapshot {
        usage: entry.usage.clone(),
        fetched_at: entry.fetched_at,
        source: source.to_string(),
        stale,
        refresh_error,
    }
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

fn calculate_bounds_with_offset(now: i64, offset: time::UtcOffset) -> (i64, i64) {
    let local_now = time::OffsetDateTime::from_unix_timestamp(now)
        .map(|dt| dt.to_offset(offset))
        .unwrap_or_else(|_| time::OffsetDateTime::now_utc());

    let period_start = time::Date::from_calendar_date(local_now.year(), local_now.month(), 1)
        .map(|date| {
            date.with_time(time::Time::MIDNIGHT)
                .assume_offset(offset)
                .unix_timestamp()
        })
        .unwrap_or_else(|_| {
            let utc_now = time::OffsetDateTime::from_unix_timestamp(now)
                .unwrap_or_else(|_| time::OffsetDateTime::now_utc());
            time::Date::from_calendar_date(utc_now.year(), utc_now.month(), 1)
                .map(|date| {
                    date.with_time(time::Time::MIDNIGHT)
                        .assume_utc()
                        .unix_timestamp()
                })
                .unwrap_or(now - 30 * 86400)
        });

    let today_start =
        time::Date::from_calendar_date(local_now.year(), local_now.month(), local_now.day())
            .map(|date| {
                date.with_time(time::Time::MIDNIGHT)
                    .assume_offset(offset)
                    .unix_timestamp()
            })
            .unwrap_or_else(|_| {
                let utc_now = time::OffsetDateTime::from_unix_timestamp(now)
                    .unwrap_or_else(|_| time::OffsetDateTime::now_utc());
                time::Date::from_calendar_date(utc_now.year(), utc_now.month(), utc_now.day())
                    .map(|date| {
                        date.with_time(time::Time::MIDNIGHT)
                            .assume_utc()
                            .unix_timestamp()
                    })
                    .unwrap_or(now - 86400)
            });

    (period_start, today_start)
}

fn calculate_period_and_today_bounds(now: i64) -> (i64, i64) {
    let local_offset = time::UtcOffset::current_local_offset().unwrap_or(time::UtcOffset::UTC);
    calculate_bounds_with_offset(now, local_offset)
}

async fn fetch_openai_usage_from_api(admin_key: &str) -> Result<OpenAiUsage, String> {
    if admin_key.trim().is_empty() {
        return Err("OpenAI 조직 관리자 API 키를 입력하세요.".to_string());
    }

    let now = current_unix_timestamp()?;
    let (period_start, today_start) = calculate_period_and_today_bounds(now);

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
        let start_time = bucket
            .get("start_time")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let end_time = bucket
            .get("end_time")
            .and_then(Value::as_i64)
            .unwrap_or(start_time + 86400);
        let is_today = start_time >= today_start || end_time > today_start;
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

#[tauri::command]
async fn fetch_openai_usage(
    admin_key: String,
    force_refresh: Option<bool>,
    usage_state: tauri::State<'_, OpenAiUsageState>,
) -> Result<OpenAiUsageSnapshot, String> {
    let admin_key = admin_key.trim();
    if admin_key.is_empty() {
        return Err("OpenAI 조직 관리자 API 키를 입력하세요.".to_string());
    }

    let key_fingerprint = openai_key_fingerprint(admin_key);
    let force_refresh = force_refresh.unwrap_or(false);
    let now = current_unix_timestamp()?;
    let observed_generation = {
        let cache = usage_state.cache.lock().await;
        match cache.as_ref() {
            Some(entry) if cache_matches_key(entry, &key_fingerprint) => {
                if should_return_fresh_cache(entry, &key_fingerprint, force_refresh, now) {
                    return Ok(usage_snapshot(entry, "cache", false, None));
                }
                Some(entry.generation)
            }
            _ => None,
        }
    };

    let _refresh_guard = usage_state.refresh_gate.lock().await;
    let now = current_unix_timestamp()?;
    {
        let cache = usage_state.cache.lock().await;
        if let Some(entry) = cache.as_ref() {
            let refreshed_while_waiting =
                refresh_completed_while_waiting(entry, &key_fingerprint, observed_generation);
            if should_return_fresh_cache(entry, &key_fingerprint, force_refresh, now)
                || refreshed_while_waiting
            {
                return Ok(usage_snapshot(entry, "cache", false, None));
            }
        }
    }

    match fetch_openai_usage_from_api(admin_key).await {
        Ok(usage) => {
            let fetched_at = current_unix_timestamp()?;
            let mut cache = usage_state.cache.lock().await;
            let generation = cache
                .as_ref()
                .map(|entry| entry.generation.saturating_add(1))
                .unwrap_or(1);
            let entry = CachedOpenAiUsage {
                key_fingerprint,
                usage,
                fetched_at,
                generation,
            };
            let snapshot = usage_snapshot(&entry, "network", false, None);
            *cache = Some(entry);
            Ok(snapshot)
        }
        Err(error) => {
            let cache = usage_state.cache.lock().await;
            if let Some(entry) = cache
                .as_ref()
                .filter(|entry| cache_matches_key(entry, &key_fingerprint))
            {
                Ok(usage_snapshot(entry, "cache", true, Some(error)))
            } else {
                Err(error)
            }
        }
    }
}

#[tauri::command]
fn get_runtime_diagnostics(
    app: tauri::AppHandle,
    app_server: tauri::State<'_, CodexAppServer>,
) -> RuntimeDiagnostics {
    let sidecar_available = app.shell().sidecar("codex").is_ok();
    let sidecar_running = app_server
        .server
        .lock()
        .map(|server| server.is_some())
        .unwrap_or(false);
    RuntimeDiagnostics {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        operating_system: std::env::consts::OS.to_string(),
        architecture: std::env::consts::ARCH.to_string(),
        test_mode: option_env!("TOKENGLASS_TEST_MODE") == Some("true"),
        sidecar_available,
        sidecar_running,
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(CodexAppServer::default())
        .manage(OpenAiUsageState::default())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let quit_i = MenuItem::with_id(app, "quit", "종료 (Quit)", true, None::<&str>)?;
            let toggle_widget_i = MenuItem::with_id(
                app,
                "toggle_widget",
                "위젯 켜기/끄기 (Toggle Widget)",
                true,
                None::<&str>,
            )?;
            let menu = Menu::with_items(app, &[&toggle_widget_i, &quit_i])?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(move |app, event| match event.id.as_ref() {
                    "quit" => {
                        app.exit(0);
                    }
                    "toggle_widget" => {
                        if let Some(widget) = app.get_webview_window("widget") {
                            let _ = widget.close();
                            if let Ok(store) = app.store("settings.json") {
                                store.set("tokenglass_show_widget", serde_json::json!(false));
                                let _ = store.save();
                            }
                        } else {
                            let _ = WebviewWindowBuilder::new(
                                app,
                                "widget",
                                WebviewUrl::App("/widget".into()),
                            )
                            .title("Widget")
                            .inner_size(200.0, 100.0)
                            // .transparent(true)
                            .decorations(false)
                            .always_on_top(true)
                            .skip_taskbar(true)
                            .build();
                            if let Ok(store) = app.store("settings.json") {
                                store.set("tokenglass_show_widget", serde_json::json!(true));
                                let _ = store.save();
                            }
                        }
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            if let Ok(Some(monitor)) = window.current_monitor() {
                                let physical_size = monitor.size();
                                let physical_position = monitor.position();

                                if let Ok(window_size) = window.outer_size() {
                                    let x = physical_position.x + physical_size.width as i32
                                        - window_size.width as i32
                                        - 20;
                                    let y = physical_position.y + physical_size.height as i32
                                        - window_size.height as i32
                                        - 60;
                                    let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
                                }
                            }

                            let is_visible = window.is_visible().unwrap_or(false);
                            if is_visible {
                                let _ = window.hide();
                            } else {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;

            // Load widget on startup if enabled
            if let Ok(store) = app.store("settings.json") {
                if let Some(val) = store.get("tokenglass_show_widget") {
                    if val.as_bool().unwrap_or(false) {
                        let _ = WebviewWindowBuilder::new(
                            app,
                            "widget",
                            WebviewUrl::App("/widget".into()),
                        )
                        .title("Widget")
                        .inner_size(200.0, 100.0)
                        // .transparent(true)
                        .decorations(false)
                        .always_on_top(true)
                        .skip_taskbar(true)
                        .build();
                    }
                }
            }

            Ok(())
        })
        .on_window_event(|window, event| match event {
            WindowEvent::Focused(false) => {
                if window.label() == "main" {
                    let _ = window.hide();
                }
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            fetch_openai_usage,
            start_chatgpt_login,
            fetch_chatgpt_subscription_usage,
            get_runtime_diagnostics
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod diagnostics_tests {
    use super::{
        cache_is_fresh, cache_matches_key, openai_key_fingerprint, refresh_completed_while_waiting,
        should_return_fresh_cache, usage_snapshot, CachedOpenAiUsage, ModelUsage, OpenAiUsage,
        OpenAiUsageState, OPENAI_USAGE_CACHE_TTL_SECONDS,
    };
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    fn cached_usage(key: &str, fetched_at: i64, generation: u64) -> CachedOpenAiUsage {
        CachedOpenAiUsage {
            key_fingerprint: openai_key_fingerprint(key),
            usage: OpenAiUsage {
                total_billed: 1.25,
                today_usage: 0.5,
                input_tokens: 10,
                output_tokens: 20,
                models: vec![ModelUsage {
                    name: "gpt-test".to_string(),
                    tokens: 30,
                }],
                period_start: 100,
                period_end: 200,
            },
            fetched_at,
            generation,
        }
    }

    #[test]
    fn diagnostics_redact_api_keys_and_bearer_tokens() {
        let value = "key sk-admin-secret-value Authorization: Bearer oauth-secret";
        let sanitized = super::sanitize_diagnostic_text(value);
        assert!(!sanitized.contains("secret-value"));
        assert!(!sanitized.contains("oauth-secret"));
        assert!(sanitized.contains("[redacted]"));
    }

    #[test]
    fn timezone_bounds_calculation_respects_offset() {
        use super::calculate_bounds_with_offset;
        // 2026-08-12 16:00:00 UTC (1786550400)
        // In KST (UTC+9), this is 2026-08-13 01:00:00 KST
        let now_kst = 1786550400_i64;
        let kst_offset = time::UtcOffset::from_hms(9, 0, 0).unwrap();
        let (period_start, today_start) = calculate_bounds_with_offset(now_kst, kst_offset);

        // KST today start (2026-08-13 00:00:00 KST) = 2026-08-12 15:00:00 UTC = 1786546800
        assert_eq!(today_start, 1786546800);
        // KST period start (2026-08-01 00:00:00 KST) = 2026-07-31 15:00:00 UTC = 1785510000
        assert_eq!(period_start, 1785510000);
    }

    #[test]
    fn cache_respects_ttl_force_refresh_and_key_isolation() {
        let key = "sk-admin-one";
        let entry = cached_usage(key, 1_000, 7);
        let fingerprint = openai_key_fingerprint(key);

        assert!(cache_is_fresh(
            &entry,
            1_000 + OPENAI_USAGE_CACHE_TTL_SECONDS - 1
        ));
        assert!(!cache_is_fresh(
            &entry,
            1_000 + OPENAI_USAGE_CACHE_TTL_SECONDS
        ));
        assert!(should_return_fresh_cache(
            &entry,
            &fingerprint,
            false,
            1_001
        ));
        assert!(!should_return_fresh_cache(
            &entry,
            &fingerprint,
            true,
            1_001
        ));
        assert!(!cache_matches_key(
            &entry,
            &openai_key_fingerprint("sk-admin-two")
        ));
    }

    #[test]
    fn stale_snapshot_keeps_last_successful_data() {
        let entry = cached_usage("sk-admin-one", 1_000, 7);
        let snapshot = usage_snapshot(
            &entry,
            "cache",
            true,
            Some("network unavailable".to_string()),
        );

        assert!(snapshot.stale);
        assert_eq!(snapshot.fetched_at, 1_000);
        assert_eq!(snapshot.usage.total_billed, 1.25);
        assert_eq!(
            snapshot.refresh_error.as_deref(),
            Some("network unavailable")
        );
    }

    #[tokio::test]
    async fn concurrent_requests_reuse_one_completed_refresh_generation() {
        let state = Arc::new(OpenAiUsageState::default());
        let key = "sk-admin-one";
        let fingerprint = openai_key_fingerprint(key);
        {
            let mut cache = state.cache.lock().await;
            *cache = Some(cached_usage(key, 1_000, 1));
        }

        let refresh_count = Arc::new(AtomicUsize::new(0));
        let refresh_gate = state.refresh_gate.lock().await;
        refresh_count.fetch_add(1, Ordering::SeqCst);
        {
            let mut cache = state.cache.lock().await;
            *cache = Some(cached_usage(key, 1_500, 2));
        }

        let waiting_state = Arc::clone(&state);
        let waiting_fingerprint = fingerprint.clone();
        let waiting_request = tokio::spawn(async move {
            let _gate = waiting_state.refresh_gate.lock().await;
            let cache = waiting_state.cache.lock().await;
            let entry = cache.as_ref().expect("refresh should populate the cache");
            refresh_completed_while_waiting(entry, &waiting_fingerprint, Some(1))
        });

        drop(refresh_gate);
        assert!(waiting_request
            .await
            .expect("waiting request should complete"));
        assert_eq!(refresh_count.load(Ordering::SeqCst), 1);
    }
}
