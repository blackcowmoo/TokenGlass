use crate::codex::CodexAppServer;
use serde::Serialize;
use tauri_plugin_shell::ShellExt;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeDiagnostics {
    app_version: String,
    operating_system: String,
    architecture: String,
    test_mode: bool,
    sidecar_available: bool,
    sidecar_running: bool,
}

#[tauri::command]
pub fn get_runtime_diagnostics(
    app: tauri::AppHandle,
    app_server: tauri::State<'_, CodexAppServer>,
) -> RuntimeDiagnostics {
    RuntimeDiagnostics {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        operating_system: std::env::consts::OS.to_string(),
        architecture: std::env::consts::ARCH.to_string(),
        test_mode: option_env!("TOKENGLASS_TEST_MODE") == Some("true"),
        sidecar_available: app.shell().sidecar("codex").is_ok(),
        sidecar_running: app_server.is_running(),
    }
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

#[cfg(test)]
mod tests {
    use super::sanitize_diagnostic_text;

    #[test]
    fn diagnostics_redact_api_keys_and_bearer_tokens() {
        let value = "key sk-admin-secret-value Authorization: Bearer oauth-secret";
        let sanitized = sanitize_diagnostic_text(value);
        assert!(!sanitized.contains("secret-value"));
        assert!(!sanitized.contains("oauth-secret"));
        assert!(sanitized.contains("[redacted]"));
    }
}
