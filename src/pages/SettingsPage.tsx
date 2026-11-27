import { useEffect, useRef, useState, type ReactNode } from "react";
import { Card, PageHeader } from "../components/PageHeader";
import { Segmented, SliderField, Toggle } from "../components/controls";
import {
  Cpu,
  Crosshair,
  Database,
  Download,
  FolderOpen,
  Gauge,
  Keyboard,
  Languages,
  Monitor,
  RotateCcw,
  Settings as SettingsIcon,
  TriangleExclamationPoint,
  Upload,
} from "../components/icons";
import { ConfirmDialog } from "../components/Modal";
import { ShortcutRecorder } from "../components/ShortcutRecorder";
import { useI18n } from "../hooks/useI18n";
import { api } from "../services/api";
import { formatBytes, formatDuration, formatMicros } from "../services/format";
import { store, useStore } from "../services/store";
import { toast } from "../services/toast";
import type { PerformanceInfo, Settings, ShortcutAction } from "../types/models";
import type { TranslationKey } from "../i18n";

type SectionId = "general" | "overlay" | "monitors" | "shortcuts" | "performance" | "data";

const SECTIONS: { id: SectionId; label: TranslationKey; icon: typeof Monitor }[] = [
  { id: "general", label: "settings.general", icon: SettingsIcon },
  { id: "overlay", label: "settings.overlay", icon: Crosshair },
  { id: "monitors", label: "settings.monitors", icon: Monitor },
  { id: "shortcuts", label: "settings.shortcuts", icon: Keyboard },
  { id: "performance", label: "settings.performance", icon: Gauge },
  { id: "data", label: "settings.data", icon: Database },
];

/** Sends the new settings and keeps only what the backend actually applied. */
async function saveSettings(change: (s: Settings) => Settings): Promise<boolean> {
  const current = store.get().settings;
  if (!current) return false;
  try {
    const applied = await api.updateSettings(change(structuredClone(current)));
    store.set({ settings: applied });
    return true;
  } catch (e) {
    toast.error((e as Error).message);
    return false;
  }
}

function Setting(props: { title: string; hint?: string; children: ReactNode }) {
  return (
    <div className="setting">
      <div className="text">
        <span className="title">{props.title}</span>
        {props.hint && <span className="hint">{props.hint}</span>}
      </div>
      <div className="control">{props.children}</div>
    </div>
  );
}

export function SettingsPage() {
  const { t } = useI18n();
  const [section, setSection] = useState<SectionId>("general");
  const settings = useStore((s) => s.settings);
  if (!settings) return null;

  return (
    <div className="page">
      <PageHeader title={t("nav.settings")} subtitle={t("settings.subtitle")} />
      <div className="settings-layout">
        <nav className="settings-nav">
          {SECTIONS.map(({ id, label, icon: Icon }) => (
            <button key={id} aria-current={section === id} onClick={() => setSection(id)}>
              <Icon /> {t(label)}
            </button>
          ))}
        </nav>
        <div className="settings-content">
          {section === "general" && <GeneralSection settings={settings} />}
          {section === "overlay" && <OverlaySection settings={settings} />}
          {section === "monitors" && <MonitorsSection settings={settings} />}
          {section === "shortcuts" && <ShortcutsSection />}
          {section === "performance" && <PerformanceSection settings={settings} />}
          {section === "data" && <DataSection />}
        </div>
      </div>
    </div>
  );
}

