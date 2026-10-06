import { useCallback, useEffect, useRef, useState } from "react";
import type { PointerEvent as ReactPointerEvent } from "react";
import Button from "../components/Button";
import { useLang } from "../i18n";
import { cancelRegion, getRegionSnapshot, startSession } from "../ipc";
import type { RegionSel, Snapshot } from "../ipc/types";
import { roundPx } from "../lib/format";
import "./Region.css";

const FULL_RECT: RegionSel = { x: 0, y: 0, w: 1, h: 1 };
const MIN_PX = 16;
const CORNERS = ["nw", "ne", "sw", "se"] as const;
type Corner = (typeof CORNERS)[number];
type Point = { x: number; y: number };

// Matches --space-lg in src/styles/tokens.css: the stage's own padding, which the pixel
// layout below has to subtract itself (it cannot ask the CSS custom property for its
// value without a getComputedStyle round-trip that jsdom does not reliably support).
const STAGE_PADDING_PX = 16;

function clamp(v: number, lo: number, hi: number): number {
  return Math.min(hi, Math.max(lo, v));
}

function normalize(a: Point, b: Point): RegionSel {
  return { x: Math.min(a.x, b.x), y: Math.min(a.y, b.y), w: Math.abs(a.x - b.x), h: Math.abs(a.y - b.y) };
}

function enforceMin(sel: RegionSel, pxW: number, pxH: number): RegionSel {
  const minWFrac = pxW > 0 ? Math.min(1, MIN_PX / pxW) : 0;
  const minHFrac = pxH > 0 ? Math.min(1, MIN_PX / pxH) : 0;
  let { x, y, w, h } = sel;
  if (w < minWFrac) {
    const grow = (minWFrac - w) / 2;
    x = clamp(x - grow, 0, 1 - minWFrac);
    w = minWFrac;
  }
  if (h < minHFrac) {
    const grow = (minHFrac - h) / 2;
    y = clamp(y - grow, 0, 1 - minHFrac);
    h = minHFrac;
  }
  return { x, y, w, h };
}

function oppositeCorner(rect: RegionSel, corner: Corner): Point {
  const x = corner === "nw" || corner === "sw" ? rect.x + rect.w : rect.x;
  const y = corner === "nw" || corner === "ne" ? rect.y + rect.h : rect.y;
  return { x, y };
}

function pointFromClient(clientX: number, clientY: number, frame: HTMLElement): Point {
  const r = frame.getBoundingClientRect();
  if (r.width === 0 || r.height === 0) return { x: 0, y: 0 };
  return { x: clamp((clientX - r.left) / r.width, 0, 1), y: clamp((clientY - r.top) / r.height, 0, 1) };
}

