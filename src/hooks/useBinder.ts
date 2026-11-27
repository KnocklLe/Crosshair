import { useCallback, useRef } from "react";

/**
 * Returns `bind(path)` → a *stable* `(value) => void` that calls
 * `apply(patchObjectFor(path, value))`. Stable callbacks let memoized
 * controls skip re-rendering when another field changes.
 *
 * `bind("outline.thickness")(2)` → `apply({ outline: { thickness: 2 } })`.
 */
export function useBinder<P>(apply: (patch: P) => void) {
  const applyRef = useRef(apply);
  applyRef.current = apply;
  const cache = useRef(new Map<string, (v: unknown) => void>());

  return useCallback(<V,>(path: string): ((v: V) => void) => {
    let fn = cache.current.get(path);
    if (!fn) {
      fn = (value: unknown) => {
        const keys = path.split(".");
        const patch: Record<string, unknown> = {};
        let cursor = patch;
        keys.forEach((k, i) => {
          if (i === keys.length - 1) cursor[k] = value;
          else cursor = cursor[k] = {} as Record<string, unknown>;
        });
        applyRef.current(patch as P);
      };
      cache.current.set(path, fn);
    }
    return fn as (v: V) => void;
  }, []);
}