function GeneralSection({ settings }: { settings: Settings }) {
  const { t } = useI18n();
  return (
    <Card title={t("settings.general")} icon={<Languages />}>
      <div className="setting-list">
        <Setting title={t("settings.language")}>
          <select
            className="select"
            value={settings.language}
            onChange={(e) => saveSettings((s) => ({ ...s, language: e.target.value as Settings["language"] }))}
          >
            <option value="auto">{t("settings.lang.auto")}</option>
            <option value="en">English</option>
            <option value="fr">Français</option>
          </select>
        </Setting>
        <Setting title={t("settings.theme")}>
          <Segmented
            value={settings.theme}
            onChange={(theme) => saveSettings((s) => ({ ...s, theme }))}
            options={(["dark", "light", "system"] as const).map((v) => ({ value: v, label: t(`settings.theme.${v}`) }))}
          />
        </Setting>
        <Setting title={t("settings.startup")} hint={t("settings.startupHint")}>
          <Toggle
            checked={settings.launchAtStartup}
            label={t("settings.startup")}
            onChange={(v) => saveSettings((s) => ({ ...s, launchAtStartup: v }))}
          />
        </Setting>
        <Setting title={t("settings.startMinimized")} hint={t("settings.startMinimizedHint")}>
          <Toggle
            checked={settings.startMinimized}
            disabled={!settings.launchAtStartup}
            label={t("settings.startMinimized")}
            onChange={(v) => saveSettings((s) => ({ ...s, startMinimized: v }))}
          />
        </Setting>
        <Setting title={t("settings.closeToTray")} hint={t("settings.closeToTrayHint")}>
          <Toggle
            checked={settings.closeToTray}
            label={t("settings.closeToTray")}
            onChange={(v) => saveSettings((s) => ({ ...s, closeToTray: v }))}
          />
        </Setting>
      </div>
    </Card>
  );
}

const OFFSET_RANGE = { min: -300, max: 300, step: 1 };

function OverlaySection({ settings }: { settings: Settings }) {
  const { t } = useI18n();
  const displays = useStore((s) => s.displays);
  const o = settings.overlay;
  const [offset, setOffset] = useState({ x: o.offsetX, y: o.offsetY });
  const timer = useRef<number | null>(null);
  useEffect(() => setOffset({ x: o.offsetX, y: o.offsetY }), [o.offsetX, o.offsetY]);

  const commitOffset = (next: { x: number; y: number }) => {
    setOffset(next);
    if (timer.current !== null) window.clearTimeout(timer.current);
    timer.current = window.setTimeout(
      () => saveSettings((s) => ({ ...s, overlay: { ...s.overlay, offsetX: next.x, offsetY: next.y } })),
      200,
    );
  };
  const setOverlay = (patch: Partial<Settings["overlay"]>) => saveSettings((s) => ({ ...s, overlay: { ...s.overlay, ...patch } }));

  return (
    <Card title={t("settings.overlay")} icon={<Crosshair />}>
      <div className="setting-list">
        <Setting title={t("settings.autoActivate")}>
          <Toggle checked={o.autoActivate} label={t("settings.autoActivate")} onChange={(v) => setOverlay({ autoActivate: v })} />
        </Setting>
        <Setting title={t("settings.monitor")}>
          <select className="select" value={o.monitor ?? ""} onChange={(e) => setOverlay({ monitor: e.target.value || null })}>
            <option value="">{t("settings.monitorPrimary")}</option>
            {displays.map((d) => (
              <option key={d.id} value={d.id}>
                {d.index}. {d.name} — {d.width}×{d.height}
              </option>
            ))}
          </select>
        </Setting>
        <Setting title={t("settings.visibility")} hint={t("settings.visibilityHint")}>
          <select
            className="select"
            value={o.visibility}
            onChange={(e) => setOverlay({ visibility: e.target.value as Settings["overlay"]["visibility"] })}
          >
            <option value="always">{t("settings.visibility.always")}</option>
            <option value="fullscreen_only">{t("settings.visibility.fullscreen_only")}</option>
          </select>
        </Setting>
        <Setting title={t("settings.keepOnTop")} hint={t("settings.keepOnTopHint")}>
          {o.keepOnTop && (
            <select
              className="select"
              style={{ width: 110 }}
              aria-label={t("settings.interval")}
              value={o.topmostIntervalMs}
              onChange={(e) => setOverlay({ topmostIntervalMs: Number(e.target.value) })}
            >
              {[500, 1000, 2000, 5000, 10000].map((ms) => (
                <option key={ms} value={ms}>
                  {ms / 1000} s
                </option>
              ))}
            </select>
          )}
          <Toggle checked={o.keepOnTop} label={t("settings.keepOnTop")} onChange={(v) => setOverlay({ keepOnTop: v })} />
        </Setting>
        <Setting title={t("settings.dpiScaling")} hint={t("settings.dpiScalingHint")}>
          <Toggle checked={o.dpiScaling} label={t("settings.dpiScaling")} onChange={(v) => setOverlay({ dpiScaling: v })} />
        </Setting>
        <div className="setting" style={{ display: "block" }}>
          <div className="row between" style={{ marginBottom: 10 }}>
            <div className="text">
              <span className="title">{t("settings.offset")}</span>
              <span className="hint">{t("settings.offsetHint")}</span>
            </div>
            <button className="btn btn-sm btn-ghost" onClick={() => commitOffset({ x: 0, y: 0 })} disabled={offset.x === 0 && offset.y === 0}>
              <RotateCcw /> {t("common.reset")}
            </button>
          </div>
          <div className="stack">
            <SliderField inline label="X" value={offset.x} range={OFFSET_RANGE} unit="px" onChange={(x) => commitOffset({ ...offset, x })} />
            <SliderField inline label="Y" value={offset.y} range={OFFSET_RANGE} unit="px" onChange={(y) => commitOffset({ ...offset, y })} />
          </div>
        </div>
      </div>
    </Card>
  );
}

