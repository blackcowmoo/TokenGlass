use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_store::StoreExt;

const SETTINGS_STORE_PATH: &str = "settings.json";
const SHOW_WIDGET_SETTING_KEY: &str = "tokenglass_show_widget";
pub(crate) const WIDGET_WINDOW_LABEL: &str = "widget";

fn save_widget_visibility(app: &AppHandle, is_visible: bool) {
    if let Ok(store) = app.store(SETTINGS_STORE_PATH) {
        store.set(SHOW_WIDGET_SETTING_KEY, serde_json::json!(is_visible));
        let _ = store.save();
    }
}

fn create_widget_window(app: &AppHandle) {
    let _ = WebviewWindowBuilder::new(app, WIDGET_WINDOW_LABEL, WebviewUrl::App("/widget".into()))
        .title("Widget")
        .inner_size(200.0, 100.0)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .build();
}

pub(crate) fn toggle_widget(app: &AppHandle) {
    if let Some(widget) = app.get_webview_window(WIDGET_WINDOW_LABEL) {
        let _ = widget.close();
        save_widget_visibility(app, false);
    } else {
        create_widget_window(app);
        save_widget_visibility(app, true);
    }
}

pub(crate) fn restore_widget_if_enabled(app: &AppHandle) {
    let is_enabled = app
        .store(SETTINGS_STORE_PATH)
        .ok()
        .and_then(|store| store.get(SHOW_WIDGET_SETTING_KEY))
        .and_then(|value| value.as_bool())
        .unwrap_or(false);

    if is_enabled {
        create_widget_window(app);
    }
}
