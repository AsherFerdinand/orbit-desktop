use tauri::{Emitter, LogicalSize, Manager, PhysicalPosition};
// All geometry is derived from the primary monitor work area and its DPI scale.
pub fn geometry(
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    scale: f64,
    width: u32,
    height: u32,
) -> (i32, i32, u32, u32) {
    let width = ((width as f64 * scale).round() as u32).min(w);
    let height = ((height as f64 * scale).round() as u32).min(h);
    let top = (12.0 * scale).round() as u32;
    (
        x + ((w - width) / 2) as i32,
        y + top.min(h - height) as i32,
        width,
        height,
    )
}
#[tauri::command]
pub fn resize_island(
    app: tauri::AppHandle,
    expanded: bool,
    width: u32,
    height: u32,
    panel_height: u32,
    paper_count: u32,
) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or("Main window unavailable")?;
    let monitor = window
        .primary_monitor()
        .map_err(|e| e.to_string())?
        .ok_or("Primary monitor unavailable")?;
    let area = monitor.work_area();
    let scale = monitor.scale_factor();
    let (x, y, w, h) = geometry(
        area.position.x,
        area.position.y,
        area.size.width,
        area.size.height,
        scale,
        if expanded {
            width.clamp(360, 480) + 16
        } else {
            248
        },
        if expanded {
            panel_height.clamp(180, 550) + 32
        } else {
            height.clamp(48, 64) + 32
        },
    );
    if window.outer_size().map_err(|e| e.to_string())? != tauri::PhysicalSize::new(w, h) {
        window
            .set_size(LogicalSize::new(w as f64 / scale, h as f64 / scale))
            .map_err(|e| e.to_string())?;
    }
    if window.outer_position().map_err(|e| e.to_string())? != PhysicalPosition::new(x, y) {
        window
            .set_position(PhysicalPosition::new(x, y))
            .map_err(|e| e.to_string())?;
    }
    memory_hint(&window, expanded);
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::Graphics::Gdi::{DeleteObject, SetWindowRgn};
        let region = island_region(w, h, scale, expanded, height, paper_count)?;
        let hwnd = window.hwnd().map_err(|e| e.to_string())?;
        if SetWindowRgn(hwnd.0 as _, region, 1) == 0 {
            DeleteObject(region);
            return Err(std::io::Error::last_os_error().to_string());
        }
    }
    Ok(())
}
// CSS and GDI share this 24px top inset and exact unrotated paper geometry.
#[cfg(windows)]
fn island_region(
    w: u32,
    h: u32,
    scale: f64,
    expanded: bool,
    height: u32,
    papers: u32,
) -> Result<windows_sys::Win32::Graphics::Gdi::HRGN, String> {
    use windows_sys::Win32::Graphics::Gdi::{CombineRgn, CreateRoundRectRgn, DeleteObject, RGN_OR};
    let px = |v: f64| (v * scale).round() as i32;
    unsafe {
        let margin = px(8.0);
        let top = px(24.0);
        let d = px(if expanded { 56.0 } else { height as f64 });
        let region = CreateRoundRectRgn(margin, top, w as i32 - margin, h as i32 - margin, d, d);
        if region.is_null() {
            return Err("Cannot create island region".into());
        }
        for i in 0..papers.min(3) {
            let x = w as i32 / 2 + px(28.0 + i as f64 * 10.0);
            let y = px(8.0 + i as f64 * 4.0);
            let paper = CreateRoundRectRgn(x, y, x + px(38.0), y + px(28.0), px(8.0), px(8.0));
            if paper.is_null() {
                DeleteObject(region);
                return Err("Cannot create paper region".into());
            }
            let ok = CombineRgn(region, region, paper, RGN_OR);
            DeleteObject(paper);
            if ok == 0 {
                DeleteObject(region);
                return Err("Cannot combine paper region".into());
            }
        }
        Ok(region)
    }
}
#[tauri::command]
pub fn hide_island(app: tauri::AppHandle) -> Result<(), String> {
    let w = app
        .get_webview_window("main")
        .ok_or("Main window unavailable")?;
    w.hide().map_err(|e| e.to_string())?;
    memory_hint(&w, false);
    let _ = w.emit("orbit-visible", false);
    if let Some(runtime) = app.try_state::<crate::runtime::Runtime>() {
        let _ = runtime.tx.send(crate::runtime::Message::Activity(false));
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn centers_at_high_dpi() {
        assert_eq!(
            geometry(0, 0, 1920, 1080, 1.5, 248, 72),
            (774, 18, 372, 108)
        );
    }
    #[test]
    fn stays_inside_negative_origin_work_area() {
        let (x, y, w, h) = geometry(-1280, 40, 1280, 680, 1.0, 496, 522);
        assert!(x >= -1280 && x + w as i32 <= 0);
        assert!(y >= 40 && y + h as i32 <= 720);
    }
    #[test]
    fn clamps_small_work_area() {
        assert_eq!(geometry(0, 0, 300, 400, 1.0, 496, 522), (0, 0, 300, 400));
    }
}

#[cfg(test)]
mod region_tests {
    #[test]
    fn rounded_region_excludes_transparent_margin_and_corner() {
        unsafe {
            use windows_sys::Win32::Graphics::Gdi::{CreateRoundRectRgn, DeleteObject, PtInRegion};
            let region = CreateRoundRectRgn(8, 8, 240, 64, 56, 56);
            assert!(!region.is_null());
            assert_eq!(PtInRegion(region, 2, 35), 0);
            assert_eq!(PtInRegion(region, 9, 9), 0);
            assert_ne!(PtInRegion(region, 120, 35), 0);
            DeleteObject(region);
        }
    }
}

pub static MEMORY_HINT_LOW: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);
pub fn memory_hint(window: &tauri::WebviewWindow, expanded: bool) {
    // Supported WebView2 policy, not a forced working-set trim or renderer suspension.
    let _ = window.with_webview(move |webview| unsafe {
        use webview2_com::Microsoft::Web::WebView2::Win32::{
            ICoreWebView2_19, COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW,
            COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL,
        };
        use windows::core::Interface;
        if let Ok(core) = webview.controller().CoreWebView2() {
            if let Ok(core) = core.cast::<ICoreWebView2_19>() {
                if core
                    .SetMemoryUsageTargetLevel(if expanded {
                        COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL
                    } else {
                        COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW
                    })
                    .is_ok()
                {
                    MEMORY_HINT_LOW.store(!expanded, std::sync::atomic::Ordering::Relaxed);
                }
            }
        }
    });
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HitTest {
    visible_island_interactive: bool,
    paper_tab_interactive: Option<bool>,
    space_above_island_passes_through: bool,
    transparent_margin_passes_through: bool,
    rounded_corner_passes_through: bool,
}
pub fn verify_hit_test(app: &tauri::AppHandle) -> Option<HitTest> {
    let window = app.get_webview_window("main")?;
    if !window.is_visible().ok()? {
        return None;
    }
    let hwnd = window.hwnd().ok()?.0 as windows_sys::Win32::Foundation::HWND;
    unsafe {
        use windows_sys::Win32::{
            Foundation::{POINT, RECT},
            UI::WindowsAndMessaging::{GetAncestor, GetWindowRect, WindowFromPoint, GA_ROOT},
        };
        let mut rect: RECT = std::mem::zeroed();
        if GetWindowRect(hwnd, &mut rect) == 0 {
            return None;
        }
        let root = |x, y| GetAncestor(WindowFromPoint(POINT { x, y }), GA_ROOT);
        let scale = window.scale_factor().ok()?;
        let corner = (9.0 * scale).round() as i32;
        let top = (25.0 * scale).round() as i32;
        let has_papers = crate::settings::load_settings(app.clone())
            .ok()
            .is_some_and(|s| s.quick_notes && s.paper_stack)
            && crate::notes::active_count(app).unwrap_or(0) > 0;
        Some(HitTest {
            paper_tab_interactive: has_papers.then(|| {
                root(
                    (rect.left + rect.right) / 2 + (40.0 * scale).round() as i32,
                    rect.top + (12.0 * scale).round() as i32,
                ) == hwnd
            }),
            space_above_island_passes_through: root(
                (rect.left + rect.right) / 2 - (24.0 * scale).round() as i32,
                rect.top + (12.0 * scale).round() as i32,
            ) != hwnd,
            visible_island_interactive: root(
                (rect.left + rect.right) / 2,
                (rect.top + rect.bottom) / 2,
            ) == hwnd,
            transparent_margin_passes_through: root(rect.left + 2, (rect.top + rect.bottom) / 2)
                != hwnd,
            rounded_corner_passes_through: root(rect.left + corner, rect.top + top) != hwnd,
        })
    }
}

#[cfg(test)]
mod paper_region_tests {
    use super::*;
    #[test]
    fn visible_papers_are_interactive_but_surroundings_are_not() {
        unsafe {
            use windows_sys::Win32::Graphics::Gdi::{DeleteObject, PtInRegion};
            for scale in [1.0, 1.25, 1.5, 2.0] {
                for count in 0..=3 {
                    let w = (248.0 * scale) as u32;
                    let h = (88.0 * scale) as u32;
                    let r = island_region(w, h, scale, false, 56, count).unwrap();
                    let pt = |x: f64, y: f64| {
                        PtInRegion(r, (x * scale).round() as i32, (y * scale).round() as i32) != 0
                    };
                    assert!(pt(124.0, 50.0));
                    assert!(!pt(2.0, 40.0));
                    assert!(!pt(100.0, 12.0));
                    assert!(!pt(9.0, 25.0));
                    assert_eq!(pt(160.0, 12.0), count > 0);
                    assert_eq!(pt(205.0, 21.0), count >= 3);
                    DeleteObject(r);
                }
            }
        }
    }
}
pub static VISUAL_REGION_UPDATES: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0);
use serde::Deserialize;
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VisualRegion {
    left: f64,
    top: f64,
    width: f64,
    height: f64,
    radius: f64,
}
#[tauri::command]
pub fn set_island_region(
    app: tauri::AppHandle,
    body: VisualRegion,
    papers: Vec<VisualRegion>,
) -> Result<(), String> {
    use windows_sys::Win32::Graphics::Gdi::{
        CombineRgn, CreateRoundRectRgn, DeleteObject, SetWindowRgn, RGN_OR,
    };
    if papers.len() > 3 {
        return Err("Too many paper regions".into());
    }
    let window = app.get_webview_window("main").ok_or("Window unavailable")?;
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    let make = |r: &VisualRegion| -> Result<windows_sys::Win32::Graphics::Gdi::HRGN, String> {
        if ![r.left, r.top, r.width, r.height, r.radius]
            .iter()
            .all(|v| v.is_finite())
            || r.width <= 0.0
            || r.height <= 0.0
            || r.width > 1000.0
            || r.height > 1000.0
            || r.left.abs() > 1000.0
            || r.top.abs() > 1000.0
            || !(0.0..=100.0).contains(&r.radius)
        {
            return Err("Invalid visual region".into());
        }
        let px = |v: f64| (v * scale).round() as i32;
        let region = unsafe {
            CreateRoundRectRgn(
                px(r.left),
                px(r.top),
                px(r.left + r.width),
                px(r.top + r.height),
                px(r.radius * 2.0),
                px(r.radius * 2.0),
            )
        };
        if region.is_null() {
            Err("Cannot create visual region".into())
        } else {
            Ok(region)
        }
    };
    let region = make(&body)?;
    unsafe {
        for r in papers {
            let paper = match make(&r) {
                Ok(p) => p,
                Err(e) => {
                    DeleteObject(region);
                    return Err(e);
                }
            };
            let ok = CombineRgn(region, region, paper, RGN_OR);
            DeleteObject(paper);
            if ok == 0 {
                DeleteObject(region);
                return Err("Cannot combine visual regions".into());
            }
        }
        let hwnd = match window.hwnd() {
            Ok(h) => h,
            Err(e) => {
                DeleteObject(region);
                return Err(e.to_string());
            }
        };
        if SetWindowRgn(hwnd.0 as _, region, 1) == 0 {
            DeleteObject(region);
            return Err(std::io::Error::last_os_error().to_string());
        }
    }
    VISUAL_REGION_UPDATES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    Ok(())
}
