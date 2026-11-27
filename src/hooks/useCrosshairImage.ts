import { useEffect, useRef, useState } from "react";
import { renderBitmap, type CrosshairBitmap } from "../services/preview";
import type { CrosshairSpec } from "../types/models";

/**
 * Returns the bitmap rendered by the backend for `spec`. While a new render
 * is in flight the previous image stays displayed (no flicker), and only the
 * most recent request is kept.
 */
export function useCrosshairImage(spec: CrosshairSpec | null | undefined, scale = 1): CrosshairBitmap | null {
  const [image, setImage] = useState<CrosshairBitmap | null>(null);
  const seq = useRef(0);
  const key = spec ? JSON.stringify(spec) : "";

  useEffect(() => {
    if (!spec) {
      setImage(null);
      return;
    }
    const id = ++seq.current;
    renderBitmap(spec, scale)
      .then((img) => {
        if (id === seq.current) setImage(img);
      })
      .catch(() => {
        /* invalid intermediate values are simply not rendered */
      });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key, scale]);

  return image;
}
