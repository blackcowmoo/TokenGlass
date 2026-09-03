mod codex;
mod commands;
mod openai;
mod window;

use codex::CodexAppServer;
use commands::{
    chatgpt::{fetch_chatgpt_subscription_usage, start_chatgpt_login},
    diagnostics::get_runtime_diagnostics,
    openai::fetch_openai_usage,
};
use openai::OpenAiUsageState;
use window::{handle_window_event, setup_app_shell};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(CodexAppServer::default())
        .manage(OpenAiUsageState::default())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .setup(setup_app_shell)
        .on_window_event(handle_window_event)
        .invoke_handler(tauri::generate_handler![
            fetch_openai_usage,
            start_chatgpt_login,
            fetch_chatgpt_subscription_usage,
            get_runtime_diagnostics
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
