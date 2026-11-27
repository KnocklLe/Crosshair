// Pixel-exact preview: the bitmap comes from the Rust renderer (same pixels as
// the overlay) and is drawn with nearest-neighbour scaling, one crosshair pixel
// = `zoom` physical screen pixels.

import { memo, useEffect, useRef, useState } from "react";
import { useCrosshairImage } from "../hooks/useCrosshairImage";
import { useI18n } from "../hooks/useI18n";
import type { CrosshairSpec } from "../types/models";
import { Segmented } from "./controls";

export type PreviewBackground = "dark" | "scene" | "light" | "checker";

const BG_KEY = "cs.preview.bg";
const ZOOM_KEY = "cs.preview.zoom";

function readPref<T extends string | number>(key: string, fallback: T, allowed: readonly T[]): T {
  try {
    const raw = localStorage.getItem(key);
    const value = (typeof fallback === "number" ? Number(raw) : raw) as T;
    return allowed.includes(value) ? value : fallback;
  } catch {
    return fallback;
  }
}

function writePref(key: string, value: string | number) {
  try {
    localStorage.setItem(key, String(value));
  } catch {
    /* preferences are optional */
  }
}

function drawBackground(ctx: CanvasRenderingContext2D, w: number, h: number, bg: PreviewBackground, dpr: number) {
  switch (bg) {
    case "light":
      ctx.fillStyle = "#d8d8e0";
      ctx.fillRect(0, 0, w, h);
      break;
    case "checker": {
      const s = Math.round(10 * dpr);
      for (let y = 0; y < h; y += s)
        for (let x = 0; x < w; x += s) {
          ctx.fillStyle = (x / s + y / s) % 2 === 0 ? "#2b2b33" : "#1d1d23";
          ctx.fillRect(x, y, s, s);
        }
      break;
    }
    case "scene": {
      const horizon = Math.round(h * 0.58);
      const sky = ctx.createLinearGradient(0, 0, 0, horizon);
      sky.addColorStop(0, "#4f6f96");
      sky.addColorStop(1, "#b8cad9");
      ctx.fillStyle = sky;
      ctx.fillRect(0, 0, w, horizon);
      // distant hills
      ctx.fillStyle = "#6f7f86";
      ctx.beginPath();
      ctx.moveTo(0, horizon);
      for (let i = 0; i <= 8; i++) ctx.lineTo((w / 8) * i, horizon - (Math.sin(i * 1.7) * 0.5 + 0.7) * h * 0.09);
      ctx.lineTo(w, horizon);
      ctx.fill();
      // buildings
      const blocks = [0.06, 0.17, 0.3, 0.62, 0.74, 0.86];
      blocks.forEach((x, i) => {
        const bw = w * (0.07 + (i % 3) * 0.02);
        const bh = h * (0.12 + ((i * 7) % 5) * 0.035);
        ctx.fillStyle = i % 2 ? "#4b4e4f" : "#5d5a54";
        ctx.fillRect(w * x, horizon - bh, bw, bh);
      });
      const ground = ctx.createLinearGradient(0, horizon, 0, h);
      ground.addColorStop(0, "#5b5a4c");
      ground.addColorStop(1, "#2a2a24");
      ctx.fillStyle = ground;
      ctx.fillRect(0, horizon, w, h - horizon);
      break;
    }
    default: {
      ctx.fillStyle = "#0e0e13";
      ctx.fillRect(0, 0, w, h);
      ctx.strokeStyle = "rgba(255,255,255,0.035)";
      ctx.lineWidth = 1;
      const step = Math.round(32 * dpr);
      const cx = Math.floor(w / 2);
      const cy = Math.floor(h / 2);
      ctx.beginPath();
      for (let x = cx % step; x < w; x += step) {
        ctx.moveTo(x + 0.5, 0);
        ctx.lineTo(x + 0.5, h);
      }
      for (let y = cy % step; y < h; y += step) {
        ctx.moveTo(0, y + 0.5);
        ctx.lineTo(w, y + 0.5);
      }
      ctx.stroke();
    }
  }
}

