use keyring::{Entry, Error as KeyringError};
use std::sync::Arc;

pub const OPENAI_KEYRING_SERVICE: &str = "com.mykim.tokenglass";
pub const OPENAI_KEYRING_ACCOUNT: &str = "openai-admin-key";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CredentialError {
    Unavailable,
    LockedOrDenied,
    OperationFailed,
}

impl CredentialError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Unavailable => "unavailable",
            Self::LockedOrDenied => "lockedOrDenied",
            Self::OperationFailed => "operationFailed",
        }
    }

    pub const fn message(self) -> &'static str {
        match self {
            Self::Unavailable => "OS 보안 저장소를 사용할 수 없습니다.",
            Self::LockedOrDenied => "OS 보안 저장소가 잠겼거나 접근이 거부되었습니다.",
            Self::OperationFailed => "OS 보안 저장소 작업에 실패했습니다.",
        }
    }
}

pub trait CredentialBackend: Send + Sync {
    fn get(&self) -> Result<Option<String>, CredentialError>;
    fn set(&self, value: &str) -> Result<(), CredentialError>;
    fn delete(&self) -> Result<(), CredentialError>;
}

#[derive(Clone)]
pub struct OpenAiCredentials {
    backend: Arc<dyn CredentialBackend>,
}

impl Default for OpenAiCredentials {
    fn default() -> Self {
        Self::new(Arc::new(NativeCredentialBackend))
    }
}

impl OpenAiCredentials {
    pub fn new(backend: Arc<dyn CredentialBackend>) -> Self {
        Self { backend }
    }

    pub async fn get(&self) -> Result<Option<String>, CredentialError> {
        let backend = Arc::clone(&self.backend);
        tokio::task::spawn_blocking(move || backend.get())
            .await
            .map_err(|_| CredentialError::OperationFailed)?
    }

    pub async fn set(&self, value: String) -> Result<(), CredentialError> {
        let backend = Arc::clone(&self.backend);
        tokio::task::spawn_blocking(move || backend.set(&value))
            .await
            .map_err(|_| CredentialError::OperationFailed)?
    }

    pub async fn delete(&self) -> Result<(), CredentialError> {
        let backend = Arc::clone(&self.backend);
        tokio::task::spawn_blocking(move || backend.delete())
            .await
            .map_err(|_| CredentialError::OperationFailed)?
    }
}

struct NativeCredentialBackend;

impl NativeCredentialBackend {
    fn entry() -> Result<Entry, CredentialError> {
        Entry::new(OPENAI_KEYRING_SERVICE, OPENAI_KEYRING_ACCOUNT).map_err(map_keyring_error)
    }
}

impl CredentialBackend for NativeCredentialBackend {
    fn get(&self) -> Result<Option<String>, CredentialError> {
        match Self::entry()?.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(KeyringError::NoEntry) => Ok(None),
            Err(error) => Err(map_keyring_error(error)),
        }
    }

    fn set(&self, value: &str) -> Result<(), CredentialError> {
        Self::entry()?
            .set_password(value)
            .map_err(map_keyring_error)
    }

    fn delete(&self) -> Result<(), CredentialError> {
        match Self::entry()?.delete_credential() {
            Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
            Err(error) => Err(map_keyring_error(error)),
        }
    }
}

fn map_keyring_error(error: KeyringError) -> CredentialError {
    match error {
        KeyringError::NoStorageAccess(_) => CredentialError::LockedOrDenied,
        KeyringError::PlatformFailure(_) => CredentialError::Unavailable,
        KeyringError::NoEntry
        | KeyringError::BadEncoding(_)
        | KeyringError::TooLong(_, _)
        | KeyringError::Invalid(_, _)
        | KeyringError::Ambiguous(_) => CredentialError::OperationFailed,
        _ => CredentialError::OperationFailed,
    }
}

#[cfg(test)]
mod tests {
    use super::{map_keyring_error, CredentialBackend, CredentialError, OpenAiCredentials};
    use keyring::Error as KeyringError;
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct MemoryCredentialBackend {
        value: Mutex<Option<String>>,
    }

    impl CredentialBackend for MemoryCredentialBackend {
        fn get(&self) -> Result<Option<String>, CredentialError> {
            Ok(self.value.lock().expect("test mutex").clone())
        }

        fn set(&self, value: &str) -> Result<(), CredentialError> {
            *self.value.lock().expect("test mutex") = Some(value.to_string());
            Ok(())
        }

        fn delete(&self) -> Result<(), CredentialError> {
            *self.value.lock().expect("test mutex") = None;
            Ok(())
        }
    }

    #[tokio::test]
    async fn memory_backend_saves_updates_reads_and_deletes_idempotently() {
        let credentials = OpenAiCredentials::new(Arc::new(MemoryCredentialBackend::default()));

        assert_eq!(credentials.get().await, Ok(None));
        assert_eq!(credentials.set("first".to_string()).await, Ok(()));
        assert_eq!(credentials.get().await, Ok(Some("first".to_string())));
        assert_eq!(credentials.set("second".to_string()).await, Ok(()));
        assert_eq!(credentials.get().await, Ok(Some("second".to_string())));
        assert_eq!(credentials.delete().await, Ok(()));
        assert_eq!(credentials.delete().await, Ok(()));
        assert_eq!(credentials.get().await, Ok(None));
    }

    #[test]
    fn keyring_errors_map_to_stable_secret_free_categories() {
        let platform_error = || {
            Box::new(std::io::Error::other("private platform detail"))
                as Box<dyn std::error::Error + Send + Sync>
        };
        assert_eq!(
            map_keyring_error(KeyringError::NoStorageAccess(platform_error())),
            CredentialError::LockedOrDenied
        );
        assert_eq!(
            map_keyring_error(KeyringError::PlatformFailure(platform_error())),
            CredentialError::Unavailable
        );
        assert_eq!(
            map_keyring_error(KeyringError::BadEncoding(b"secret".to_vec())),
            CredentialError::OperationFailed
        );
        assert_eq!(CredentialError::LockedOrDenied.code(), "lockedOrDenied");
        assert_eq!(CredentialError::Unavailable.code(), "unavailable");
        assert_eq!(CredentialError::OperationFailed.code(), "operationFailed");
    }
}
