import { memo, useState, useEffect, useMemo, useRef } from "react";
import { AnimatePresence, motion, useReducedMotion } from "motion/react";
import {
  paperCount,
  progressAt,
  reliableTimeline,
  canSeek,
  seekPosition,
  type MediaItem,
} from "../services/media";
export const Artwork = memo(function Artwork({
  src,
  small = false,
  paused = false,
}: {
  src: string | null;
  small?: boolean;
  paused?: boolean;
}) {
  const [failed, setFailed] = useState<string | null>(null);
  useEffect(() => setFailed(null), [src]);
  const reduced = useReducedMotion();
  const valid = artworkVisible(src, failed);
  return (
    <div
      className={
        "artwork " + (small ? "small " : "") + (paused ? "paused" : "")
      }
      aria-label={valid ? "Session artwork" : "Artwork unavailable"}
    >
      <AnimatePresence initial={false}>
        <motion.div
          key={valid ? src : "fallback"}
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          transition={{ duration: reduced ? 0 : 0.18 }}
        >
          {valid ? (
            <img
              src={src ?? undefined}
              alt="Media artwork"
              onError={() => setFailed(src)}
            />
          ) : (
            <span aria-label="Music">♫</span>
          )}
        </motion.div>
      </AnimatePresence>
      {paused && <i aria-label="Paused" />}
    </div>
  );
});
export const Progress = memo(function Progress({
  item,
  active,
  thin = false,
  seek,
  disabled = false,
  interaction,
}: {
  item: MediaItem;
  active: boolean;
  thin?: boolean;
  seek?: (position: number) => Promise<void>;
  disabled?: boolean;
  interaction?: (active: boolean) => void;
}) {
  const [, tick] = useState(0);
  const [preview, setPreview] = useState<number | null>(null);
  const scrub = useRef<{
    pointer: number | null;
    position: number;
    committing: boolean;
  } | null>(null);
  const interactive = !thin && active && canSeek(item) && !!seek;
  useEffect(
    () => () => {
      scrub.current = null;
      interaction?.(false);
    },
    [interaction],
  );
  useEffect(() => {
    if (!interactive && scrub.current) {
      scrub.current = null;
      setPreview(null);
      interaction?.(false);
    }
  }, [interactive, interaction]);
  const anchor = useMemo(
    () => ({ clock: Date.now(), mono: performance.now() }),
    [item],
  );
  useEffect(() => {
    if (!active || item.status !== 4 || !reliableTimeline(item)) return;
    const t = setInterval(() => tick((n) => n + 1), 1000);
    return () => clearInterval(t);
  }, [active, item]);
  if (!reliableTimeline(item)) return null;
  const elapsed = Math.max(
    0,
    (anchor.clock - item.sampledAt + performance.now() - anchor.mono) / 1000,
  );
  const pos = preview ?? progressAt(item, elapsed) ?? 0;
  const cancel = () => {
    if (!scrub.current || scrub.current.committing) return;
    scrub.current = null;
    setPreview(null);
    interaction?.(false);
  };
  const pointerPosition = (event: React.PointerEvent<HTMLDivElement>) => {
    const bounds = event.currentTarget.getBoundingClientRect();
    return seekPosition(
      event.clientX,
      bounds.left,
      bounds.width,
      item.duration!,
    );
  };
  const commit = async (position: number) => {
    const request = { pointer: null, position, committing: true };
    scrub.current = request;
    setPreview(position);
    interaction?.(true);
    try {
      await seek!(position);
    } catch (error) {
      console.warn("ORBIT media seek:", error);
    } finally {
      if (scrub.current === request) {
        scrub.current = null;
        setPreview(null);
        interaction?.(false);
      }
    }
  };
  return (
    <div
      className={thin ? "ambient-progress" : "track-progress"}
      data-running={active && item.status === 4 && preview === null}
    >
      <progress
        className="sr-only"
        aria-hidden={interactive || undefined}
        aria-label="Playback progress"
        aria-valuetext={`${clock(pos)} of ${clock(item.duration!)}`}
        max={item.duration!}
        value={pos}
      />
      <div
        className={"progress-track" + (interactive ? " seekable" : "")}
        role={interactive ? "slider" : undefined}
        tabIndex={interactive ? 0 : undefined}
        aria-hidden={!interactive || undefined}
        aria-label={interactive ? "Seek playback position" : undefined}
        aria-valuemin={interactive ? 0 : undefined}
        aria-valuemax={interactive ? item.duration! : undefined}
        aria-valuenow={interactive ? pos : undefined}
        aria-valuetext={
          interactive ? `${clock(pos)} of ${clock(item.duration!)}` : undefined
        }
        aria-disabled={
          interactive ? disabled || !!scrub.current?.committing : undefined
        }
        onPointerDown={(event) => {
          if (!interactive || disabled || scrub.current || event.button !== 0)
            return;
          const position = pointerPosition(event);
          if (position === null) return;
          event.preventDefault();
          event.currentTarget.setPointerCapture(event.pointerId);
          scrub.current = {
            pointer: event.pointerId,
            position,
            committing: false,
          };
          setPreview(position);
          interaction?.(true);
        }}
        onPointerMove={(event) => {
          if (scrub.current?.pointer !== event.pointerId) return;
          const position = pointerPosition(event);
          if (position !== null) {
            scrub.current.position = position;
            setPreview(position);
          }
        }}
        onPointerUp={(event) => {
          if (scrub.current?.pointer !== event.pointerId) return;
          if (!interactive || disabled) {
            cancel();
            return;
          }
          const position = pointerPosition(event) ?? scrub.current.position;
          // Mark committing before release: lostpointercapture must not cancel it.
          const task = commit(position);
          if (event.currentTarget.hasPointerCapture(event.pointerId))
            event.currentTarget.releasePointerCapture(event.pointerId);
          void task;
        }}
        onPointerCancel={(event) => {
          if (scrub.current?.pointer === event.pointerId) cancel();
        }}
        onLostPointerCapture={(event) => {
          if (scrub.current?.pointer === event.pointerId) cancel();
        }}
        onKeyDown={(event) => {
          if (!interactive || disabled || scrub.current) return;
          const target =
            event.key === "Home"
              ? 0
              : event.key === "End"
                ? item.duration!
                : event.key === "ArrowLeft" || event.key === "ArrowDown"
                  ? pos - 5
                  : event.key === "ArrowRight" || event.key === "ArrowUp"
                    ? pos + 5
                    : null;
          if (target === null) return;
          event.preventDefault();
          event.stopPropagation();
          void commit(Math.max(0, Math.min(item.duration!, target)));
        }}
      >
        <i
          style={{
            transform: `scaleX(${Math.max(0, Math.min(1, pos / item.duration!))})`,
          }}
        />
      </div>
      {!thin && (
        <span>
          <span>{clock(pos)}</span>
          <span>{clock(item.duration!)}</span>
        </span>
      )}
    </div>
  );
});
export function clock(seconds: number) {
  const n = Math.max(0, Math.floor(seconds));
  return `${Math.floor(n / 60)}:${String(n % 60).padStart(2, "0")}`;
}
export function PaperStack({
  count,
  active,
  animate,
  open,
  update,
}: {
  count: number;
  active: number;
  animate: boolean;
  open: () => void;
  update?: () => void;
}) {
  const reduced = useReducedMotion();
  return (
    <div className="paper-stack">
      <AnimatePresence>
        {Array.from({ length: paperCount(count) }, (_, i) => (
          <motion.button
            onUpdate={update}
            onAnimationComplete={update}
            key={i}
            className={`paper paper-${i}`}
            style={{
              left: `calc(50% + ${28 + i * 10}px)`,
              top: 8 + i * 4,
              zIndex: i,
            }}
            initial={animate && !reduced ? { y: 10, opacity: 0 } : false}
            animate={{ y: 0, opacity: 1 }}
            exit={{ y: 8, opacity: 0 }}
            transition={{ duration: animate && !reduced ? 0.16 : 0 }}
            aria-label={`Show ${active} active notes`}
            onClick={open}
          >
            <span />
          </motion.button>
        ))}
      </AnimatePresence>
    </div>
  );
}

export function artworkVisible(src: string | null, failed: string | null) {
  return !!src && src !== failed;
}
