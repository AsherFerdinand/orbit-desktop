use tauri::{
    menu::{ContextMenu, Menu, MenuEvent},
    Emitter, Manager,
};
pub struct NativeMenu(pub Menu<tauri::Wry>);
pub fn handle(app: &tauri::AppHandle, event: MenuEvent) {
    if let Some(w) = app.get_webview_window("main") {
        match event.id.as_ref() {
            "show" => {
                let _ = w.show();
                let _ = w.emit("orbit-visible", true);
            }
            "settings" => {
                let _ = w.show();
                let _ = w.set_focus();
                let _ = w.emit("open-settings", ());
            }
            "profile" => {
                let _ = w.emit("measure-current", ());
            }
            "hide" => {
                let _ = crate::window::hide_island(app.clone());
            }
            "quit" => app.exit(0),
            _ => {}
        }
    }
}
#[tauri::command]
pub fn show_system_menu(app: tauri::AppHandle) -> Result<(), String> {
    let w = app.get_webview_window("main").ok_or("Window unavailable")?;
    app.state::<NativeMenu>()
        .0
        .popup(w.as_ref().window())
        .map_err(|e| e.to_string())
}
