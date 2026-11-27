import logo from "../assets/logo.svg";
import { Card } from "../components/PageHeader";
import { CrosshairPreview, CrosshairThumb } from "../components/CrosshairPreview";
import { Kbd } from "../components/controls";
import {
  ChevronRight,
  CircleDot,
  Crosshair,
  Keyboard,
  Layers,
  Library,
  Monitor,
  Power,
  Star,
  TriangleExclamationPoint,
  Users,
  Zap,
} from "../components/icons";
import { useI18n } from "../hooks/useI18n";
import { overlayTone, useOverlayToggle } from "../hooks/useOverlayToggle";
import { useResource } from "../hooks/useResource";
import { displayParts } from "../services/accelerator";
import { api } from "../services/api";
import { store, useStore } from "../services/store";
import { toast } from "../services/toast";

export function DashboardPage() {
  const { t, lang, date } = useI18n();
  const ov = useOverlayToggle();
  const profile = useStore((s) => s.activeProfile);
  const displays = useStore((s) => s.displays);
  const dash = useResource(() => api.getDashboard());
  const d = dash.data;
  const o = ov.overlay;
  const tone = overlayTone(o);
  const monitor = displays.find((m) => m.id === o?.monitorId);
  const kind = profile?.spec.kind ?? "crosshair";

  const stateText = !o?.running
    ? t("overlay.stopped")
    : o.visible
      ? t("overlay.active")
      : o.requested
        ? t("overlay.autoHidden")
        : t("overlay.inactive");

  const switchMode = async () => {
    try {
      await api.toggleDotMode();
    } catch (e) {
      toast.error((e as Error).message);
    }
  };

  const registered = d?.shortcuts.filter((s) => s.registered).length ?? 0;

  return (
    <div className="page">
      <div className="dash-grid">
        <section className="card hero">
          <div className="hero-info">
            <div className="hero-brand">
              <img src={logo} alt="" />
              <div>
                <h2>Crosshair Studio</h2>
                <p className="muted">{t("dash.subtitle")}</p>
              </div>
            </div>

            <div className="hero-status">
              <span className="field-label">{t("dash.statusTitle")}</span>
              <div className="state">
                <span className={`status-dot ${tone}`} />
                {stateText}
                {o?.monitorName && <span className="muted">{t("overlay.onMonitor", { monitor: o.monitorName })}</span>}
              </div>
              {o && o.running && (
                <span className="faint num">
                  {t("dash.window", { w: o.width, h: o.height, renders: o.renderCount })}
                </span>
              )}
            </div>

            <button
              className={`power${ov.requested ? " on" : ""}`}
              onClick={() => ov.toggle()}
              disabled={ov.busy || !o?.running}
              aria-pressed={ov.requested}
            >
              <span className="knob">{ov.busy ? <span className="spinner" /> : <Power />}</span>
              {ov.requested ? `${t("common.on")} · ${t("overlay.turnOff")}` : `${t("common.off")} · ${t("overlay.turnOn")}`}
            </button>

            {o?.exclusiveFullscreen && (
              <div className="banner warning">
                <TriangleExclamationPoint />
                <span>{t("overlay.exclusive")}</span>
              </div>
            )}
            {o?.monitorFallback && (
              <div className="banner warning">
                <TriangleExclamationPoint />
                <span>{t("overlay.fallback")}</span>
              </div>
            )}
            {o?.lastError && (
              <div className="banner danger">
                <TriangleExclamationPoint />
                <span>{t("overlay.error", { message: o.lastError })}</span>
              </div>
            )}
          </div>

          <div className="hero-preview stack">
            <CrosshairPreview spec={profile?.spec} defaultZoom={2} label={t("dash.activeCrosshair")} />
          </div>
        </section>

        <Card title={t("dash.quick")} icon={<Zap />}>
          <div className="quick-list">
            <button className="quick-item" onClick={() => store.navigate("editor")}>
              <span className="icon">
                <Crosshair />
              </span>
              <span className="label">{t("dash.openEditor")}</span>
              <ChevronRight />
            </button>
            <button className="quick-item" onClick={() => store.navigate("dot")}>
              <span className="icon">
                <CircleDot />
              </span>
              <span className="label">{t("dash.openDot")}</span>
              <ChevronRight />
            </button>
            <button className="quick-item" onClick={() => store.navigate("library")}>
              <span className="icon">
                <Library />
              </span>
              <span className="label">{t("dash.openLibrary")}</span>
              <ChevronRight />
            </button>
            <button className="quick-item" onClick={switchMode}>
              <span className="icon">
                <Layers />
              </span>
              <span className="label">
                {t("dash.switchMode", { mode: t(kind === "dot" ? "kind.crosshair" : "kind.dot") })}
              </span>
              <ChevronRight />
            </button>
          </div>
        </Card>
      </div>

      <div className="stat-grid">
        <div className="card stat">
          <span className="label">
            <Users /> {t("dash.activeProfile")}
          </span>
          <span className="value truncate" style={{ fontSize: 19 }}>
            {profile?.name ?? "—"}
          </span>
          <span className="sub">
            {t(kind === "dot" ? "kind.dot" : "kind.crosshair")} · {t("dash.profiles")}: {d?.state.profileCount ?? "—"}
          </span>
        </div>
        <div className="card stat">
          <span className="label">
            <Library /> {t("dash.presets")}
          </span>
          <span className="value">{d?.presetCount ?? "—"}</span>
          <span className="sub">
            <Star size={11} /> {t("dash.favorites")}: {d?.favoriteCount ?? "—"}
          </span>
        </div>
        <div className="card stat">
          <span className="label">
            <Monitor /> {t("dash.monitor")}
          </span>
          <span className="value truncate" style={{ fontSize: 19 }}>
            {monitor ? `${monitor.width}×${monitor.height}` : "—"}
          </span>
          <span className="sub">
            {monitor ? `${monitor.name} · ${Math.round(monitor.scale * 100)} % · ${monitor.refreshRate} Hz` : "—"}
          </span>
        </div>
        <div className="card stat">
          <span className="label">
            <Keyboard /> {t("dash.shortcuts")}
          </span>
          <span className="value">{d ? `${registered}/${d.shortcuts.length}` : "—"}</span>
          <span className="sub">{d ? t("dash.shortcutsRegistered", { count: registered, total: d.shortcuts.length }) : ""}</span>
        </div>
      </div>

      <div className="dash-grid">
        <Card title={t("dash.recents")} icon={<Layers />}>
          {d && d.recents.length === 0 && <div className="empty">{t("dash.recentsEmpty")}</div>}
          <div className="recent-grid">
            {d?.recents.map((r) => (
              <button
                key={r.id}
                className="recent-item"
                title={`${t("common.apply")} · ${r.name}`}
                onClick={() =>
                  api
                    .applyRecent(r.id)
                    .then(() => toast.success(t("library.applied", { name: r.name })))
                    .catch((e: Error) => toast.error(e.message))
                }
              >
                <div className="thumb" style={{ height: 96 }}>
                  <CrosshairThumb spec={r.spec} size={80} />
                </div>
                <span className="name truncate">{r.name}</span>
                <span className="when">{date(r.usedAt)}</span>
              </button>
            ))}
          </div>
        </Card>

        <Card title={t("dash.shortcuts")} icon={<Keyboard />}>
          <div className="shortcut-list">
            {d?.shortcuts.map((s) => (
              <div key={s.action} className="row between">
                <span className="muted">{t(`settings.shortcut.${s.action}`)}</span>
                <span className="row">
                  {s.enabled ? (
                    <Kbd parts={displayParts(s.accelerator, lang)} />
                  ) : (
                    <span className="badge">{t("settings.shortcutDisabled")}</span>
                  )}
                  {s.enabled && !s.registered && <span className="badge badge-danger">{t("settings.shortcutFailed")}</span>}
                </span>
              </div>
            ))}
          </div>
          <p className="field-hint" style={{ marginTop: 14 }}>
            {t("dash.compat")}
          </p>
        </Card>
      </div>
    </div>
  );
}
