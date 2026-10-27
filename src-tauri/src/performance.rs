use serde::Serialize;
use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};
use tauri::Manager;
use windows_sys::Win32::{
    Foundation::{CloseHandle, FILETIME, INVALID_HANDLE_VALUE},
    System::{
        Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
            TH32CS_SNAPPROCESS,
        },
        ProcessStatus::{K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS_EX},
        Threading::{
            GetActiveProcessorCount, GetProcessTimes, OpenProcess, PROCESS_QUERY_INFORMATION,
            PROCESS_VM_READ,
        },
    },
};
#[derive(Clone)]
struct Process {
    ticks: u64,
    working: u64,
    private: u64,
}
fn processes() -> Result<(HashMap<u32, Process>, usize), String> {
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return Err(std::io::Error::last_os_error().to_string());
        }
        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut parents = Vec::new();
        let mut ok = Process32FirstW(snapshot, &mut entry);
        while ok != 0 {
            parents.push((entry.th32ProcessID, entry.th32ParentProcessID));
            ok = Process32NextW(snapshot, &mut entry);
        }
        CloseHandle(snapshot);
        let mut ids = HashSet::from([std::process::id()]);
        loop {
            let previous = ids.len();
            for (id, parent) in &parents {
                if ids.contains(parent) {
                    ids.insert(*id);
                }
            }
            if ids.len() == previous {
                break;
            }
        }
        let expected = ids.len();
        let mut result = HashMap::new();
        for id in ids {
            let process = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, 0, id);
            if process.is_null() {
                continue;
            }
            let mut mem: PROCESS_MEMORY_COUNTERS_EX = std::mem::zeroed();
            mem.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32;
            let (mut created, mut exit, mut kernel, mut user): (
                FILETIME,
                FILETIME,
                FILETIME,
                FILETIME,
            ) = std::mem::zeroed();
            if K32GetProcessMemoryInfo(
                process,
                (&mut mem as *mut PROCESS_MEMORY_COUNTERS_EX).cast(),
                mem.cb,
            ) != 0
                && GetProcessTimes(process, &mut created, &mut exit, &mut kernel, &mut user) != 0
            {
                let time = |t: FILETIME| ((t.dwHighDateTime as u64) << 32) | t.dwLowDateTime as u64;
                result.insert(
                    id,
                    Process {
                        ticks: time(kernel) + time(user),
                        working: mem.WorkingSetSize as u64,
                        private: mem.PrivateUsage as u64,
                    },
                );
            }
            CloseHandle(process);
        }
        Ok((result, expected))
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    visual_region_updates: u64,
    app_version: &'static str,
    context: String,
    sample_seconds: f64,
    webview_memory_hint_low: bool,
    window_hit_test: Option<crate::window::HitTest>,
    logical_processors: u32,
    cpu_percent: f64,
    working_set_mib: f64,
    private_mib: f64,
    measured_processes: usize,
    discovered_processes: usize,
    notes: &'static str,
}
fn measure() -> Result<Report, String> {
    std::thread::sleep(Duration::from_secs(2));
    let (before, _) = processes()?;
    let before_regions =
        crate::window::VISUAL_REGION_UPDATES.load(std::sync::atomic::Ordering::Relaxed);
    let start = Instant::now();
    std::thread::sleep(Duration::from_secs(20));
    let (after, discovered) = processes()?;
    if !after.contains_key(&std::process::id()) {
        return Err("Process counters unavailable".into());
    }
    let elapsed = start.elapsed().as_secs_f64();
    let cpu_ticks: u64 = after
        .iter()
        .filter_map(|(id, p)| before.get(id).map(|b| p.ticks.saturating_sub(b.ticks)))
        .sum();
    let count = unsafe { GetActiveProcessorCount(0xffff) }.max(1);
    Ok(Report {
        visual_region_updates:crate::window::VISUAL_REGION_UPDATES.load(std::sync::atomic::Ordering::Relaxed).saturating_sub(before_regions),
        app_version: env!("CARGO_PKG_VERSION"),
        context: "unspecified".into(),
        sample_seconds: elapsed,
        window_hit_test: None,
        webview_memory_hint_low: crate::window::MEMORY_HINT_LOW.load(std::sync::atomic::Ordering::Relaxed),
        logical_processors: count,
        cpu_percent: cpu_ticks as f64 / 10_000_000.0 / elapsed / count as f64 * 100.0,
        working_set_mib: after.values().map(|p| p.working).sum::<u64>() as f64 / 1_048_576.0,
        private_mib: after.values().map(|p| p.private).sum::<u64>() as f64 / 1_048_576.0,
        measured_processes: after.len(),
        discovered_processes: discovered,
        notes: "Own process and descendants including WebView2. Working sets can double-count shared pages. CPU includes processes present at both endpoints; transient processes may be missed. This is a short sample, not a leak test.",
    })
}
#[tauri::command]
pub async fn measure_performance(
    app: tauri::AppHandle,
    context: Option<String>,
) -> Result<String, String> {
    let mut report = tauri::async_runtime::spawn_blocking(measure)
        .await
        .map_err(|e| e.to_string())??;
    report.context = context
        .filter(|s| s == "collapsed-idle" || s == "current-view")
        .unwrap_or_else(|| "unspecified".into());
    report.window_hit_test = crate::window::verify_hit_test(&app);
    let json = serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?;
    let directory = app.path().app_config_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    std::fs::write(directory.join("performance.json"), &json).map_err(|e| e.to_string())?;
    Ok(json)
}
