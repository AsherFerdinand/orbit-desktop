import { useRef, useState, useEffect } from "react";
import { AnimatePresence, motion, useReducedMotion } from "motion/react";
import { useIslandRegion } from "./hooks/useIslandRegion";
import { useSystemTheme } from "./hooks/useSystemTheme";
import { useOrbit } from "./hooks/useOrbit";
import { useMedia } from "./hooks/useMedia";
import { useNotes } from "./hooks/useNotes";
import { OrbitMark, BatteryIcon, SpeakerIcon } from "./components/Icons";
import { Artwork, Progress, PaperStack } from "./components/Ambient";
import { AmbientMedia } from "./modules/AmbientMedia";
import { Notes } from "./modules/Notes";
import { Settings } from "./modules/Settings";
import { native, hide, showSystemMenu } from "./services/native";
import { batteryLabel } from "./types";
export default function App() {
  const media = useMedia(),
    notes = useNotes();
  const [measuredHeight, setMeasuredHeight] = useState<number | null>(null);
  const unified = useRef<HTMLDivElement>(null);
  const o = useOrbit(
    notes.active,
    !!media.item,
    notes.notes.length > 0,
    measuredHeight,
  );
  const systemTheme = useSystemTheme(o.settings.theme === "system");
  const appearance =
    o.settings.theme === "system" ? systemTheme : o.settings.theme;
  useEffect(() => {
    if (!unified.current) return;
    const observer = new ResizeObserver(() => {
      const h = unified.current?.getBoundingClientRect().height ?? 0;
      if (h > 0)
        setMeasuredHeight(Math.min(550, Math.ceil(h + o.settings.height + 20)));
    });
    observer.observe(unified.current);
    return () => observer.disconnect();
  }, [o.settings.height]);
  const reduced = useReducedMotion(),
    [focusNotes, focus] = useState(0);
  const keyboard = useRef(false);
  const volumeHud =
    o.view === "closed" && o.notification?.kind === "VOLUME_CHANGED";
  const panelNotice =
    o.notification?.kind === "VOLUME_CHANGED" ? null : o.notification;
  const expanded = o.view !== "closed" || !!panelNotice;
  const b = o.settings.batteryEnabled ? o.battery : null;
  const spring = reduced
    ? { duration: 0 }
    : {
        type: "spring" as const,
        stiffness: 370 * o.settings.animationSpeed,
        damping: 32,
      };
  const openNotes = () => {
    o.setView("ambient");
    focus((n) => n + 1);
  };
  const region = useIslandRegion();
  useEffect(() => {
    region.update();
    const t = setTimeout(region.update, 200);
    return () => clearTimeout(t);
  }, [o.view, o.count, o.currentHeight, o.settings, volumeHud, region.update]);
  useEffect(() => {
    // A press may finish outside the shell; keep the existing focus guard until
    // release, then retain it only for keyboard or editable/control focus.
    const release = () => {
      const active = document.activeElement;
      o.setFocused(
        active instanceof HTMLElement &&
          !!region.shell.current?.contains(active) &&
          (keyboard.current ||
            active.matches("input,textarea,select,[contenteditable=true]")),
      );
    };
    window.addEventListener("pointerup", release);
    window.addEventListener("pointercancel", release);
    return () => {
      window.removeEventListener("pointerup", release);
      window.removeEventListener("pointercancel", release);
    };
  }, [o.setFocused, region.shell]);
  const system = o.snapshot?.system;
  return (
    <main
      className={
        "stage ambient-stage " + appearance + (native ? "" : " preview")
      }
    >
      {!native && (
        <aside className="preview-note">
          <OrbitMark />
          <h1>ORBIT</h1>
          <p>Your desktop, in motion.</p>
          <span>
            Interface preview · Windows readings and local notes connect in the
            desktop app.
          </span>
        </aside>
      )}
      <motion.div
        ref={region.shell}
        onUpdate={region.update}
        onAnimationComplete={region.update}
        className={"ambient-shell" + (volumeHud ? " volume-hud-active" : "")}
        onPointerEnter={() => {
          o.cancelCollapse();
          if (!expanded) o.scheduleExpand();
        }}
        onPointerLeave={() => {
          o.cancelExpand();
          o.scheduleCollapse();
        }}
        onPointerDownCapture={(e) => {
          (e.target as Element)
            .closest("button")
            ?.setPointerCapture(e.pointerId);
          keyboard.current = false;
          o.setFocused(true);
        }}
        onFocusCapture={() => o.setFocused(true)}
        onBlurCapture={(e) => {
          if (!e.currentTarget.contains(e.relatedTarget)) o.setFocused(false);
        }}
        onKeyDown={(e) => {
          if (
            e.shiftKey &&
            e.key === "F10" &&
            !(e.target as HTMLElement).closest("input,textarea")
          ) {
            e.preventDefault();
            void showSystemMenu().catch(o.report);
          }
          if (e.key === "Tab") keyboard.current = true;
          if (e.key === "Escape") o.setView("closed");
        }}
        initial={false}
        animate={{ width: expanded ? o.settings.width : 232 }}
        transition={spring}
      >
        <PaperStack
          update={region.update}
          count={o.count}
          active={notes.active}
          animate={o.settings.noteAnimations}
          open={openNotes}
        />
        <motion.section
          ref={region.body}
          onUpdate={region.update}
          onAnimationComplete={region.update}
          className="island ambient-island"
          aria-label="ORBIT desktop companion"
          initial={false}
          animate={{
            height: expanded
              ? panelNotice
                ? 220
                : o.currentHeight
              : o.settings.height,
            borderRadius: expanded ? 28 : 32,
          }}
          transition={spring}
          onContextMenu={(e) => {
            if ((e.target as HTMLElement).closest("input,textarea")) return;
            e.preventDefault();
            void showSystemMenu().catch(o.report);
          }}
        >
          <AnimatePresence initial={false}>
            {volumeHud && (
              <motion.div
                key="volume-hud"
                className="volume-hud"
                role="status"
                aria-live="polite"
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                exit={{ opacity: 0 }}
                transition={{ duration: reduced ? 0 : 0.12 }}
              >
                <VolumeReadout
                  muted={o.notification?.title === "Sound muted"}
                  label={
                    o.notification?.title === "Sound muted"
                      ? "Muted"
                      : (o.notification?.detail ?? "—")
                  }
                />
              </motion.div>
            )}
          </AnimatePresence>
          <header
            className="ambient-header"
            style={{ height: o.settings.height }}
          >
            <button
              className="ambient-brand"
              aria-label={expanded ? "ORBIT" : "Expand ORBIT"}
              aria-expanded={expanded}
              onClick={() => o.setView("ambient")}
            >
              {!expanded && media.item && o.settings.collapsedArtwork ? (
                <Artwork
                  src={media.item.artwork}
                  small
                  paused={media.item.status !== 4}
                />
              ) : (
                <OrbitMark />
              )}
              <span>ORBIT</span>
            </button>
            <div className="ambient-status">
              {b && (
                <span className="compact-battery" title={batteryLabel(b)}>
                  <BatteryIcon percent={b.percent} charging={b.charging} />
                  <span>{b.percent === null ? "—" : `${b.percent}%`}</span>
                </span>
              )}
              {!expanded && o.count > 0 && (
                <button
                  className="note-count"
                  onClick={openNotes}
                  aria-label={`Show ${notes.active} active notes`}
                >
                  {notes.active > 3 ? `+${notes.active - 3}` : "·"}
                </button>
              )}
              {expanded && (
                <button
                  className="collapse-button"
                  aria-label={
                    o.view === "settings" ? "Close preferences" : "Collapse"
                  }
                  onClick={() =>
                    o.setView(o.view === "settings" ? "ambient" : "closed")
                  }
                >
                  −
                </button>
              )}
            </div>
          </header>
          {!expanded && media.item && o.settings.playbackProgress && (
            <Progress item={media.item} active={o.visible && !volumeHud} thin />
          )}
          <div className="ambient-content" hidden={!expanded}>
            {panelNotice && (
              <div
                className="notification"
                role={panelNotice.priority >= 85 ? "alert" : "status"}
              >
                <small>ORBIT NOTICE</small>
                <strong>{panelNotice.title}</strong>
                <p>{panelNotice.detail}</p>
                <button onClick={o.dismiss}>Dismiss</button>
              </div>
            )}
            <div hidden={o.view !== "settings" || !!panelNotice}>
              <div className="preferences-title">
                Make it yours <span>0.4</span>
              </div>
              <Settings
                active={o.visible && o.view === "settings" && !panelNotice}
                value={o.settings}
                media={media}
                change={(s) => void o.change(s)}
                measure={() => void o.measure()}
                performanceResult={o.performanceResult}
              />
            </div>
            <div
              ref={unified}
              className="unified-panel"
              hidden={o.view === "settings" || !!panelNotice}
            >
              <AmbientMedia
                interaction={o.setMediaInteraction}
                media={media}
                progress={o.settings.playbackProgress}
                visible={
                  o.visible &&
                  expanded &&
                  o.view !== "settings" &&
                  !o.notification
                }
              />
              {o.settings.compactMetrics && (
                <div
                  className="compact-metrics"
                  aria-label="System information"
                >
                  <span title={batteryLabel(b)}>
                    {b?.charging ? "ϟ " : ""}
                    {b?.percent !== null && b?.percent !== undefined
                      ? `${b.percent}% battery`
                      : b?.present === false
                        ? "No battery"
                        : "Battery —"}
                  </span>
                  {o.settings.systemEnabled && (
                    <>
                      <span>
                        CPU{" "}
                        <b>
                          {system?.cpu == null
                            ? "—"
                            : `${system.cpu.toFixed(0)}%`}
                        </b>
                      </span>
                      <span
                        title={
                          system?.ramUsed != null && system?.ramTotal != null
                            ? `${(system.ramUsed / 1073741824).toFixed(1)} / ${(system.ramTotal / 1073741824).toFixed(1)} GiB`
                            : undefined
                        }
                      >
                        RAM{" "}
                        <b>
                          {system?.ramPercent == null
                            ? "—"
                            : `${system.ramPercent.toFixed(0)}%`}
                        </b>
                      </span>
                    </>
                  )}
                </div>
              )}
              <div title={o.snapshot?.audioError ?? "Windows system volume"}>
                <VolumeReadout
                  muted={o.snapshot?.volume?.muted ?? false}
                  label={
                    o.snapshot?.volume
                      ? o.snapshot.volume.muted
                        ? "Muted"
                        : o.snapshot.volume.percent + "%"
                      : "—"
                  }
                />
              </div>
              {o.settings.quickNotes && (
                <Notes
                  store={notes}
                  block={o.setEditing}
                  focusRequest={focusNotes}
                />
              )}
              {media.error && (
                <p className="inline-error" role="status">
                  {media.error}
                </p>
              )}
              <footer className="ambient-footer">
                <span
                  title={
                    system?.network === null
                      ? "Network unknown"
                      : system?.network
                        ? "Local adapter connected"
                        : "No connected local adapter"
                  }
                >
                  <i className={system?.network ? "connected" : ""} />
                  Your desktop, in motion.
                </span>
                <button
                  disabled={!native}
                  onClick={() => void hide().catch(o.report)}
                >
                  Hide ↗
                </button>
              </footer>
            </div>
            {o.error && (
              <p className="inline-error" role="alert">
                {o.error}
              </p>
            )}
          </div>
        </motion.section>
      </motion.div>
    </main>
  );
}

function VolumeReadout({ muted, label }: { muted: boolean; label: string }) {
  return (
    <span className="volume-readout" aria-label={"System volume: " + label}>
      <SpeakerIcon muted={muted} />
      <span>{label}</span>
    </span>
  );
}
