import { useEffect, useRef, useState } from "react";
import { ClockIcon } from "../components/Icons";
import { reminderInput, reminderTimestamp } from "../services/notes";
import type { useNotes } from "../hooks/useNotes";
export function Notes({
  store,
  block,
  focusRequest,
}: {
  store: ReturnType<typeof useNotes>;
  block: (value: boolean) => void;
  focusRequest: number;
}) {
  const [opened, setOpened] = useState<string | null>(null);
  const [draft, setDraft] = useState(""),
    [editing, setEditing] = useState<string | null>(null),
    [edit, setEdit] = useState(""),
    [showDone, setShowDone] = useState(false);
  const [reminderOpen, setReminderOpen] = useState(false),
    [reminderTime, setReminderTime] = useState(""),
    [editReminder, setEditReminder] = useState(""),
    [editReminderEnabled, setEditReminderEnabled] = useState(false),
    [reminderError, setReminderError] = useState("");
  const focusAfterSave = useRef(false);
  const reminderEdited = useRef(false);
  const input = useRef<HTMLInputElement>(null),
    list = useRef<HTMLDivElement>(null);
  useEffect(() => {
    block(!!draft.length || editing !== null || reminderOpen || !!reminderTime);
    return () => block(false);
  }, [draft, editing, reminderOpen, reminderTime, block]);
  useEffect(() => {
    if (focusRequest) {
      list.current?.scrollIntoView({ block: "nearest" });
      input.current?.focus();
    }
  }, [focusRequest]);
  useEffect(() => {
    if (focusAfterSave.current && !store.busy) {
      focusAfterSave.current = false;
      input.current?.focus();
    }
  }, [store.busy, draft]);
  async function add() {
    if (!draft.trim() || store.busy) return;
    try {
      setReminderError("");
      const at = reminderTimestamp(reminderTime);
      if (
        await store.mutate("create", null, draft, null, {
          at,
          enabled: at !== null,
        })
      ) {
        focusAfterSave.current = true;
        setDraft("");
        setReminderTime("");
        setReminderOpen(false);
      }
    } catch (e) {
      setReminderError(String(e));
    }
  }
  async function save(id: string) {
    const original = store.notes.find((n) => n.id === id);
    try {
      setReminderError("");
      const reminder = reminderEdited.current
        ? {
            at: reminderTimestamp(editReminder, original?.reminderAt ?? null),
            enabled: editReminderEnabled && !!editReminder,
          }
        : undefined;
      if (await store.mutate("edit", id, edit, null, reminder))
        setEditing(null);
    } catch (e) {
      setReminderError(String(e));
    }
  }
  const shown = store.notes
    .filter((n) => showDone || !n.completed)
    .slice()
    .reverse();
  return (
    <section className="quick-notes" aria-label="Quick notes">
      <div className="notes-label">
        <span>
          QUICK NOTES <b>{store.active}</b>
        </span>
        {store.notes.some((n) => n.completed) && (
          <button
            aria-pressed={showDone}
            onClick={() => setShowDone((v) => !v)}
          >
            {showDone ? "Hide completed" : "Completed"}
          </button>
        )}
      </div>
      <form
        onSubmit={(e) => {
          e.preventDefault();
          void add();
        }}
      >
        <input
          ref={input}
          aria-label="Quick note"
          placeholder="Write a quick note..."
          value={draft}
          maxLength={1000}
          disabled={!store.ready || store.busy}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Escape" && (draft || reminderOpen || reminderTime)) {
              e.stopPropagation();
              setDraft("");
              setReminderTime("");
              setReminderOpen(false);
              setReminderError("");
            }
          }}
        />
        <button
          type="button"
          className="reminder-toggle"
          aria-label="Add a reminder"
          aria-expanded={reminderOpen}
          aria-pressed={!!reminderTime}
          title={
            reminderTime
              ? new Date(reminderTime).toLocaleString()
              : "Add reminder"
          }
          disabled={!store.ready || store.busy}
          onClick={() => setReminderOpen(!reminderOpen)}
        >
          <ClockIcon />
        </button>
        <button
          type="submit"
          disabled={!store.ready || store.busy || !draft.trim()}
          aria-label="Add note"
        >
          Add
        </button>
      </form>
      {reminderOpen && (
        <div className="reminder-fields">
          <label>
            Remind me{" "}
            <input
              type="datetime-local"
              aria-label="New note reminder"
              value={reminderTime}
              disabled={store.busy}
              onChange={(e) => setReminderTime(e.target.value)}
            />
          </label>
          <button
            type="button"
            onClick={() => {
              setReminderTime("");
              setReminderOpen(false);
              setReminderError("");
            }}
          >
            Remove reminder
          </button>
          <small>ORBIT must be running. Local Windows time.</small>
        </div>
      )}
      {shown.length > 0 && (
        <div
          className="note-list"
          ref={list}
          tabIndex={0}
          aria-label="Saved notes"
        >
          {shown.map((n) => (
            <div
              className={
                "note-row " +
                (n.completed ? "completed " : "") +
                (opened === n.id ? "open" : "")
              }
              key={n.id}
            >
              {editing === n.id ? (
                <form
                  onSubmit={(e) => {
                    e.preventDefault();
                    void save(n.id);
                  }}
                >
                  <input
                    autoFocus
                    aria-label="Edit note"
                    value={edit}
                    maxLength={1000}
                    onChange={(e) => setEdit(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === "Escape") {
                        e.stopPropagation();
                        setEditing(null);
                      }
                    }}
                  />
                  <div className="reminder-fields">
                    <label>
                      <input
                        type="checkbox"
                        aria-label="Enable note reminder"
                        disabled={store.busy || n.completed}
                        checked={editReminderEnabled}
                        onChange={(e) => {
                          reminderEdited.current = true;
                          setEditReminderEnabled(e.target.checked);
                        }}
                      />
                      Reminder
                    </label>
                    <input
                      type="datetime-local"
                      aria-label="Edit note reminder"
                      value={editReminder}
                      disabled={store.busy || n.completed}
                      onChange={(e) => {
                        reminderEdited.current = true;
                        setEditReminder(e.target.value);
                        setEditReminderEnabled(!!e.target.value);
                      }}
                    />
                    <button
                      type="button"
                      disabled={store.busy}
                      onClick={() => {
                        reminderEdited.current = true;
                        setEditReminder("");
                        setEditReminderEnabled(false);
                      }}
                    >
                      Remove
                    </button>
                  </div>
                  <button disabled={store.busy || !edit.trim()}>Save</button>
                  <button type="button" onClick={() => setEditing(null)}>
                    Cancel
                  </button>
                </form>
              ) : (
                <>
                  <button
                    className="note-check"
                    aria-label={n.completed ? "Reopen note" : "Complete note"}
                    aria-pressed={n.completed}
                    disabled={store.busy}
                    onClick={() =>
                      void store.mutate("complete", n.id, null, !n.completed)
                    }
                  >
                    {n.completed ? "✓" : "○"}
                  </button>
                  <button
                    className="note-text"
                    aria-expanded={opened === n.id}
                    title={n.text}
                    onClick={() => setOpened(opened === n.id ? null : n.id)}
                  >
                    {n.text}
                  </button>
                  {n.reminderAt && (
                    <span
                      className={
                        "note-reminder" + (n.reminderEnabled ? " active" : "")
                      }
                      title={
                        (n.reminderFired
                          ? "Processed: "
                          : n.reminderEnabled
                            ? "Reminder: "
                            : "Disabled: ") +
                        new Date(n.reminderAt).toLocaleString()
                      }
                      aria-label={
                        "Reminder " + new Date(n.reminderAt).toLocaleString()
                      }
                    >
                      <ClockIcon />
                    </span>
                  )}
                  <button
                    className="note-edit"
                    aria-label="Edit saved note"
                    disabled={store.busy}
                    onClick={() => {
                      reminderEdited.current = false;
                      setEdit(n.text);
                      setEditReminder(reminderInput(n.reminderAt));
                      setEditReminderEnabled(!!n.reminderEnabled);
                      setReminderError("");
                      setEditing(n.id);
                    }}
                  >
                    Edit
                  </button>
                  <button
                    className="note-delete"
                    aria-label="Delete note"
                    disabled={store.busy}
                    onClick={() =>
                      void store.mutate("delete", n.id, null, null)
                    }
                  >
                    ×
                  </button>
                </>
              )}
            </div>
          ))}
        </div>
      )}
      {reminderError && (
        <p className="inline-error" role="alert">
          {reminderError}
        </p>
      )}
      {store.error && (
        <p className="inline-error" role="alert">
          {store.error}
        </p>
      )}
      {!store.ready && !store.error && <small>Loading local notes…</small>}
    </section>
  );
}
