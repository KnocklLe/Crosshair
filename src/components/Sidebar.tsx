import logo from "../assets/logo.svg";
import { useI18n } from "../hooks/useI18n";
import { overlayTone, useOverlayToggle } from "../hooks/useOverlayToggle";
import { store, useStore } from "../services/store";
import type { PageId } from "../types/models";
import type { TranslationKey } from "../i18n";
import { Toggle } from "./controls";
import { CircleDot, Crosshair, Info, LayoutDashboard, Library, Settings, Users } from "./icons";

const ITEMS: { id: PageId; label: TranslationKey; icon: typeof Crosshair }[] = [
  { id: "dashboard", label: "nav.dashboard", icon: LayoutDashboard },
  { id: "editor", label: "nav.editor", icon: Crosshair },
  { id: "dot", label: "nav.dot", icon: CircleDot },
  { id: "library", label: "nav.library", icon: Library },
  { id: "profiles", label: "nav.profiles", icon: Users },
  { id: "settings", label: "nav.settings", icon: Settings },
  { id: "about", label: "nav.about", icon: Info },
];

export function Sidebar() {
  const { t } = useI18n();
  const page = useStore((s) => s.page);
  const version = useStore((s) => s.app?.version);
  const profileName = useStore((s) => s.activeProfile?.name);
  const ov = useOverlayToggle();
  const tone = overlayTone(ov.overlay);
  const stateLabel = !ov.overlay?.running
    ? t("overlay.stopped")
    : ov.visible
      ? t("overlay.active")
      : ov.requested
        ? t("overlay.autoHidden")
        : t("overlay.inactive");

  return (
    <aside className="sidebar">
      <div className="brand">
        <img src={logo} alt="" />
        <div>
          <div className="name">Crosshair Studio</div>
          <div className="version">v{version}</div>
        </div>
      </div>
      <nav className="nav">
        {ITEMS.map(({ id, label, icon: Icon }) => (
          <button
            key={id}
            className="nav-item"
            aria-current={page === id ? "page" : undefined}
            title={t(label)}
            onClick={() => store.navigate(id)}
          >
            <Icon />
            <span>{t(label)}</span>
          </button>
        ))}
      </nav>
      <div className="nav-spacer" />
      <div className="overlay-mini">
        <div className="row">
          <span className={`status-dot ${tone}`} />
          <div className="text grow">
            <div className="label truncate">{stateLabel}</div>
          </div>
          <Toggle checked={ov.requested} onChange={() => ov.toggle()} disabled={ov.busy || !ov.overlay?.running} label={t("nav.overlay")} />
        </div>
        <div className="sub truncate">{profileName}</div>
      </div>
    </aside>
  );
}