export default function Region() {
  const { t } = useLang();
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [rect, setRect] = useState<RegionSel | null>(null);
  const [frameSize, setFrameSize] = useState({ width: 0, height: 0 });
  const stageRef = useRef<HTMLDivElement>(null);
  const frameRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    getRegionSnapshot().then(setSnapshot);
  }, []);

  // Sizes the frame to the largest box that fits the stage while keeping the captured
  // window's real aspect ratio -- computed in JS (rather than pure CSS) so the overlay
  // (dim bands, selection rect, handles) can be positioned with the same plain pixel
  // numbers, with no percentage/containing-block ambiguity.
  useEffect(() => {
    const snap = snapshot;
    const stage = stageRef.current;
    if (!snap || !stage) return;

    function compute() {
      // Re-checked here (not just above): TS does not carry a narrowing from the
      // effect body into a callback that can run later (ResizeObserver/resize), even
      // for a `const`.
      if (!stage || !snap) return;
      const availW = Math.max(1, stage.clientWidth - STAGE_PADDING_PX * 2);
      const availH = Math.max(1, stage.clientHeight - STAGE_PADDING_PX * 2);
      const ratio = snap.width / snap.height;
      let w = availW;
      let h = w / ratio;
      if (h > availH) {
        h = availH;
        w = h * ratio;
      }
      setFrameSize({ width: Math.round(w), height: Math.round(h) });
    }

    compute();
    if (typeof ResizeObserver !== "undefined") {
      const ro = new ResizeObserver(compute);
      ro.observe(stage);
      return () => ro.disconnect();
    }
    window.addEventListener("resize", compute);
    return () => window.removeEventListener("resize", compute);
  }, [snapshot]);

  const handleStart = useCallback(() => {
    if (!snapshot) return;
    void startSession(snapshot.windowId, rect ?? FULL_RECT);
  }, [snapshot, rect]);

  const handleCancel = useCallback(() => {
    void cancelRegion();
  }, []);

  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") {
        e.preventDefault();
        handleCancel();
      } else if (e.key === "Enter") {
        e.preventDefault();
        handleStart();
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [handleStart, handleCancel]);

  function beginDrag(onMovePoint: (p: Point) => void) {
    function onMove(e: PointerEvent) {
      const frame = frameRef.current;
      if (!frame) return;
      onMovePoint(pointFromClient(e.clientX, e.clientY, frame));
    }
    function onUp() {
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
    }
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
  }

  function handleFrameDown(e: ReactPointerEvent<HTMLDivElement>) {
    const snap = snapshot;
    const frame = frameRef.current;
    if (!snap || !frame) return;
    e.preventDefault();
    const anchor = pointFromClient(e.clientX, e.clientY, frame);
    beginDrag((p) => {
      setRect(enforceMin(normalize(anchor, p), snap.width, snap.height));
    });
  }

  function handleHandleDown(e: ReactPointerEvent<HTMLDivElement>, corner: Corner) {
    const snap = snapshot;
    const frame = frameRef.current;
    if (!snap || !frame) return;
    e.preventDefault();
    e.stopPropagation();
    const anchor = oppositeCorner(rect ?? FULL_RECT, corner);
    beginDrag((p) => {
      setRect(enforceMin(normalize(anchor, p), snap.width, snap.height));
    });
  }

  if (!snapshot) return null;

  const effectiveRect = rect ?? FULL_RECT;
  const pxW = roundPx(effectiveRect.w, snapshot.width);
  const pxH = roundPx(effectiveRect.h, snapshot.height);
  const fw = frameSize.width;
  const fh = frameSize.height;

  return (
    <div className="region" data-testid="region-root">
      <div ref={stageRef} className="region-stage">
        {fw > 0 && fh > 0 && (
          <div
            ref={frameRef}
            className="region-frame"
            style={{ width: fw, height: fh }}
            onPointerDown={handleFrameDown}
          >
            <img
              className="region-image"
              src={snapshot.image}
              alt={t("region.imageAlt", { title: snapshot.title })}
              draggable={false}
            />
            <div className="region-dim region-dim-top" style={{ height: effectiveRect.y * fh }} />
            <div className="region-dim region-dim-bottom" style={{ top: (effectiveRect.y + effectiveRect.h) * fh }} />
            <div
              className="region-dim region-dim-left"
              style={{ top: effectiveRect.y * fh, height: effectiveRect.h * fh, width: effectiveRect.x * fw }}
            />
            <div
              className="region-dim region-dim-right"
              style={{ top: effectiveRect.y * fh, height: effectiveRect.h * fh, left: (effectiveRect.x + effectiveRect.w) * fw }}
            />
            <div
              className="region-rect"
              style={{ left: effectiveRect.x * fw, top: effectiveRect.y * fh, width: effectiveRect.w * fw, height: effectiveRect.h * fh }}
            >
              <span className="region-size tabular-nums">{t("region.size", { w: pxW, h: pxH })}</span>
              {CORNERS.map((corner) => (
                <div
                  key={corner}
                  className={`region-handle region-handle-${corner}`}
                  onPointerDown={(e) => handleHandleDown(e, corner)}
                />
              ))}
            </div>
          </div>
        )}
      </div>
      <div className="region-controls">
        <p className="region-hint">{t("region.hint")}</p>
        <div className="region-buttons">
          <Button kind="secondary" onClick={handleCancel}>
            {t("action.cancel")}
          </Button>
          <Button kind="primary" onClick={handleStart}>
            {t("action.start")}
          </Button>
        </div>
      </div>
    </div>
  );
}
