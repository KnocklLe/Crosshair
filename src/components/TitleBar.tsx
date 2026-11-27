import { getCurrentWindow } from "@tauri-apps/api/window";
import logo from "../assets/logo.svg";
import { useI18n } from "../hooks/useI18n";
import { Maximize2, Minus, X } from "./icons";

// Resolved lazily: the window handle is only needed when a button is used.
const win = () => getCurrentWindow();

/** Custom title bar (the window has no native decorations). */
export function TitleBar() {
  const { t } = useI18n();
  return (
    <header className="titlebar" data-tauri-drag-region onDoubleClick={() => win().toggleMaximize()}>
      <div className="titlebar-brand" data-tauri-drag-region>
        <img src={logo} alt="" />
        <span>Crosshair Studio</span>
      </div>
      <div className="window-controls" onDoubleClick={(e) => e.stopPropagation()}>
        <button onClick={() => win().minimize()} aria-label={t("window.minimize")} title={t("window.minimize")}>
          <Minus />
        </button>
        <button onClick={() => win().toggleMaximize()} aria-label={t("window.maximize")} title={t("window.maximize")}>
          <Maximize2 />
        </button>
        <button className="close" onClick={() => win().close()} aria-label={t("window.close")} title={t("window.close")}>
          <X />
        </button>
      </div>
    </header>
  );
}