function MonitorsSection({ settings }: { settings: Settings }) {
  const { t } = useI18n();
  const displays = useStore((s) => s.displays);
  const overlay = useStore((s) => s.overlay);
  const [busy, setBusy] = useState(false);
  const selected = settings.overlay.monitor ?? displays.find((d) => d.primary)?.id;

  const redetect = async () => {
    setBusy(true);
    try {
      store.set({ displays: await api.listDisplays(), overlay: await api.refreshOverlay() });
    } catch (e) {
      toast.error((e as Error).message);
    } finally {
      setBusy(false);
    }
  };

  return (
    <Card
      title={t("settings.monitors")}
      icon={<Monitor />}
      actions={
        <button className="btn btn-sm" onClick={redetect} disabled={busy}>
          {busy ? <span className="spinner" /> : <RotateCcw />} {t("settings.refreshMonitors")}
        </button>
      }
    >
      <p className="field-hint" style={{ marginBottom: 14 }}>
        {t("settings.monitorsHint")}
      </p>
      {overlay?.monitorFallback && (
        <div className="banner warning" style={{ marginBottom: 14 }}>
          <TriangleExclamationPoint />
          <span>{t("overlay.fallback")}</span>
        </div>
      )}
      <div className="monitor-grid">
        {displays.map((d) => {
          const isSelected = d.id === selected;
          return (
            <div key={d.id} className={`monitor-card${isSelected ? " selected" : ""}`}>
              <div className="screen">{d.index}</div>
              <div className="row between">
                <span className="name truncate">{d.name}</span>
                <span className={`badge ${d.primary ? "badge-accent" : ""}`}>{d.primary ? t("settings.primary") : t("settings.secondary")}</span>
              </div>
              <dl>
                <dt>{t("settings.mon.resolution")}</dt>
                <dd>
                  {d.width} × {d.height}
                </dd>
                <dt>{t("settings.mon.scale")}</dt>
                <dd>
                  {d.dpi} · {Math.round(d.scale * 100)} %
                </dd>
                <dt>{t("settings.mon.refresh")}</dt>
                <dd>{d.refreshRate ? `${d.refreshRate} Hz` : "—"}</dd>
                <dt>{t("settings.mon.position")}</dt>
                <dd>
                  {d.x}, {d.y}
                </dd>
                <dt>{t("settings.mon.id")}</dt>
                <dd className="mono truncate">{d.id}</dd>
              </dl>
              <button
                className={`btn btn-sm ${isSelected ? "" : "btn-primary"}`}
                disabled={isSelected}
                onClick={() => saveSettings((s) => ({ ...s, overlay: { ...s.overlay, monitor: d.primary ? null : d.id } }))}
              >
                {isSelected ? t("settings.inUse") : t("settings.useMonitor")}
              </button>
            </div>
          );
        })}
      </div>
    </Card>
  );
}