const ZOOMS = [1, 2, 4, 8] as const;
const BGS = ["dark", "scene", "light", "checker"] as const;

export const CrosshairPreview = memo(function CrosshairPreview(props: {
  spec: CrosshairSpec | null | undefined;
  toolbar?: boolean;
  defaultZoom?: number;
  label?: string;
}) {
  const { t } = useI18n();
  const wrap = useRef<HTMLDivElement>(null);
  const canvas = useRef<HTMLCanvasElement>(null);
  const [size, setSize] = useState({ w: 0, h: 0, dpr: 1 });
  const [bg, setBg] = useState<PreviewBackground>(() => readPref(BG_KEY, "dark", BGS));
  const [zoom, setZoom] = useState<number>(() =>
    props.toolbar ? readPref(ZOOM_KEY, props.defaultZoom ?? 2, ZOOMS) : (props.defaultZoom ?? 2),
  );
  const image = useCrosshairImage(props.spec);

  useEffect(() => {
    const el = wrap.current;
    if (!el) return;
    const ro = new ResizeObserver(([entry]) => {
      const r = entry.contentRect;
      setSize({ w: r.width, h: r.height, dpr: window.devicePixelRatio || 1 });
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  useEffect(() => {
    const c = canvas.current;
    if (!c || size.w === 0) return;
    const W = Math.round(size.w * size.dpr);
    const H = Math.round(size.h * size.dpr);
    if (c.width !== W || c.height !== H) {
      c.width = W;
      c.height = H;
    }
    const ctx = c.getContext("2d");
    if (!ctx) return;
    ctx.imageSmoothingEnabled = true;
    drawBackground(ctx, W, H, bg, size.dpr);
    if (image) {
      ctx.imageSmoothingEnabled = false;
      const dw = image.width * zoom;
      const dh = image.height * zoom;
      ctx.drawImage(image.bitmap, Math.floor(W / 2) - dw / 2, Math.floor(H / 2) - dh / 2, dw, dh);
    }
  }, [image, size, bg, zoom]);

  return (
    <div className="preview" ref={wrap}>
      <canvas ref={canvas} aria-label={props.label ?? t("preview.exact")} role="img" />
      {props.label && <span className="preview-tag">{props.label}</span>}
      {props.toolbar && (
        <div className="preview-toolbar">
          <Segmented
            label={t("preview.background")}
            value={bg}
            onChange={(v) => {
              setBg(v);
              writePref(BG_KEY, v);
            }}
            options={BGS.map((b) => ({ value: b, label: t(`preview.bg.${b}`) }))}
          />
          <Segmented
            label={t("preview.zoom")}
            value={zoom}
            onChange={(v) => {
              setZoom(v);
              writePref(ZOOM_KEY, v);
            }}
            options={ZOOMS.map((z) => ({ value: z, label: z === 1 ? "1:1" : `${z}×` }))}
          />
        </div>
      )}
    </div>
  );
});

/** Small thumbnail (library, profiles, history). */
export const CrosshairThumb = memo(function CrosshairThumb({ spec, size = 96 }: { spec: CrosshairSpec; size?: number }) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const image = useCrosshairImage(spec);

  useEffect(() => {
    const c = canvas.current;
    if (!c) return;
    const dpr = window.devicePixelRatio || 1;
    const S = Math.round(size * dpr);
    c.width = S;
    c.height = S;
    const ctx = c.getContext("2d");
    if (!ctx) return;
    ctx.clearRect(0, 0, S, S);
    if (!image) return;
    const fit = (S * 0.82) / image.width;
    // Small crosshairs are enlarged by a whole factor (stays sharp); large ones are reduced.
    const z = fit >= 1 ? Math.max(1, Math.min(3, Math.floor(fit))) : fit;
    ctx.imageSmoothingEnabled = z < 1;
    const d = image.width * z;
    const x = Math.floor(S / 2) - Math.round(d / 2);
    ctx.drawImage(image.bitmap, x, x, d, d);
  }, [image, size]);

  return <canvas ref={canvas} style={{ width: size, height: size }} aria-hidden="true" />;
});
