use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fs,
    io::Write,
    path::Path,
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{Emitter, Manager};
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Note {
    pub id: String,
    pub text: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub completed: bool,
    #[serde(default)]
    pub reminder_at: Option<u64>,
    #[serde(default)]
    pub reminder_enabled: bool,
    #[serde(default)]
    pub reminder_fired: bool,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReminderChange {
    pub at: Option<u64>,
    pub enabled: bool,
}
#[derive(Default)]
pub struct NotesLock(Mutex<()>);
// Serialize claims/writes even if another ORBIT instance is running.
struct NotesGuard<'a> {
    _local: std::sync::MutexGuard<'a, ()>,
    handle: windows_sys::Win32::Foundation::HANDLE,
}
impl NotesLock {
    fn lock(&self) -> Result<NotesGuard<'_>, String> {
        use windows_sys::Win32::{
            Foundation::{CloseHandle, WAIT_ABANDONED, WAIT_OBJECT_0},
            System::Threading::{CreateMutexW, WaitForSingleObject},
        };
        let local = self.0.lock().map_err(|e| e.to_string())?;
        let name: Vec<u16> = "Local\\ORBIT.Notes.v1"
            .encode_utf16()
            .chain(Some(0))
            .collect();
        let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
        if handle.is_null() {
            return Err(std::io::Error::last_os_error().to_string());
        }
        let result = unsafe { WaitForSingleObject(handle, 5000) };
        if result != WAIT_OBJECT_0 && result != WAIT_ABANDONED {
            unsafe {
                CloseHandle(handle);
            }
            return Err("Notes are busy in another ORBIT instance; try again".into());
        }
        Ok(NotesGuard {
            _local: local,
            handle,
        })
    }
}
impl Drop for NotesGuard<'_> {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::System::Threading::ReleaseMutex(self.handle);
            windows_sys::Win32::Foundation::CloseHandle(self.handle);
        }
    }
}
const MAX_TIMESTAMP: u64 = 8_640_000_000_000_000;
pub const OVERDUE_GRACE_MS: u64 = 15 * 60 * 1000;
#[cfg(test)]
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
fn reminder_valid(note: &Note) -> bool {
    note.reminder_at
        .is_none_or(|at| at > 0 && at <= MAX_TIMESTAMP)
        && (!note.reminder_enabled
            || (note.reminder_at.is_some() && !note.reminder_fired && !note.completed))
}
fn change_reminder(note: &mut Note, change: &ReminderChange, now: u64) -> Result<(), String> {
    if change.enabled && (change.at.is_none() || note.completed) {
        return Err("Choose a reminder time for an incomplete note".into());
    }
    let new_occurrence = change.at != note.reminder_at;
    if change.at.is_some_and(|at| at == 0 || at > MAX_TIMESTAMP) {
        return Err("Invalid reminder timestamp".into());
    }
    if (new_occurrence || (change.enabled && !note.reminder_enabled))
        && change.enabled
        && change.at.is_some_and(|at| at <= now)
    {
        return Err("Choose a future reminder time".into());
    }
    if change.enabled && !new_occurrence && note.reminder_fired {
        return Err("Choose a new future time to rearm this reminder".into());
    }
    if new_occurrence {
        note.reminder_fired = false;
    }
    note.reminder_at = change.at;
    note.reminder_enabled = change.enabled && change.at.is_some();
    if note.reminder_at.is_none() {
        note.reminder_fired = false;
    }
    Ok(())
}
pub fn nearest_reminder(notes: &[Note]) -> Option<u64> {
    notes
        .iter()
        .filter(|n| !n.completed && n.reminder_enabled && !n.reminder_fired)
        .filter_map(|n| n.reminder_at)
        .min()
}
// Persist the claim before any OS call. Failure to save means no notifications.
fn claim_due(path: &Path, now: u64) -> Result<(Vec<Note>, Vec<Note>, bool), String> {
    let mut notes = load(path)?;
    let mut due = Vec::new();
    let mut changed = false;
    for note in &mut notes {
        if !note.completed
            && note.reminder_enabled
            && !note.reminder_fired
            && note.reminder_at.is_some_and(|at| at <= now)
        {
            if now.saturating_sub(note.reminder_at.unwrap()) <= OVERDUE_GRACE_MS {
                due.push(note.clone());
            }
            note.reminder_fired = true;
            note.reminder_enabled = false;
            note.updated_at = now.max(note.updated_at.saturating_add(1));
            changed = true;
        }
    }
    if changed {
        save(path, &notes)?;
    }
    Ok((notes, due, changed))
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    version: u8,
    notes: Vec<Note>,
}
fn text(value: &str) -> Result<String, String> {
    let v = value.trim();
    if v.is_empty()
        || v.chars().count() > 500
        || v.chars()
            .any(|c| c.is_control() && !matches!(c, '\n' | '\t'))
    {
        return Err("Notes must contain 1–500 plain-text characters".into());
    }
    Ok(v.into())
}
fn load(path: &Path) -> Result<Vec<Note>, String> {
    if !path.exists() {
        return Ok(vec![]);
    }
    if fs::metadata(path).map_err(|e| e.to_string())?.len() > 2 * 1_048_576 {
        return Err("Notes file exceeds the supported size; file preserved".into());
    }
    let data = fs::read(path).map_err(|e| format!("Cannot read notes: {e}"))?;
    let d:Document=serde_json::from_slice(&data).map_err(|e|format!("Notes file is damaged: {e}. Original preserved; restore notes.json.bak while ORBIT is closed."))?;
    let mut ids = HashSet::new();
    if d.version != 1
        || d.notes.len() > 500
        || d.notes.iter().any(|n| {
            n.id.is_empty()
                || n.id.len() > 100
                || !ids.insert(&n.id)
                || text(&n.text).is_err()
                || n.created_at == 0
                || n.updated_at < n.created_at
                || n.updated_at == u64::MAX
                || !reminder_valid(n)
        })
    {
        return Err("Notes file failed validation; original preserved".into());
    }
    Ok(d.notes)
}
fn save(path: &Path, notes: &[Note]) -> Result<(), String> {
    let parent = path.parent().ok_or("Invalid notes location")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let temp = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(&Document {
        version: 1,
        notes: notes.to_vec(),
    })
    .map_err(|e| e.to_string())?;
    if path.exists() {
        fs::copy(path, path.with_extension("json.bak"))
            .map_err(|e| format!("Cannot preserve notes backup: {e}"))?;
    }
    let mut file = fs::File::create(&temp).map_err(|e| e.to_string())?;
    file.write_all(&bytes).map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    drop(file);
    #[cfg(windows)]
    unsafe {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::{
            MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
        };
        let from: Vec<u16> = temp.as_os_str().encode_wide().chain(Some(0)).collect();
        let to: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        if MoveFileExW(
            from.as_ptr(),
            to.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        ) == 0
        {
            return Err(format!(
                "Cannot replace notes file: {}",
                std::io::Error::last_os_error()
            ));
        }
    }
    #[cfg(not(windows))]
    fs::rename(temp, path).map_err(|e| e.to_string())?;
    Ok(())
}
static SEQUENCE: AtomicU64 = AtomicU64::new(0);
#[cfg(test)]
fn mutate(
    path: &Path,
    action: &str,
    id: Option<&str>,
    value: Option<&str>,
    completed: Option<bool>,
) -> Result<Vec<Note>, String> {
    mutate_with_reminder(path, action, id, value, completed, None)
}
fn mutate_with_reminder(
    path: &Path,
    action: &str,
    id: Option<&str>,
    value: Option<&str>,
    completed: Option<bool>,
    reminder: Option<&ReminderChange>,
) -> Result<Vec<Note>, String> {
    let mut notes = load(path)?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?;
    match action {
        "create" => {
            if notes.len() >= 500 {
                return Err("Note limit reached (500); remove older notes first".into());
            }
            notes.push(Note {
                id: format!(
                    "n-{}-{}-{}",
                    now.as_nanos(),
                    std::process::id(),
                    SEQUENCE.fetch_add(1, Ordering::Relaxed)
                ),
                text: text(value.ok_or("Missing note text")?)?,
                created_at: now.as_millis() as u64,
                updated_at: now.as_millis() as u64,
                completed: false,
                reminder_at: None,
                reminder_enabled: false,
                reminder_fired: false,
            });
            if let Some(change) = reminder {
                change_reminder(notes.last_mut().unwrap(), change, now.as_millis() as u64)?;
            }
        }
        "edit" | "complete" | "delete" => {
            let at = notes
                .iter()
                .position(|n| Some(n.id.as_str()) == id)
                .ok_or("This note no longer exists")?;
            if action == "delete" {
                notes.remove(at);
            } else {
                let n = &mut notes[at];
                let before = n.clone();
                if action == "edit" {
                    n.text = text(value.ok_or("Missing note text")?)?;
                    if let Some(change) = reminder {
                        change_reminder(n, change, now.as_millis() as u64)?;
                    }
                } else {
                    n.completed = completed.ok_or("Missing completion state")?;
                    if n.completed {
                        n.reminder_enabled = false;
                    }
                }
                if *n == before {
                    return Ok(notes);
                }
                n.updated_at = (now.as_millis() as u64).max(n.updated_at + 1);
            }
        }
        _ => return Err("Unknown note action".into()),
    }
    save(path, &notes)?;
    Ok(notes)
}
fn path(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    Ok(app
        .path()
        .app_config_dir()
        .map_err(|e| e.to_string())?
        .join("notes.json"))
}
#[tauri::command]
pub fn list_notes(
    app: tauri::AppHandle,
    lock: tauri::State<NotesLock>,
) -> Result<Vec<Note>, String> {
    let _guard = lock.lock()?;
    load(&path(&app)?)
}
#[tauri::command]
pub fn mutate_note(
    app: tauri::AppHandle,
    lock: tauri::State<NotesLock>,
    action: String,
    id: Option<String>,
    text: Option<String>,
    completed: Option<bool>,
    reminder: Option<ReminderChange>,
) -> Result<Vec<Note>, String> {
    let _guard = lock.lock()?;
    let notes = mutate_with_reminder(
        &path(&app)?,
        &action,
        id.as_deref(),
        text.as_deref(),
        completed,
        reminder.as_ref(),
    )?;
    if let Some(runtime) = app.try_state::<crate::runtime::Runtime>() {
        let _ = runtime
            .tx
            .send(crate::runtime::Message::NotesChanged(nearest_reminder(
                &notes,
            )));
    }
    let _ = app.emit("orbit-notes", ());
    Ok(notes)
}
pub fn next_reminder(app: &tauri::AppHandle) -> Result<Option<u64>, String> {
    let lock = app.state::<NotesLock>();
    let _guard = lock.lock()?;
    Ok(nearest_reminder(&load(&path(app)?)?))
}
pub fn deliver_due(
    app: &tauri::AppHandle,
    now: u64,
    settings: &crate::settings::Settings,
) -> Result<Option<u64>, String> {
    let lock = app.state::<NotesLock>();
    let _guard = lock.lock()?;
    let (notes, due, changed) = claim_due(&path(app)?, now)?;
    for note in due {
        if let Err(error) = crate::reminders::notify(app, &note.text) {
            eprintln!("ORBIT reminder delivery failed (occurrence retired): {error}");
        }
        let _ = app.emit(
            "orbit-event",
            crate::events::Event::new(
                "NOTE_REMINDER_DUE",
                "Note reminder",
                note.text,
                90,
                settings,
            ),
        );
    }
    if changed {
        let _ = app.emit("orbit-notes", ());
    }
    Ok(nearest_reminder(&notes))
}
pub fn active_count(app: &tauri::AppHandle) -> Option<usize> {
    Some(
        load(&path(app).ok()?)
            .ok()?
            .iter()
            .filter(|n| !n.completed)
            .count(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(name: &str) -> std::path::PathBuf {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join(format!("notes-{name}-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir.join("notes.json")
    }
    #[test]
    fn lifecycle_and_restart() {
        let p = fixture("lifecycle");
        let first = mutate(&p, "create", None, Some("  Remember this  "), None).unwrap();
        let n = first[0].clone();
        assert_eq!(n.text, "Remember this");
        assert_eq!(load(&p).unwrap(), first);
        let unchanged = mutate(&p, "edit", Some(&n.id), Some(&n.text), None).unwrap();
        assert_eq!(unchanged[0], n);
        assert!(!p.with_extension("json.bak").exists());
        let unchanged = mutate(&p, "complete", Some(&n.id), None, Some(false)).unwrap();
        assert_eq!(unchanged[0], n);
        assert!(!p.with_extension("json.bak").exists());
        let edited = mutate(&p, "edit", Some(&n.id), Some("Edited"), None).unwrap();
        assert_eq!(edited[0].id, n.id);
        assert_eq!(edited[0].created_at, n.created_at);
        assert!(edited[0].updated_at > n.updated_at);
        let done = mutate(&p, "complete", Some(&n.id), None, Some(true)).unwrap();
        assert!(done[0].completed);
        assert!(load(&p).unwrap()[0].completed);
        let undone = mutate(&p, "complete", Some(&n.id), None, Some(false)).unwrap();
        assert!(!undone[0].completed);
        assert!(mutate(&p, "delete", Some(&n.id), None, None)
            .unwrap()
            .is_empty());
        assert!(load(&p).unwrap().is_empty());
        fs::remove_dir_all(p.parent().unwrap()).unwrap();
    }
    #[test]
    fn corrupt_file_is_never_overwritten() {
        let p = fixture("corrupt");
        fs::write(&p, b"{broken").unwrap();
        assert!(load(&p).is_err());
        assert!(mutate(&p, "create", None, Some("New"), None).is_err());
        assert_eq!(fs::read(&p).unwrap(), b"{broken");
        fs::remove_dir_all(p.parent().unwrap()).unwrap();
    }
    #[test]
    fn validation_backup_and_unique_ids() {
        let p = fixture("validate");
        assert!(mutate(&p, "create", None, Some(" "), None).is_err());
        assert!(text(&"x".repeat(501)).is_err());
        assert!(text("a\0b").is_err());
        let a = mutate(
            &p,
            "create",
            None,
            Some("<script>plain text</script>"),
            None,
        )
        .unwrap();
        let b = mutate(&p, "create", None, Some("second"), None).unwrap();
        assert_ne!(b[0].id, b[1].id);
        assert_eq!(load(&p.with_extension("json.bak")).unwrap(), a);
        assert!(mutate(&p, "edit", Some("missing"), Some("x"), None).is_err());
        fs::remove_dir_all(p.parent().unwrap()).unwrap();
    }
    #[test]
    fn duplicate_ids_and_unknown_version_rejected() {
        let p = fixture("schema");
        let a = mutate(&p, "create", None, Some("x"), None).unwrap();
        fs::write(
            &p,
            serde_json::to_vec(&Document {
                version: 1,
                notes: vec![a[0].clone(), a[0].clone()],
            })
            .unwrap(),
        )
        .unwrap();
        assert!(load(&p).is_err());
        fs::write(&p, b"{\"version\":2,\"notes\":[]}").unwrap();
        assert!(load(&p).is_err());
        fs::remove_dir_all(p.parent().unwrap()).unwrap();
    }
    #[test]
    fn legacy_notes_and_reminder_fields_survive_reload() {
        let p = fixture("reminder-migration");
        fs::write(&p, br#"{"version":1,"notes":[{"id":"legacy","text":"Old note","createdAt":1,"updatedAt":1,"completed":false}]}"#).unwrap();
        let old = load(&p).unwrap();
        assert_eq!(old[0].reminder_at, None);
        assert!(!old[0].reminder_enabled && !old[0].reminder_fired);
        let at = now_ms() + 120_000;
        let edited = mutate_with_reminder(
            &p,
            "edit",
            Some("legacy"),
            Some("Old note"),
            None,
            Some(&ReminderChange {
                at: Some(at),
                enabled: true,
            }),
        )
        .unwrap();
        assert_eq!(load(&p).unwrap(), edited);
        assert_eq!(nearest_reminder(&load(&p).unwrap()), Some(at));
        assert_eq!(load(&p.with_extension("json.bak")).unwrap(), old);
        fs::remove_dir_all(p.parent().unwrap()).unwrap();
    }
    fn add_reminder(p: &Path, at: u64) -> Note {
        mutate_with_reminder(
            p,
            "create",
            None,
            Some("Reminder fixture"),
            None,
            Some(&ReminderChange {
                at: Some(at),
                enabled: true,
            }),
        )
        .unwrap()
        .last()
        .unwrap()
        .clone()
    }
    #[test]
    fn nearest_deadline_does_not_fire_early_and_claims_once_across_restart() {
        let p = fixture("reminder-due");
        let at = now_ms() + 120_000;
        let later = add_reminder(&p, at + 60_000);
        let first = add_reminder(&p, at);
        assert_eq!(nearest_reminder(&load(&p).unwrap()), Some(at));
        let (_, due, changed) = claim_due(&p, at - 1).unwrap();
        assert!(due.is_empty() && !changed);
        let (notes, due, _) = claim_due(&p, at).unwrap();
        assert_eq!(due[0].id, first.id);
        assert!(
            notes
                .iter()
                .find(|n| n.id == first.id)
                .unwrap()
                .reminder_fired
        );
        assert_eq!(
            nearest_reminder(&load(&p).unwrap()),
            Some(later.reminder_at.unwrap())
        );
        assert!(claim_due(&p, at).unwrap().1.is_empty());
        assert_eq!(claim_due(&p, at + 60_000).unwrap().1[0].id, later.id);
        assert!(claim_due(&p, at + 60_001).unwrap().1.is_empty());
        fs::remove_dir_all(p.parent().unwrap()).unwrap();
    }
    #[test]
    fn completion_deletion_and_editing_control_reminder_occurrences() {
        let p = fixture("reminder-mutations");
        let at = now_ms() + 120_000;
        let n = add_reminder(&p, at);
        let done = mutate(&p, "complete", Some(&n.id), None, Some(true)).unwrap();
        assert_eq!(nearest_reminder(&done), None);
        let reopened = mutate(&p, "complete", Some(&n.id), None, Some(false)).unwrap();
        assert!(!reopened[0].reminder_enabled);
        assert!(claim_due(&p, at).unwrap().1.is_empty());
        let change = ReminderChange {
            at: Some(at + 60_000),
            enabled: true,
        };
        mutate_with_reminder(
            &p,
            "edit",
            Some(&n.id),
            Some("Rearmed"),
            None,
            Some(&change),
        )
        .unwrap();
        assert_eq!(claim_due(&p, at + 60_000).unwrap().1.len(), 1);
        let text_only = mutate(
            &p,
            "edit",
            Some(&n.id),
            Some("Text after notification"),
            None,
        )
        .unwrap();
        assert!(text_only[0].reminder_fired && !text_only[0].reminder_enabled);
        let changed = mutate_with_reminder(
            &p,
            "edit",
            Some(&n.id),
            Some("Next occurrence"),
            None,
            Some(&ReminderChange {
                at: Some(at + 120_000),
                enabled: true,
            }),
        )
        .unwrap();
        assert!(!changed[0].reminder_fired);
        assert_eq!(nearest_reminder(&changed), Some(at + 120_000));
        mutate(&p, "delete", Some(&n.id), None, None).unwrap();
        assert!(claim_due(&p, at + 120_000).unwrap().1.is_empty());
        assert_eq!(nearest_reminder(&load(&p).unwrap()), None);
        fs::remove_dir_all(p.parent().unwrap()).unwrap();
    }
    #[test]
    fn overdue_grace_fires_recent_occurrences_and_retires_stale_ones() {
        let p = fixture("reminder-overdue");
        let at = now_ms() + 120_000;
        let stale = add_reminder(&p, at);
        let boundary = add_reminder(&p, at + 60_000);
        let recent = add_reminder(&p, at + 120_000);
        let (notes, due, changed) = claim_due(&p, at + 60_000 + OVERDUE_GRACE_MS).unwrap();
        assert!(changed);
        assert_eq!(
            due.iter().map(|n| &n.id).collect::<Vec<_>>(),
            vec![&boundary.id, &recent.id]
        );
        assert!(!due.iter().any(|n| n.id == stale.id));
        assert!(notes
            .iter()
            .all(|n| n.reminder_fired && !n.reminder_enabled));
        assert!(claim_due(&p, at + 60_000 + OVERDUE_GRACE_MS + 1)
            .unwrap()
            .1
            .is_empty());
        fs::remove_dir_all(p.parent().unwrap()).unwrap();
    }
    #[test]
    fn failed_claim_write_preserves_pending_occurrence_and_attempts_nothing() {
        let p = fixture("reminder-write-error");
        let at = now_ms() + 120_000;
        add_reminder(&p, at);
        fs::create_dir(p.with_extension("json.tmp")).unwrap();
        assert!(claim_due(&p, at).is_err());
        let pending = load(&p).unwrap();
        assert!(!pending[0].reminder_fired && pending[0].reminder_enabled);
        fs::remove_dir_all(p.parent().unwrap()).unwrap();
    }
    #[test]
    fn named_notes_lock_excludes_another_instance_guard() {
        let first = NotesLock::default();
        let guard = first.lock().unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            let second = NotesLock::default();
            let _guard = second.lock().unwrap();
            tx.send(()).unwrap();
        });
        assert!(rx
            .recv_timeout(std::time::Duration::from_millis(50))
            .is_err());
        drop(guard);
        rx.recv_timeout(std::time::Duration::from_secs(3)).unwrap();
        worker.join().unwrap();
    }
}