function ShortcutsSection() {
  const { t } = useI18n();
  const shortcuts = useStore((s) => s.shortcuts);
  const [busy, setBusy] = useState<ShortcutAction | "reset" | null>(null);

  const apply = async (action: ShortcutAction, accelerator: string, enabled: boolean) => {
    const conflict = shortcuts.find((s) => s.action !== action && s.enabled && s.accelerator.toLowerCase() === accelerator.toLowerCase());
    if (enabled && conflict) {
      toast.warning(t("shortcut.conflict", { action: t(`settings.shortcut.${conflict.action}`) }));
      return;
    }
    setBusy(action);
    try {
      store.set({ shortcuts: await api.setShortcut(action, accelerator, enabled) });
      if (enabled) toast.success(t("settings.shortcutSaved", { accel: accelerator }));
    } catch (e) {
      toast.error((e as Error).message, 8000);
      store.set({ shortcuts: await api.listShortcuts().catch(() => shortcuts) });
    } finally {
      setBusy(null);
    }
  };

  return (
    <Card
      title={t("settings.shortcuts")}
      icon={<Keyboard />}
      actions={
        <button
          className="btn btn-sm"
          disabled={busy !== null}
          onClick={async () => {
            setBusy("reset");
            try {
              store.set({ shortcuts: await api.resetShortcuts() });
            } catch (e) {
              toast.error((e as Error).message);
            } finally {
              setBusy(null);
            }
          }}
        >
          <RotateCcw /> {t("settings.shortcutsReset")}
        </button>
      }
    >
      <p className="field-hint">{t("settings.shortcutsHint")}</p>
      <div>
        {shortcuts.map((s) => (
          <div key={s.action} className="shortcut-row">
            <span className="title">{t(`settings.shortcut.${s.action}`)}</span>
            <ShortcutRecorder
              value={s.accelerator}
              disabled={busy !== null}
              onRecord={(accel) => apply(s.action, accel, true)}
              onInvalid={(reason) => toast.warning(reason === "modifier" ? t("shortcut.needModifier") : t("shortcut.record"))}
            />
            {!s.enabled ? (
              <span className="badge">{t("settings.shortcutDisabled")}</span>
            ) : s.registered ? (
              <span className="badge badge-success">{t("settings.shortcutRegistered")}</span>
            ) : (
              <span className="badge badge-danger">{t("settings.shortcutFailed")}</span>
            )}
            <Toggle
              checked={s.enabled}
              disabled={busy !== null}
              label={t("common.enabled")}
              onChange={(v) => apply(s.action, s.accelerator, v)}
            />
            {s.enabled && s.error && (
              <span className="err" title={s.error}>
                {t("settings.shortcutTaken", { accel: s.accelerator })}
              </span>
            )}
          </div>
        ))}
      </div>
    </Card>
  );
}

function PerformanceSection({ settings }: { settings: Settings }) {
  const { t } = useI18n();
  const overlay = useStore((s) => s.overlay);
  const [perf, setPerf] = useState<PerformanceInfo | null>(null);

  useEffect(() => {
    let alive = true;
    const poll = () =>
      api
        .getPerformance()
        .then((p) => alive && setPerf(p))
        .catch(() => undefined);
    poll();
    const id = window.setInterval(poll, 2000);
    return () => {
      alive = false;
      window.clearInterval(id);
    };
  }, []);

  const o = perf?.overlay ?? overlay;
  return (
    <>
      <Card title={t("settings.performance")} icon={<Gauge />}>
        <div className="setting-list">
          <Setting title={t("settings.liteMode")} hint={t("settings.liteModeHint")}>
            <Toggle
              checked={settings.performance.liteMode}
              label={t("settings.liteMode")}
              onChange={(v) => saveSettings((s) => ({ ...s, performance: { liteMode: v } }))}
            />
          </Setting>
        </div>
      </Card>
      <Card title={t("settings.rendering")} icon={<Crosshair />}>
        <p className="field-hint" style={{ marginBottom: 12 }}>
          {t("settings.renderingHint")}
        </p>
        <div className="metric-grid">
          <div className="metric">
            <div className="label">{t("settings.renderCount")}</div>
            <div className="value">{o?.renderCount ?? "—"}</div>
          </div>
          <div className="metric">
            <div className="label">{t("settings.lastRender")}</div>
            <div className="value">{o ? formatMicros(o.lastRenderMicros) : "—"}</div>
          </div>
          <div className="metric">
            <div className="label">{t("settings.overlaySize")}</div>
            <div className="value">{o ? `${o.width}×${o.height}` : "—"}</div>
          </div>
        </div>
      </Card>
      <Card title={t("settings.usage")} icon={<Cpu />}>
        <p className="field-hint" style={{ marginBottom: 12 }}>
          {t("settings.usageHint")}
        </p>
        <div className="metric-grid">
          <div className="metric">
            <div className="label">{t("settings.memory")}</div>
            <div className="value">{perf ? formatBytes(perf.process.workingSetBytes) : "—"}</div>
          </div>
          <div className="metric">
            <div className="label">{t("settings.private")}</div>
            <div className="value">{perf ? formatBytes(perf.process.privateBytes) : "—"}</div>
          </div>
          <div className="metric">
            <div className="label">{t("settings.cpu")}</div>
            <div className="value">{perf ? `${perf.process.cpuPercent.toFixed(1)} %` : "—"}</div>
          </div>
          <div className="metric">
            <div className="label">{t("settings.uptime")}</div>
            <div className="value">{perf ? formatDuration(perf.process.uptimeSeconds) : "—"}</div>
          </div>
        </div>
      </Card>
    </>
  );
}

