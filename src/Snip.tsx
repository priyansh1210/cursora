import { useEffect, useRef, useState, type MouseEvent } from "react";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";

type Point = { x: number; y: number };
type Box = { x: number; y: number; w: number; h: number };

/** The snip overlay (VIS-2.1): the frozen screen, dimmed. Drag a rectangle; Esc or a right-click cancels. */
export default function Snip() {
  // A new address each snip, so a cached grab from an earlier one can never show.
  const [src] = useState(() => `${convertFileSrc("screen", "snip")}?${Date.now()}`);
  const [from, setFrom] = useState<Point | null>(null);
  const [to, setTo] = useState<Point | null>(null);
  const sent = useRef(false);

  // Under 8×8 px counts as a cancel; Rust decides, in screen pixels (VIS-2.3).
  function finish(rect: Box | null) {
    if (sent.current) return;
    sent.current = true;
    void invoke("snip_done", { rect });
  }

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") finish(null);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const at = (e: MouseEvent): Point => ({ x: e.clientX, y: e.clientY });
  const box = from && to ? { x: Math.min(from.x, to.x), y: Math.min(from.y, to.y), w: Math.abs(to.x - from.x), h: Math.abs(to.y - from.y) } : null;
  const px = (v: number) => Math.round(v * window.devicePixelRatio);

  return (
    <div
      className="snip"
      role="application"
      aria-label="Drag to pick a part of the screen. Escape cancels."
      onMouseDown={(e) => {
        if (e.button !== 0) return;
        setFrom(at(e));
        setTo(at(e));
      }}
      onMouseMove={(e) => from && setTo(at(e))}
      onMouseUp={(e) => e.button === 0 && box && finish(box)}
      onContextMenu={(e) => {
        e.preventDefault();
        finish(null);
      }}
    >
      <img src={src} alt="" draggable={false} />
      {box ? (
        <div className="pick" style={{ left: box.x, top: box.y, width: box.w, height: box.h }}>
          <span>
            {px(box.w)} × {px(box.h)}
          </span>
        </div>
      ) : (
        <div className="shade" />
      )}
    </div>
  );
}
