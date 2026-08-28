use crate::codex::CodexAppServer;
use serde::Serialize;
use serde_json::Value;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatGptLogin {
    login_id: String,
    auth_url: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionLimit {
    limit_id: String,
    label: Option<String>,
    used_percent: Option<f64>,
    window_duration_mins: Option<u64>,
    resets_at: Option<i64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyTokenUsage {
    start_date: String,
    tokens: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatGptSubscriptionUsage {
    email: Option<String>,
    plan_type: Option<String>,
    limits: Vec<SubscriptionLimit>,
    lifetime_tokens: Option<u64>,
    peak_daily_tokens: Option<u64>,
    daily_usage: Vec<DailyTokenUsage>,
}

#[tauri::command]
pub fn start_chatgpt_login(
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
pub fn fetch_chatgpt_subscription_usage(
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
