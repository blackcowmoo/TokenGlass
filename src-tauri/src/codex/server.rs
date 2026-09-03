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

struct RunningAppServer {
    child: CommandChild,
    next_id: u64,
    waiters: Arc<Mutex<HashMap<u64, mpsc::Sender<Value>>>>,
}

#[derive(Default)]
pub struct CodexAppServer {
    server: Mutex<Option<RunningAppServer>>,
}

impl CodexAppServer {
    pub fn start_if_needed(&self, app: &tauri::AppHandle) -> Result<(), String> {
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
            child,
            next_id: 1,
            waiters,
        });
        drop(server);
        self.request("initialize", serde_json::json!({
            "clientInfo": { "name": "tokenglass", "title": "TokenGlass", "version": env!("CARGO_PKG_VERSION") }
        }))?;
        self.notify("initialized", serde_json::json!({}))
    }

    pub fn is_running(&self) -> bool {
        self.server
            .lock()
            .map(|server| server.is_some())
            .unwrap_or(false)
    }

    pub fn notify(&self, method: &str, params: Value) -> Result<(), String> {
        let mut server = self
            .server
            .lock()
            .map_err(|_| "Codex 연결 상태를 잠글 수 없습니다.".to_string())?;
        let running = server
            .as_mut()
            .ok_or("Codex App Server가 시작되지 않았습니다.")?;
        let message = serde_json::json!({ "method": method, "params": params });
        running
            .child
            .write(format!("{message}\n").as_bytes())
            .map_err(|error| format!("Codex에 요청을 보낼 수 없습니다: {error}"))
    }

    pub fn request(&self, method: &str, params: Value) -> Result<Value, String> {
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
            if let Err(error) = running.child.write(format!("{message}\n").as_bytes()) {
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
