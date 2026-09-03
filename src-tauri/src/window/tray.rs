use super::widget::{restore_widget_if_enabled, toggle_widget};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, WebviewWindow, Window, WindowEvent,
};

const MAIN_WINDOW_LABEL: &str = "main";

fn position_main_window(window: &WebviewWindow) {
    if let Ok(Some(monitor)) = window.current_monitor() {
        let monitor_size = monitor.size();
        let monitor_position = monitor.position();

        if let Ok(window_size) = window.outer_size() {
            let x = monitor_position.x + monitor_size.width as i32 - window_size.width as i32 - 20;
            let y =
                monitor_position.y + monitor_size.height as i32 - window_size.height as i32 - 60;
            let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
        }
    }
}

fn toggle_main_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) else {
        return;
    };

    position_main_window(&window);
    if window.is_visible().unwrap_or(false) {
        let _ = window.hide();
    } else {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn handle_menu_event(app: &AppHandle, menu_id: &str) {
    match menu_id {
        "quit" => app.exit(0),
        "toggle_widget" => toggle_widget(app),
        _ => {}
    }
}

fn configure_tray(app: &AppHandle) -> tauri::Result<()> {
    let quit_item = MenuItem::with_id(app, "quit", "종료 (Quit)", true, None::<&str>)?;
    let toggle_widget_item = MenuItem::with_id(
        app,
        "toggle_widget",
        "위젯 켜기/끄기 (Toggle Widget)",
        true,
        None::<&str>,
    )?;
    let menu = Menu::with_items(app, &[&toggle_widget_item, &quit_item])?;

    let _tray = TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .on_menu_event(|app, event| handle_menu_event(app, event.id.as_ref()))
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

pub fn setup_app_shell(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let app_handle = app.handle();
    configure_tray(app_handle)?;
    restore_widget_if_enabled(app_handle);
    Ok(())
}

pub fn handle_window_event(window: &Window, event: &WindowEvent) {
    if matches!(event, WindowEvent::Focused(false)) && window.label() == MAIN_WINDOW_LABEL {
        let _ = window.hide();
    }
}
