export function OrbitMark() {
  return (
    <svg
      width="22"
      height="22"
      viewBox="0 0 24 24"
      fill="none"
      aria-hidden="true"
    >
      <ellipse
        cx="12"
        cy="12"
        rx="10"
        ry="5"
        transform="rotate(-38 12 12)"
        stroke="currentColor"
        strokeWidth="1.5"
      />
      <circle cx="12" cy="12" r="3" fill="currentColor" />
    </svg>
  );
}
export function BatteryIcon({
  percent,
  charging,
}: {
  percent: number | null;
  charging?: boolean | null;
}) {
  return (
    <svg
      width="29"
      height="18"
      viewBox="0 0 29 18"
      fill="none"
      aria-hidden="true"
    >
      <rect
        x="1"
        y="2"
        width="23"
        height="14"
        rx="4"
        stroke="currentColor"
        strokeWidth="1.5"
      />
      <path
        d="M27 6v6"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
      />
      {percent !== null && (
        <rect
          x="4"
          y="5"
          width={(17 * percent) / 100}
          height="8"
          rx="1.5"
          fill="currentColor"
        />
      )}
      {charging && (
        <path d="m14 3-5 7h4l-1 5 6-8h-5l1-4Z" fill="var(--surface)" />
      )}
    </svg>
  );
}

export function SpeakerIcon({ muted = false }: { muted?: boolean }) {
  return (
    <svg
      width="20"
      height="20"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.7"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M11 4 5 9H2v6h3l6 5Z" />
      {muted ? (
        <path d="m16 9 6 6m0-6-6 6" />
      ) : (
        <>
          <path d="M15 8a6 6 0 0 1 0 8" />
          <path d="M18 5a10 10 0 0 1 0 14" />
        </>
      )}
    </svg>
  );
}

export function ClockIcon() {
  return (
    <svg
      width="13"
      height="13"
      viewBox="0 0 20 20"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      aria-hidden="true"
    >
      <circle cx="10" cy="10" r="7.5" />
      <path d="M10 5v5l3 2" />
    </svg>
  );
}
