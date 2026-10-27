//! Read-only diagnostic. Run in the same interactive Windows session as Brave.
//! No browser extension, playback command, network request, or permanent log.
use serde_json::json;
use windows::{Media::Control::*, Win32::System::WinRT::*};
fn main() {
    let result = unsafe { RoInitialize(RO_INIT_MULTITHREADED) };
    if let Err(e) = result {
        println!("{}", json!({"error":e.to_string()}));
        return;
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap();
    let result = runtime.block_on(async {
        let manager = tokio::time::timeout(std::time::Duration::from_secs(5), GlobalSystemMediaTransportControlsSessionManager::RequestAsync()?).await.map_err(|_| windows::core::Error::new(windows::core::HRESULT(0x800705B4u32 as i32), "GSMTC request timed out"))??;
        let mut sessions = Vec::new();
        for s in manager.GetSessions()? {
            let info = s.GetPlaybackInfo()?; let controls = info.Controls()?;
            let properties = match s.TryGetMediaPropertiesAsync() { Ok(op) => tokio::time::timeout(std::time::Duration::from_secs(3),op).await.ok().and_then(Result::ok), Err(_) => None };
            let timeline = s.GetTimelineProperties().ok();
            sessions.push(json!({
                "source":s.SourceAppUserModelId()?.to_string(),
                "titleProvided":properties.as_ref().and_then(|p|p.Title().ok()).is_some_and(|v|!v.is_empty()),
                "artistProvided":properties.as_ref().and_then(|p|p.Artist().ok()).is_some_and(|v|!v.is_empty()),
                "thumbnailProvided":properties.as_ref().is_some_and(|p|p.Thumbnail().is_ok()),
                "status":info.PlaybackStatus()?.0,
                "play":controls.IsPlayEnabled()?,"pause":controls.IsPauseEnabled()?,"toggle":controls.IsPlayPauseToggleEnabled()?,"next":controls.IsNextEnabled()?,"previous":controls.IsPreviousEnabled()?,
                "positionSeconds":timeline.as_ref().and_then(|t|t.Position().ok()).map(|v|v.Duration as f64/10_000_000.0),
                "endSeconds":timeline.as_ref().and_then(|t|t.EndTime().ok()).map(|v|v.Duration as f64/10_000_000.0)
            }));
        }
        Ok::<_,windows::core::Error>(json!({"managerAvailable":true,"sessionCount":sessions.len(),"sessions":sessions}))
    });
    match result {
        Ok(value) => println!("{}", serde_json::to_string_pretty(&value).unwrap()),
        Err(e) => println!(
            "{}",
            json!({"managerAvailable":false,"error":e.to_string()})
        ),
    }
    unsafe {
        RoUninitialize();
    }
}
