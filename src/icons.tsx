/** Line icons drawn as 24 × 24 strokes in the text colour. They are decorative, so screen readers skip them. */
export const PATHS = {
  // Settings tabs
  general: "M4 7h9M17 7h3M4 17h3M11 17h9M15 5v4M9 15v4",
  models: "M7 7h10v10H7zM10 10h4v4h-4zM9 3v4M15 3v4M9 17v4M15 17v4M3 9h4M3 15h4M17 9h4M17 15h4",
  personality: "M12 3a9 9 0 1 0 0 18a9 9 0 1 0 0-18M8.5 14.5a4.5 4.5 0 0 0 7 0M9 9.5h.01M15 9.5h.01",
  skills: "M5 4h14a1 1 0 0 1 1 1v14a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V5a1 1 0 0 1 1-1zM14 8l-4 8",
  voice: "M12 3a3 3 0 0 1 3 3v5a3 3 0 0 1-6 0V6a3 3 0 0 1 3-3zM5 11a7 7 0 0 0 14 0M12 18v3",
  privacy: "M12 3l7 3v5c0 4.5-3 8-7 10c-4-2-7-5.5-7-10V6z",
  history: "M12 3a9 9 0 1 0 0 18a9 9 0 1 0 0-18M12 7v5l3 2",
  about: "M12 3a9 9 0 1 0 0 18a9 9 0 1 0 0-18M12 11v5M12 8h.01",
  // The bubble
  expand: "M14 4h6v6M20 4l-7 7M10 20H4v-6M4 20l7-7",
  folder: "M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z",
  link: "M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7l-1 1M14 10a4 4 0 0 0-5.7 0l-3 3a4 4 0 0 0 5.7 5.7l1-1",
  image: "M5 4h14a1 1 0 0 1 1 1v14a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V5a1 1 0 0 1 1-1zM4 16l5-5 4 4 2-2 5 5M15 9h.01",
  file: "M6 3h8l4 4v13a1 1 0 0 1-1 1H6a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1zM14 3v4h4",
  close: "M7 7l10 10M17 7L7 17",
  chevron: "M10 7l5 5-5 5",
  // An answer's actions (DSN-2.2)
  copy: "M10 9h9a1 1 0 0 1 1 1v9a1 1 0 0 1-1 1h-9a1 1 0 0 1-1-1v-9a1 1 0 0 1 1-1zM15 9V5a1 1 0 0 0-1-1H5a1 1 0 0 0-1 1v9a1 1 0 0 0 1 1h4",
  pin: "M9 4h6M10 4v6l-3 4h10l-3-4V4M12 14v6",
  save: "M12 4v11M7 10l5 5 5-5M5 20h14",
  regenerate: "M20 11a8 8 0 1 0-2.3 5.7M20 5v6h-6",
  // The input (DSN-2.3)
  send: "M12 19V5M6 11l6-6 6 6",
  stop: "M8 8h8v8H8z",
  // Setup
  check: "M5 12.5l4.5 4.5L19 7.5",
};

export function Icon({ d, size = 18, stroke = 1.8 }: { d: string; size?: number; stroke?: number }) {
  return (
    <svg viewBox="0 0 24 24" width={size} height={size} fill="none" stroke="currentColor" strokeWidth={stroke} strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d={d} />
    </svg>
  );
}
