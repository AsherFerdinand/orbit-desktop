import type { useMedia } from "../hooks/useMedia";
import { Artwork, Progress } from "../components/Ambient";
export function AmbientMedia({
  media,
  progress,
  visible,
  interaction,
}: {
  media: ReturnType<typeof useMedia>;
  progress: boolean;
  visible: boolean;
  interaction?: (active: boolean) => void;
}) {
  const m = media.item;
  if (!m) return null;
  const playing = m.status === 4;
  return (
    <section className="now-playing" aria-label="Media playback">
      <div className="track-info">
        <Artwork src={m.artwork} paused={!playing} />
        <div>
          <small>
            {/brave/i.test(m.source) ? "BRAVE" : m.source || "WINDOWS MEDIA"} ·{" "}
            {playing ? "PLAYING" : "PAUSED"}
          </small>
          <h2 title={m.title}>{m.title || "Title not provided"}</h2>
          <p title={m.artist || m.subtitle}>
            {m.artist || m.subtitle || "Artist / channel not provided"}
          </p>
        </div>
      </div>
      {progress && (
        <Progress
          key={JSON.stringify([m.id, m.title, m.artist, m.duration])}
          item={m}
          active={visible}
          seek={media.seek}
          disabled={!media.valid || media.busy}
          interaction={interaction}
        />
      )}
      <div className="playback-row">
        <div className="transport">
          <button
            aria-label="Previous"
            disabled={!media.valid || media.busy || !m.previous}
            onClick={() => void media.act("previous")}
          >
            ⏮
          </button>
          <button
            className="play"
            aria-label={playing ? "Pause" : "Play"}
            disabled={
              !media.valid ||
              media.busy ||
              !(playing ? m.pause || m.toggle : m.play || m.toggle)
            }
            onClick={() =>
              void media.act(
                playing
                  ? m.pause
                    ? "pause"
                    : "toggle"
                  : m.play
                    ? "play"
                    : "toggle",
              )
            }
          >
            {media.busy ? "…" : playing ? "Ⅱ" : "▶"}
          </button>
          <button
            aria-label="Next"
            disabled={!media.valid || media.busy || !m.next}
            onClick={() => void media.act("next")}
          >
            ⏭
          </button>
        </div>
      </div>
      {m.error && (
        <p className="inline-error" role="status">
          {m.error}
        </p>
      )}
    </section>
  );
}
