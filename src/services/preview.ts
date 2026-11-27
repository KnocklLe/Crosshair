// Crosshair images rendered by the Rust renderer (the same pixels as the
// overlay), cached as ImageBitmaps keyed by the spec JSON.

import { api } from "./api";
import type { CrosshairSpec } from "../types/models";

export interface CrosshairBitmap {
  bitmap: ImageBitmap;
  width: number;
  height: number;
}

const MAX_ENTRIES = 160;
const cache = new Map<string, Promise<CrosshairBitmap>>();

export function specKey(spec: CrosshairSpec, scale = 1): string {
  return `${scale}|${JSON.stringify(spec)}`;
}

export function renderBitmap(spec: CrosshairSpec, scale = 1): Promise<CrosshairBitmap> {
  const key = specKey(spec, scale);
  const hit = cache.get(key);
  if (hit) {
    // LRU: move to the end.
    cache.delete(key);
    cache.set(key, hit);
    return hit;
  }
  const promise = api.renderPreview(spec, scale).then(async (img) => ({
    bitmap: await createImageBitmap(new ImageData(img.pixels, img.width, img.height)),
    width: img.width,
    height: img.height,
  }));
  promise.catch(() => cache.delete(key));
  cache.set(key, promise);
  // Evicted bitmaps are not closed: a component may still redraw one; the
  // garbage collector releases them once unreferenced.
  while (cache.size > MAX_ENTRIES) {
    cache.delete(cache.keys().next().value as string);
  }
  return promise;
}
