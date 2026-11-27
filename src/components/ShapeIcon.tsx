import type { CrosshairShape } from "../types/models";

/** Schematic icon of each crosshair shape (32×32, uses currentColor). */
export function ShapeIcon({ shape }: { shape: CrosshairShape }) {
  const arms = (gap: number, len = 9) => (
    <>
      <path d={`M16 ${16 - gap - len}V${16 - gap}`} />
      <path d={`M16 ${16 + gap}V${16 + gap + len}`} />
      <path d={`M${16 - gap - len} 16H${16 - gap}`} />
      <path d={`M${16 + gap} 16H${16 + gap + len}`} />
    </>
  );
  const dot = <circle cx="16" cy="16" r="1.8" fill="currentColor" stroke="none" />;
  let content;
  switch (shape) {
    case "cross":
      content = (
        <>
          <path d="M16 5V27" />
          <path d="M5 16H27" />
        </>
      );
      break;
    case "cross_gap":
      content = arms(4);
      break;
    case "cross_dynamic":
      content = (
        <>
          {arms(3, 6)}
          <g opacity="0.5">
            <path d="M16 2V5" />
            <path d="M16 27V30" />
            <path d="M2 16H5" />
            <path d="M27 16H30" />
          </g>
        </>
      );
      break;
    case "circle":
      content = <circle cx="16" cy="16" r="9" />;
      break;
    case "circle_dot":
      content = (
        <>
          <circle cx="16" cy="16" r="9" />
          {dot}
        </>
      );
      break;
    case "dot":
      content = <circle cx="16" cy="16" r="3.2" fill="currentColor" stroke="none" />;
      break;
    case "cross_dot":
      content = (
        <>
          {arms(4.5, 8)}
          {dot}
        </>
      );
      break;
    case "custom":
      content = (
        <>
          <path d="M16 20V28" />
          <path d="M4 16H12" />
          <path d="M20 16H28" />
          <circle cx="16" cy="16" r="11" strokeDasharray="3 3" opacity="0.6" />
          {dot}
        </>
      );
      break;
  }
  return (
    <svg viewBox="0 0 32 32" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" aria-hidden="true">
      {content}
    </svg>
  );
}
