import { useMemo, useState } from "react";
import { PageHeader } from "../components/PageHeader";
import { CrosshairThumb } from "../components/CrosshairPreview";
import { Field } from "../components/controls";
import { Check, Copy, Pencil, Plus, Save, Search, Star, Trash } from "../components/icons";
import { ConfirmDialog, Modal } from "../components/Modal";
import { useI18n } from "../hooks/useI18n";
import { useResource } from "../hooks/useResource";
import { api } from "../services/api";
import { store, useStore } from "../services/store";
import { toast } from "../services/toast";
import { PRESET_CATEGORIES, type Preset, type PresetCategory } from "../types/models";

type Filter = "all" | "favorites" | PresetCategory;

export function LibraryPage() {
  const { t, date } = useI18n();
  const limits = useStore((s) => s.limits);
  const activeSpec = useStore((s) => s.activeProfile?.spec);
  const presets = useResource(() => api.listPresets());
  const [filter, setFilter] = useState<Filter>("all");
  const [query, setQuery] = useState("");
  const [toDelete, setToDelete] = useState<Preset | null>(null);
  const [creating, setCreating] = useState<{ name: string; category: PresetCategory } | null>(null);

  const all = presets.data ?? [];
  const activeKey = activeSpec ? JSON.stringify(activeSpec) : "";
  const counts = useMemo(() => {
    const c: Record<string, number> = { all: all.length, favorites: all.filter((p) => p.favorite).length };
    for (const cat of PRESET_CATEGORIES) c[cat] = all.filter((p) => p.category === cat).length;
    return c;
  }, [all]);

  const visible = useMemo(() => {
    const q = query.trim().toLowerCase();
    return all.filter(
      (p) =>
        (filter === "all" || (filter === "favorites" ? p.favorite : p.category === filter)) &&
        (!q || p.name.toLowerCase().includes(q)),
    );
  }, [all, filter, query]);

  const run = async <T,>(fn: () => Promise<T>, success?: (r: T) => string) => {
    try {
      const r = await fn();
      if (success) toast.success(success(r));
      presets.reload();
      return r;
    } catch (e) {
      toast.error((e as Error).message);
      return undefined;
    }
  };

  const apply = (p: Preset) => run(() => api.applyPreset(p.id), () => t("library.applied", { name: p.name }));

  const edit = async (p: Preset) => {
    const ok = await run(() => api.applyPreset(p.id));
    if (ok) store.navigate(p.spec.kind === "dot" ? "dot" : "editor", { presetId: p.id, name: p.name, builtin: p.builtin });
  };

  const create = async () => {
    if (!creating || !activeSpec) return;
    const r = await run(
      () => api.createPreset({ name: creating.name, category: creating.category, spec: activeSpec }),
      (p) => t("editor.saved", { name: p.name }),
    );
    if (r) setCreating(null);
  };

  const filters: Filter[] = ["all", "favorites", ...PRESET_CATEGORIES];

  return (
    <div className="page">
      <PageHeader
        title={t("nav.library")}
        subtitle={t("library.subtitle", { count: all.length })}
        actions={
          <button className="btn btn-primary" onClick={() => setCreating({ name: "", category: "custom" })}>
            <Plus /> {t("library.newFromCurrent")}
          </button>
        }
      />

      <div className="library-toolbar">
        <div className="tabs" role="tablist">
          {filters.map((f) => (
            <button key={f} className="tab" role="tab" aria-selected={filter === f} onClick={() => setFilter(f)}>
              {f === "favorites" && <Star size={13} />}
              {f === "all" ? t("library.all") : f === "favorites" ? t("library.favorites") : t(`cat.${f}`)}
              <span className="count">{counts[f] ?? 0}</span>
            </button>
          ))}
        </div>
        <div className="input-with-icon">
          <Search />
          <input className="input" placeholder={t("common.search")} value={query} onChange={(e) => setQuery(e.target.value)} />
        </div>
      </div>

      {presets.error && <div className="banner danger">{t("common.error", { message: presets.error })}</div>}
      {!presets.loading && visible.length === 0 && <div className="card empty">{t("library.empty")}</div>}

      <div className="preset-grid">
        {visible.map((p) => {
          const isActive = JSON.stringify(p.spec) === activeKey;
          return (
            <article key={p.id} className={`card preset-card${isActive ? " active" : ""}`}>
              <div className="thumb">
                <CrosshairThumb spec={p.spec} size={110} />
                <button
                  className={`icon-btn fav${p.favorite ? " active" : ""}`}
                  title={p.favorite ? t("library.unfavorite") : t("library.favorite")}
                  aria-pressed={p.favorite}
                  onClick={() => run(() => api.setPresetFavorite(p.id, !p.favorite))}
                >
                  <Star />
                </button>
              </div>
              <div className="meta">
                <div className="row between">
                  <span className="name truncate">{p.name}</span>
                </div>
                <div className="row wrap" style={{ gap: 6 }}>
                  <span className="badge badge-accent">{t(`cat.${p.category}`)}</span>
                  <span className="badge">{t(p.spec.kind === "dot" ? "kind.dot" : "kind.crosshair")}</span>
                  {p.builtin && <span className="badge">{t("common.builtin")}</span>}
                </div>
                <span className="date">{t("common.created", { date: date(p.createdAt) })}</span>
              </div>
              <div className="actions">
                <button className={`btn btn-sm ${isActive ? "" : "btn-primary"} grow`} onClick={() => apply(p)} disabled={isActive}>
                  {isActive ? (
                    <>
                      <Check /> {t("common.applied")}
                    </>
                  ) : (
                    t("common.apply")
                  )}
                </button>
                <button className="icon-btn" title={t("common.edit")} onClick={() => edit(p)}>
                  <Pencil />
                </button>
                <button
                  className="icon-btn"
                  title={t("common.duplicate")}
                  onClick={() => run(() => api.duplicatePreset(p.id), (c) => t("library.duplicated", { name: c.name }))}
                >
                  <Copy />
                </button>
                <button className="icon-btn danger" title={t("common.delete")} onClick={() => setToDelete(p)}>
                  <Trash />
                </button>
              </div>
            </article>
          );
        })}
      </div>

      {toDelete && (
        <ConfirmDialog
          title={t("library.deleteTitle", { name: toDelete.name })}
          body={toDelete.builtin ? t("library.deleteBuiltinBody") : t("library.deleteBody")}
          confirmLabel={t("common.delete")}
          danger
          onCancel={() => setToDelete(null)}
          onConfirm={async () => {
            await run(() => api.deletePreset(toDelete.id), () => t("library.deleted"));
            setToDelete(null);
          }}
        />
      )}

      {creating && limits && (
        <Modal
          title={t("library.createTitle")}
          onClose={() => setCreating(null)}
          footer={
            <>
              <button className="btn" onClick={() => setCreating(null)}>
                {t("common.cancel")}
              </button>
              <button className="btn btn-primary" disabled={!creating.name.trim()} onClick={create}>
                <Save /> {t("common.save")}
              </button>
            </>
          }
        >
          <div className="row" style={{ gap: 16 }}>
            {activeSpec && (
              <div className="thumb" style={{ width: 88, height: 88, flex: "none" }}>
                <CrosshairThumb spec={activeSpec} size={76} />
              </div>
            )}
            <div className="stack grow">
              <Field label={t("common.name")}>
                <input
                  className="input"
                  value={creating.name}
                  maxLength={limits.nameMaxChars}
                  onChange={(e) => setCreating({ ...creating, name: e.target.value })}
                  onKeyDown={(e) => e.key === "Enter" && creating.name.trim() && create()}
                />
              </Field>
              <Field label={t("common.category")}>
                <select
                  className="select"
                  value={creating.category}
                  onChange={(e) => setCreating({ ...creating, category: e.target.value as PresetCategory })}
                >
                  {PRESET_CATEGORIES.map((c) => (
                    <option key={c} value={c}>
                      {t(`cat.${c}`)}
                    </option>
                  ))}
                </select>
              </Field>
            </div>
          </div>
        </Modal>
      )}
    </div>
  );
}
