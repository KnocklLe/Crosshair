// Tiny toast queue (no dependency).

export type ToastKind = "info" | "success" | "warning" | "error";

export interface ToastItem {
  id: number;
  kind: ToastKind;
  message: string;
}

type Listener = (items: ToastItem[]) => void;

let items: ToastItem[] = [];
let nextId = 1;
const listeners = new Set<Listener>();

function emit() {
  listeners.forEach((l) => l(items));
}

function push(kind: ToastKind, message: string, duration = 3500) {
  const id = nextId++;
  // Avoid stacking the same message twice.
  items = [...items.filter((t) => t.message !== message), { id, kind, message }].slice(-4);
  emit();
  window.setTimeout(() => dismiss(id), duration);
}

export function dismiss(id: number) {
  items = items.filter((t) => t.id !== id);
  emit();
}

export function subscribeToasts(l: Listener) {
  listeners.add(l);
  l(items);
  return () => {
    listeners.delete(l);
  };
}

export const toast = {
  info: (m: string, d?: number) => push("info", m, d),
  success: (m: string, d?: number) => push("success", m, d),
  warning: (m: string, d?: number) => push("warning", m, d ?? 6000),
  error: (m: string, d?: number) => push("error", m, d ?? 7000),
};
