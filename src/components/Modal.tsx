import { useEffect, useRef, type ReactNode } from "react";
import { X } from "./icons";
import { useI18n } from "../hooks/useI18n";

export function Modal(props: {
  title: string;
  onClose: () => void;
  children: ReactNode;
  footer?: ReactNode;
  width?: number;
}) {
  const { t } = useI18n();
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && props.onClose();
    window.addEventListener("keydown", onKey);
    // Focus the first field (or the dialog) when opened.
    const first = ref.current?.querySelector<HTMLElement>("input, textarea, select, button.btn-primary");
    (first ?? ref.current)?.focus();
    return () => window.removeEventListener("keydown", onKey);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <div className="modal-backdrop" onMouseDown={(e) => e.target === e.currentTarget && props.onClose()}>
      <div
        ref={ref}
        className="modal"
        role="dialog"
        aria-modal="true"
        aria-label={props.title}
        tabIndex={-1}
        style={props.width ? { width: `min(${props.width}px, 100%)` } : undefined}
      >
        <div className="modal-header">
          <h2>{props.title}</h2>
          <button className="icon-btn" onClick={props.onClose} aria-label={t("common.close")}>
            <X />
          </button>
        </div>
        <div className="modal-body">{props.children}</div>
        {props.footer && <div className="modal-footer">{props.footer}</div>}
      </div>
    </div>
  );
}

export function ConfirmDialog(props: {
  title: string;
  body: ReactNode;
  confirmLabel: string;
  danger?: boolean;
  busy?: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}) {
  const { t } = useI18n();
  return (
    <Modal
      title={props.title}
      onClose={props.onCancel}
      footer={
        <>
          <button className="btn" onClick={props.onCancel}>
            {t("common.cancel")}
          </button>
          <button
            className={`btn ${props.danger ? "btn-danger-solid" : "btn-primary"}`}
            onClick={props.onConfirm}
            disabled={props.busy}
          >
            {props.busy && <span className="spinner" />}
            {props.confirmLabel}
          </button>
        </>
      }
    >
      <p className="muted">{props.body}</p>
    </Modal>
  );
}
