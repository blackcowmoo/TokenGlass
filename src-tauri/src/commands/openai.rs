use crate::openai::{fetch_usage, OpenAiUsageSnapshot, OpenAiUsageState};

#[tauri::command]
pub async fn fetch_openai_usage(
    admin_key: String,
    force_refresh: Option<bool>,
    usage_state: tauri::State<'_, OpenAiUsageState>,
) -> Result<OpenAiUsageSnapshot, String> {
    fetch_usage(
        admin_key.trim(),
        force_refresh.unwrap_or(false),
        &usage_state,
    )
    .await
}
