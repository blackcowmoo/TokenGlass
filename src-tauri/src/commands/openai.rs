use crate::openai::{
    fetch_usage, CredentialError, OpenAiCredentials, OpenAiUsageSnapshot, OpenAiUsageState,
};
use serde::Serialize;
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;
use tokio::sync::Mutex;

const LEGACY_KEY: &str = "tokenglass_openai_admin_key";
const SETTINGS_STORE: &str = "settings.json";

pub struct OpenAiKeyState {
    credentials: OpenAiCredentials,
    migration_gate: Mutex<()>,
}

impl Default for OpenAiKeyState {
    fn default() -> Self {
        Self {
            credentials: OpenAiCredentials::default(),
            migration_gate: Mutex::new(()),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenAiKeyStatus {
    configured: bool,
    error_code: Option<&'static str>,
}

fn credential_error(error: CredentialError) -> String {
    error.message().to_string()
}

async fn ensure_migrated(app: &AppHandle, key_state: &OpenAiKeyState) -> Result<(), String> {
    let _gate = key_state.migration_gate.lock().await;
    let store = app
        .store(SETTINGS_STORE)
        .map_err(|_| "일반 설정 저장소를 열 수 없습니다.".to_string())?;
    let legacy = store
        .get(LEGACY_KEY)
        .and_then(|value| value.as_str().map(str::to_owned))
        .filter(|value| !value.trim().is_empty());

    if key_state
        .credentials
        .get()
        .await
        .map_err(credential_error)?
        .is_none()
    {
        if let Some(value) = legacy.as_ref() {
            key_state
                .credentials
                .set(value.clone())
                .await
                .map_err(|_| "기존 키를 보안 저장소로 이전하지 못했습니다.".to_string())?;
            let verified = key_state
                .credentials
                .get()
                .await
                .map_err(|_| "기존 키를 보안 저장소에서 확인하지 못했습니다.".to_string())?;
            if verified.as_deref() != Some(value) {
                return Err("기존 키를 보안 저장소에서 확인하지 못했습니다.".to_string());
            }
        }
    }

    if legacy.is_some() {
        store.delete(LEGACY_KEY);
        let _ = store.save();
    }
    Ok(())
}

#[tauri::command]
pub async fn get_openai_key_status(
    app: AppHandle,
    key_state: tauri::State<'_, OpenAiKeyState>,
) -> Result<OpenAiKeyStatus, String> {
    ensure_migrated(&app, &key_state).await?;
    match key_state.credentials.get().await {
        Ok(value) => Ok(OpenAiKeyStatus {
            configured: value.is_some(),
            error_code: None,
        }),
        Err(error) => Ok(OpenAiKeyStatus {
            configured: false,
            error_code: Some(error.code()),
        }),
    }
}

#[tauri::command]
pub async fn save_openai_admin_key(
    admin_key: String,
    key_state: tauri::State<'_, OpenAiKeyState>,
    usage_state: tauri::State<'_, OpenAiUsageState>,
) -> Result<(), String> {
    let key = admin_key.trim();
    if key.is_empty() {
        return Err("OpenAI 조직 관리자 API 키를 입력하세요.".to_string());
    }
    key_state
        .credentials
        .set(key.to_string())
        .await
        .map_err(credential_error)?;
    usage_state.clear_cache().await;
    Ok(())
}

#[tauri::command]
pub async fn delete_openai_admin_key(
    key_state: tauri::State<'_, OpenAiKeyState>,
    usage_state: tauri::State<'_, OpenAiUsageState>,
) -> Result<(), String> {
    key_state
        .credentials
        .delete()
        .await
        .map_err(credential_error)?;
    usage_state.clear_cache().await;
    Ok(())
}

#[tauri::command]
pub async fn fetch_openai_usage(
    app: AppHandle,
    force_refresh: Option<bool>,
    key_state: tauri::State<'_, OpenAiKeyState>,
    usage_state: tauri::State<'_, OpenAiUsageState>,
) -> Result<OpenAiUsageSnapshot, String> {
    ensure_migrated(&app, &key_state).await?;
    let admin_key = key_state
        .credentials
        .get()
        .await
        .map_err(credential_error)?
        .ok_or_else(|| "OpenAI 조직 관리자 API 키를 연결하세요.".to_string())?;
    fetch_usage(&admin_key, force_refresh.unwrap_or(false), &usage_state).await
}
