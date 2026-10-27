use serde::Serialize;
use windows_sys::Win32::{
    Foundation::FILETIME,
    NetworkManagement::{
        IpHelper::{FreeMibTable, GetIfTable2, MIB_IF_TABLE2},
        Ndis::IfOperStatusUp,
    },
    System::{
        SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX},
        Threading::GetSystemTimes,
    },
};
#[derive(Clone, Debug, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
    pub cpu: Option<f64>,
    pub ram_used: Option<u64>,
    pub ram_total: Option<u64>,
    pub ram_percent: Option<f64>,
    pub network: Option<bool>,
}
#[derive(Clone, Copy, Debug)]
pub struct CpuTicks {
    pub idle: u64,
    pub kernel: u64,
    pub user: u64,
}
pub fn cpu_delta(previous: Option<CpuTicks>, current: CpuTicks) -> Option<f64> {
    let p = previous?;
    let total = current
        .kernel
        .checked_sub(p.kernel)?
        .checked_add(current.user.checked_sub(p.user)?)?;
    let idle = current.idle.checked_sub(p.idle)?;
    if total == 0 || idle > total {
        return None;
    }
    Some(100.0 * (total - idle) as f64 / total as f64)
}
pub fn memory(total: u64, available: u64) -> Option<(u64, f64)> {
    if total == 0 || available > total {
        return None;
    }
    let used = total - available;
    Some((used, 100.0 * used as f64 / total as f64))
}
fn ticks() -> Option<CpuTicks> {
    unsafe {
        let (mut i, mut k, mut u): (FILETIME, FILETIME, FILETIME) = std::mem::zeroed();
        if GetSystemTimes(&mut i, &mut k, &mut u) == 0 {
            return None;
        }
        let value = |t: FILETIME| ((t.dwHighDateTime as u64) << 32) | t.dwLowDateTime as u64;
        Some(CpuTicks {
            idle: value(i),
            kernel: value(k),
            user: value(u),
        })
    }
}
pub fn network() -> Option<bool> {
    unsafe {
        let mut table: *mut MIB_IF_TABLE2 = std::ptr::null_mut();
        if GetIfTable2(&mut table) != 0 || table.is_null() {
            return None;
        }
        let rows =
            std::slice::from_raw_parts((*table).Table.as_ptr(), (*table).NumEntries as usize);
        let connected = rows
            .iter()
            .any(|r| r.Type != 24 && r.OperStatus == IfOperStatusUp);
        FreeMibTable(table.cast());
        Some(connected)
    }
}
pub fn sample(previous: &mut Option<CpuTicks>) -> SystemInfo {
    let current = ticks();
    let cpu = current.and_then(|c| cpu_delta(*previous, c));
    *previous = current;
    let mut result = SystemInfo {
        cpu,
        ..Default::default()
    };
    unsafe {
        let mut m: MEMORYSTATUSEX = std::mem::zeroed();
        m.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
        if GlobalMemoryStatusEx(&mut m) != 0 {
            if let Some((used, pct)) = memory(m.ullTotalPhys, m.ullAvailPhys) {
                result.ram_used = Some(used);
                result.ram_total = Some(m.ullTotalPhys);
                result.ram_percent = Some(pct);
            }
        }
    }
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cpu_initial_and_zero_interval() {
        let t = CpuTicks {
            idle: 10,
            kernel: 20,
            user: 10,
        };
        assert_eq!(cpu_delta(None, t), None);
        assert_eq!(cpu_delta(Some(t), t), None);
    }
    #[test]
    fn cpu_includes_idle_in_kernel() {
        let p = CpuTicks {
            idle: 100,
            kernel: 200,
            user: 100,
        };
        let c = CpuTicks {
            idle: 140,
            kernel: 260,
            user: 140,
        };
        assert_eq!(cpu_delta(Some(p), c), Some(60.0));
    }
    #[test]
    fn cpu_reset_is_unavailable() {
        let p = CpuTicks {
            idle: 100,
            kernel: 200,
            user: 100,
        };
        assert_eq!(
            cpu_delta(
                Some(p),
                CpuTicks {
                    idle: 1,
                    kernel: 2,
                    user: 1
                }
            ),
            None
        );
    }
    #[test]
    fn memory_calculations() {
        assert_eq!(memory(8000, 3000), Some((5000, 62.5)));
        assert_eq!(memory(0, 0), None);
        assert_eq!(memory(10, 20), None);
    }
    #[test]
    fn native_system_read() {
        let mut baseline = None;
        let first = sample(&mut baseline);
        assert_eq!(first.cpu, None);
        assert!(first.ram_total.is_some());
        std::thread::sleep(std::time::Duration::from_millis(100));
        let second = sample(&mut baseline);
        assert!(second.cpu.is_some_and(|v| (0.0..=100.0).contains(&v)));
        println!("Real system: {second:?}");
    }
}
