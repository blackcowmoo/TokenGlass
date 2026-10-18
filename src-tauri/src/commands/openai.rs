use crate::openai::{
    fetch_usage, CredentialError, OpenAiCredentials, OpenAiUsageSnapshot, OpenAiUsageState,
};
use serde::Serialize;
use tauri::AppHandle;
use tauri_plugin_store::{Store, StoreExt};
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

trait LegacyKeyStore: Send + Sync {
    fn legacy_key(&self) -> Option<String>;
    fn remove_legacy_key(&self) -> Result<(), ()>;
}

impl<R: tauri::Runtime> LegacyKeyStore for Store<R> {
    fn legacy_key(&self) -> Option<String> {
        self.get(LEGACY_KEY)
            .and_then(|value| value.as_str().map(str::to_owned))
            .filter(|value| !value.trim().is_empty())
    }

    fn remove_legacy_key(&self) -> Result<(), ()> {
        let legacy = self.legacy_key().ok_or(())?;
        self.delete(LEGACY_KEY);
        if self.save().is_err() {
            self.set(LEGACY_KEY, legacy);
            return Err(());
        }
        Ok(())
    }
}

async fn migrate_legacy_key(
    credentials: &OpenAiCredentials,
    migration_gate: &Mutex<()>,
    legacy_store: &dyn LegacyKeyStore,
) -> Result<(), String> {
    let _gate = migration_gate.lock().await;
    let legacy = legacy_store.legacy_key();
    let stored = credentials.get().await.map_err(credential_error)?;

    if stored.is_none() {
        if let Some(value) = legacy.as_ref() {
            credentials
                .set(value.clone())
                .await
                .map_err(|_| "기존 키를 보안 저장소로 이전하지 못했습니다.".to_string())?;
            let verified = credentials
                .get()
                .await
                .map_err(|_| "기존 키를 보안 저장소에서 확인하지 못했습니다.".to_string())?;
            if verified.as_deref() != Some(value) {
                return Err("기존 키를 보안 저장소에서 확인하지 못했습니다.".to_string());
            }
        }
    }

    if legacy.is_some() {
        legacy_store
            .remove_legacy_key()
            .map_err(|_| "기존 키를 일반 설정 저장소에서 정리하지 못했습니다.".to_string())?;
    }
    Ok(())
}

async fn ensure_migrated(app: &AppHandle, key_state: &OpenAiKeyState) -> Result<(), String> {
    let store = app
        .store(SETTINGS_STORE)
        .map_err(|_| "일반 설정 저장소를 열 수 없습니다.".to_string())?;
    migrate_legacy_key(
        &key_state.credentials,
        &key_state.migration_gate,
        store.as_ref(),
    )
    .await
}

async fn save_key(
    credentials: &OpenAiCredentials,
    usage_state: &OpenAiUsageState,
    key: String,
) -> Result<(), String> {
    credentials.set(key).await.map_err(credential_error)?;
    usage_state.clear_cache().await;
    Ok(())
}

async fn delete_key(
    credentials: &OpenAiCredentials,
    usage_state: &OpenAiUsageState,
) -> Result<(), String> {
    credentials.delete().await.map_err(credential_error)?;
    usage_state.clear_cache().await;
    Ok(())
}

async fn fetch_usage_for_key(
    credentials: &OpenAiCredentials,
    force_refresh: bool,
    usage_state: &OpenAiUsageState,
) -> Result<OpenAiUsageSnapshot, String> {
    let admin_key = credentials
        .get()
        .await
        .map_err(credential_error)?
        .ok_or_else(|| "OpenAI 조직 관리자 API 키를 연결하세요.".to_string())?;
    fetch_usage(&admin_key, force_refresh, usage_state).await
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
    save_key(&key_state.credentials, &usage_state, key.to_string()).await
}

#[tauri::command]
pub async fn delete_openai_admin_key(
    key_state: tauri::State<'_, OpenAiKeyState>,
    usage_state: tauri::State<'_, OpenAiUsageState>,
) -> Result<(), String> {
    delete_key(&key_state.credentials, &usage_state).await
}