function DataSection() {
  const { t } = useI18n();
  const app = useStore((s) => s.app);
  const [restoreSettings, setRestoreSettings] = useState(false);
  const [confirmReset, setConfirmReset] = useState(false);
  const [busy, setBusy] = useState(false);

  const guarded = async (fn: () => Promise<void>) => {
    setBusy(true);
    try {
      await fn();
    } catch (e) {
      toast.error((e as Error).message, 9000);
    } finally {
      setBusy(false);
    }
  };

  return (
    <>
      <Card title={t("settings.dataLocation")} icon={<FolderOpen />}>
        <div className="path-box">
          <code title={app?.dbPath}>{app?.dbPath}</code>
          <button className="btn btn-sm" onClick={() => guarded(() => api.openDataFolder())}>
            <FolderOpen /> {t("settings.openFolder")}
          </button>
        </div>
        <p className="field-hint" style={{ marginTop: 8 }}>
          {t("settings.schema", { version: app?.schemaVersion ?? "?" })}
        </p>
      </Card>

      <Card title={t("settings.backup")} icon={<Database />}>
        <p className="field-hint" style={{ marginBottom: 14 }}>
          {t("settings.backupHint")}
        </p>
        <div className="row wrap">
          <button
            className="btn"
            disabled={busy}
            onClick={() =>
              guarded(async () => {
                const path = await api.exportBackup(t("settings.exportBackup"));
                if (path) toast.success(t("settings.backupExported", { path }));
              })
            }
          >
            <Download /> {t("settings.exportBackup")}
          </button>
          <button
            className="btn"
            disabled={busy}
            onClick={() =>
              guarded(async () => {
                const r = await api.importBackup(t("settings.importBackup"), restoreSettings);
                if (!r) return;
                if (r.settingsRestored) store.set({ settings: await api.getSettings(), shortcuts: await api.listShortcuts() });
                toast.success(t("profiles.imported", { profiles: r.profilesImported, presets: r.presetsImported }));
                if (r.skipped.length) toast.warning(`${t("profiles.skipped", { count: r.skipped.length })} ${r.skipped.map((s) => s.name).join(", ")}`);
              })
            }
          >
            <Upload /> {t("settings.importBackup")}
          </button>
          <label className="check">
            <input type="checkbox" checked={restoreSettings} onChange={(e) => setRestoreSettings(e.target.checked)} />
            {t("settings.restoreSettings")}
          </label>
        </div>
        <div className="row" style={{ marginTop: 14 }}>
          <button
            className="btn btn-ghost"
            disabled={busy}
            onClick={() =>
              guarded(async () => {
                const n = await api.restoreBuiltinPresets();
                toast.success(t("settings.restoredBuiltins", { count: n }));
                store.bumpData();
              })
            }
          >
            <RotateCcw /> {t("settings.restoreBuiltins")}
          </button>
        </div>
      </Card>

      <Card title={t("settings.reset")} icon={<TriangleExclamationPoint />} className="danger-zone">
        <div className="row between wrap">
          <p className="muted">{t("settings.resetHint")}</p>
          <button className="btn btn-danger" onClick={() => setConfirmReset(true)}>
            {t("settings.reset")}
          </button>
        </div>
      </Card>

      {confirmReset && (
        <ConfirmDialog
          title={t("settings.resetTitle")}
          body={t("settings.resetBody")}
          confirmLabel={t("settings.reset")}
          danger
          busy={busy}
          onCancel={() => setConfirmReset(false)}
          onConfirm={() =>
            guarded(async () => {
              const settings = await api.resetAllData();
              store.set({ settings, shortcuts: await api.listShortcuts() });
              setConfirmReset(false);
              toast.success(t("settings.resetDone"));
            })
          }
        />
      )}
    </>
  );
}
