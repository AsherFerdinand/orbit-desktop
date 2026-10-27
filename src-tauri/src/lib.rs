mod audio;
mod battery;
mod events;
mod media;
mod native_menu;
mod notes;
mod performance;
mod power;
mod reminders;
mod runtime;
mod settings;
mod system;
mod theme;
mod window;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager,
};
pub fn run() {
    tauri::Builder::default()
        .manage(settings::SettingsLock::default())
        .manage(notes::NotesLock::default())
        .invoke_handler(tauri::generate_handler![
            battery::get_battery,
            native_menu::show_system_menu,
            notes::list_notes,
            notes::mutate_note,
            media::get_media,
            media::get_media_artwork,
            media::media_action,
            media::media_seek,
            settings::load_settings,
            settings::save_settings,
            theme::get_windows_theme,
            theme::set_windows_theme,
            window::resize_island,
            window::set_island_region,
            window::hide_island,
            runtime::get_snapshot,
            runtime::set_monitoring,
            runtime::set_volume,
            performance::measure_performance
        ])
        .setup(|app| {
            let initial = settings::load_settings(app.handle().clone()).unwrap_or_default();
            app.manage(runtime::Runtime::start(app.handle().clone(), initial));
            app.manage(media::Media::start(app.handle().clone()));
            let show = MenuItem::with_id(app, "show", "Show ORBIT", true, None::<&str>)?;
            let settings_item =
                MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
            let profile = MenuItem::with_id(
                app,
                "profile",
                "Measure current footprint (20s)",
                true,
                None::<&str>,
            )?;
            let hide = MenuItem::with_id(app, "hide", "Hide ORBIT", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Exit ORBIT", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &settings_item, &profile, &hide, &quit])?;
            app.manage(native_menu::NativeMenu(menu.clone()));
            app.handle().on_menu_event(native_menu::handle);
            let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/32x32.png"))?;
            TrayIconBuilder::new()
                .icon(icon)
                .tooltip("ORBIT · Your desktop, in motion.")
                .menu(&menu)
                .show_menu_on_left_click(true)
                .build(app)?;
            window::resize_island(app.handle().clone(), false, 420, 56, 348, 0)
                .map_err(std::io::Error::other)?;
            if let Some(w) = app.get_webview_window("main") {
                w.show()?;
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("ORBIT could not start")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                let _ = app
                    .state::<media::Media>()
                    .tx
                    .try_send(media::Message::Stop);
                let _ = app
                    .state::<runtime::Runtime>()
                    .tx
                    .send(runtime::Message::Stop);
            }
        });
}