#[tauri::command]
pub async fn fetch_openai_usage(
    app: AppHandle,
    force_refresh: Option<bool>,
    key_state: tauri::State<'_, OpenAiKeyState>,
    usage_state: tauri::State<'_, OpenAiUsageState>,
) -> Result<OpenAiUsageSnapshot, String> {
    ensure_migrated(&app, &key_state).await?;
    fetch_usage_for_key(
        &key_state.credentials,
        force_refresh.unwrap_or(false),
        &usage_state,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::{
        delete_key, fetch_usage_for_key, migrate_legacy_key, save_key, LegacyKeyStore,
        OpenAiKeyStatus,
    };
    use crate::openai::{CredentialBackend, CredentialError, OpenAiCredentials, OpenAiUsageState};
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    };
    use tokio::sync::Mutex as AsyncMutex;

    #[derive(Default)]
    struct TestCredentialBackend {
        value: Mutex<Option<String>>,
        fail_set: bool,
        mismatch_readback: bool,
        reads: AtomicUsize,
        writes: AtomicUsize,
    }

    impl CredentialBackend for TestCredentialBackend {
        fn get(&self) -> Result<Option<String>, CredentialError> {
            let read = self.reads.fetch_add(1, Ordering::SeqCst);
            if self.mismatch_readback && read > 0 {
                return Ok(Some("different-value".to_string()));
            }
            Ok(self.value.lock().expect("credential mutex").clone())
        }

        fn set(&self, value: &str) -> Result<(), CredentialError> {
            self.writes.fetch_add(1, Ordering::SeqCst);
            if self.fail_set {
                return Err(CredentialError::Unavailable);
            }
            *self.value.lock().expect("credential mutex") = Some(value.to_string());
            Ok(())
        }

        fn delete(&self) -> Result<(), CredentialError> {
            *self.value.lock().expect("credential mutex") = None;
            Ok(())
        }
    }

    #[derive(Default)]
    struct TestLegacyStore {
        value: Mutex<Option<String>>,
        fail_remove: std::sync::atomic::AtomicBool,
        removals: AtomicUsize,
    }

    impl LegacyKeyStore for TestLegacyStore {
        fn legacy_key(&self) -> Option<String> {
            self.value.lock().expect("legacy mutex").clone()
        }

        fn remove_legacy_key(&self) -> Result<(), ()> {
            self.removals.fetch_add(1, Ordering::SeqCst);
            if self.fail_remove.load(Ordering::SeqCst) {
                return Err(());
            }
            *self.value.lock().expect("legacy mutex") = None;
            Ok(())
        }
    }

    fn credentials(backend: Arc<TestCredentialBackend>) -> OpenAiCredentials {
        OpenAiCredentials::new(backend)
    }

    fn legacy(value: Option<&str>) -> TestLegacyStore {
        TestLegacyStore {
            value: Mutex::new(value.map(str::to_owned)),
            ..TestLegacyStore::default()
        }
    }

    #[test]
    fn key_status_serialization_contains_only_configuration_and_safe_error_code() {
        let status = OpenAiKeyStatus {
            configured: true,
            error_code: Some("lockedOrDenied"),
        };
        let serialized = serde_json::to_string(&status).expect("status should serialize");

        assert_eq!(
            serialized,
            r#"{"configured":true,"errorCode":"lockedOrDenied"}"#
        );
        assert!(!serialized.contains("sk-admin-canary-secret"));
    }

    #[tokio::test]
    async fn migration_verifies_secure_write_before_removing_legacy_value() {
        let backend = Arc::new(TestCredentialBackend::default());
        let credentials = credentials(backend.clone());
        let store = legacy(Some("legacy-secret"));

        migrate_legacy_key(&credentials, &AsyncMutex::new(()), &store)
            .await
            .expect("migration should succeed");

        assert_eq!(
            backend.value.lock().unwrap().as_deref(),
            Some("legacy-secret")
        );
        assert_eq!(*store.value.lock().unwrap(), None);
        assert_eq!(store.removals.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn failed_secure_write_preserves_legacy_value() {
        let backend = Arc::new(TestCredentialBackend {
            fail_set: true,
            ..TestCredentialBackend::default()
        });
        let store = legacy(Some("legacy-secret"));

        let result = migrate_legacy_key(&credentials(backend), &AsyncMutex::new(()), &store).await;

        assert!(result.is_err());
        assert_eq!(
            store.value.lock().unwrap().as_deref(),
            Some("legacy-secret")
        );
        assert_eq!(store.removals.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn readback_mismatch_preserves_legacy_value() {
        let backend = Arc::new(TestCredentialBackend {
            mismatch_readback: true,
            ..TestCredentialBackend::default()
        });
        let store = legacy(Some("legacy-secret"));

        let result = migrate_legacy_key(&credentials(backend), &AsyncMutex::new(()), &store).await;

        assert!(result.is_err());
        assert_eq!(
            store.value.lock().unwrap().as_deref(),
            Some("legacy-secret")
        );
        assert_eq!(store.removals.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn secure_value_wins_and_legacy_cleanup_failure_can_be_retried() {
        let backend = Arc::new(TestCredentialBackend::default());
        *backend.value.lock().unwrap() = Some("secure-value".to_string());
        let credentials = credentials(backend.clone());
        let store = TestLegacyStore {
            value: Mutex::new(Some("legacy-value".to_string())),
            fail_remove: std::sync::atomic::AtomicBool::new(true),
            ..TestLegacyStore::default()
        };
        let gate = AsyncMutex::new(());

        assert!(migrate_legacy_key(&credentials, &gate, &store)
            .await
            .is_err());
        assert_eq!(
            backend.value.lock().unwrap().as_deref(),
            Some("secure-value")
        );
        assert_eq!(store.value.lock().unwrap().as_deref(), Some("legacy-value"));
        store.fail_remove.store(false, Ordering::SeqCst);
        migrate_legacy_key(&credentials, &gate, &store)
            .await
            .expect("cleanup retry should succeed");
        assert_eq!(backend.writes.load(Ordering::SeqCst), 0);
        assert_eq!(*store.value.lock().unwrap(), None);
    }

    #[tokio::test]
    async fn concurrent_migration_calls_are_serialized_and_idempotent() {
        let backend = Arc::new(TestCredentialBackend::default());
        let credentials = credentials(backend.clone());
        let store = Arc::new(legacy(Some("legacy-secret")));
        let gate = Arc::new(AsyncMutex::new(()));
        let mut workers = Vec::new();
        for _ in 0..8 {
            let credentials = credentials.clone();
            let store = store.clone();
            let gate = gate.clone();
            workers.push(tokio::spawn(async move {
                migrate_legacy_key(&credentials, &gate, store.as_ref()).await
            }));
        }
        for worker in workers {
            worker.await.unwrap().expect("migration should succeed");
        }

        assert_eq!(backend.writes.load(Ordering::SeqCst), 1);
        assert_eq!(store.removals.load(Ordering::SeqCst), 1);
        assert_eq!(*store.value.lock().unwrap(), None);
    }

    #[tokio::test]
    async fn repeated_migration_after_restart_fixture_is_idempotent() {
        let backend = Arc::new(TestCredentialBackend::default());
        let credentials = credentials(backend.clone());
        let store = legacy(Some("legacy-secret"));

        migrate_legacy_key(&credentials, &AsyncMutex::new(()), &store)
            .await
            .expect("first startup migration should succeed");
        migrate_legacy_key(&credentials, &AsyncMutex::new(()), &store)
            .await
            .expect("subsequent startup migration should succeed");

        assert_eq!(backend.writes.load(Ordering::SeqCst), 1);
        assert_eq!(store.removals.load(Ordering::SeqCst), 1);
        assert_eq!(
            backend.value.lock().unwrap().as_deref(),
            Some("legacy-secret")
        );
        assert_eq!(*store.value.lock().unwrap(), None);
    }

    #[tokio::test]
    async fn successful_key_save_and_delete_clear_usage_cache() {
        let backend = Arc::new(TestCredentialBackend::default());
        let credentials = credentials(backend.clone());
        let usage_state = OpenAiUsageState::default();
        usage_state.seed_test_cache("previous-key").await;

        save_key(&credentials, &usage_state, "new-key".to_string())
            .await
            .expect("save should succeed");
        assert!(!usage_state.has_test_cache().await);
        assert_eq!(backend.value.lock().unwrap().as_deref(), Some("new-key"));

        usage_state.seed_test_cache("new-key").await;
        delete_key(&credentials, &usage_state)
            .await
            .expect("delete should succeed");
        assert!(!usage_state.has_test_cache().await);
        assert_eq!(*backend.value.lock().unwrap(), None);
    }

    #[tokio::test]
    async fn failed_key_save_keeps_previous_credential_and_cache() {
        let backend = Arc::new(TestCredentialBackend {
            fail_set: true,
            ..TestCredentialBackend::default()
        });
        *backend.value.lock().unwrap() = Some("previous-key".to_string());
        let credentials = credentials(backend.clone());
        let usage_state = OpenAiUsageState::default();
        usage_state.seed_test_cache("previous-key").await;

        let result = save_key(&credentials, &usage_state, "replacement-key".to_string()).await;

        assert!(result.is_err());
        assert_eq!(
            backend.value.lock().unwrap().as_deref(),
            Some("previous-key")
        );
        assert!(usage_state.has_test_cache().await);
    }

    #[tokio::test]
    async fn usage_fetch_without_a_saved_key_returns_a_safe_setup_error() {
        let credentials = credentials(Arc::new(TestCredentialBackend::default()));
        let usage_state = OpenAiUsageState::default();

        let error = match fetch_usage_for_key(&credentials, false, &usage_state).await {
            Ok(_) => panic!("missing key should fail before making a request"),
            Err(error) => error,
        };

        assert_eq!(error, "OpenAI 조직 관리자 API 키를 연결하세요.");
    }
}
