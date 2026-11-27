import { useCallback, useMemo, useRef, useState } from "react";
import { Card, PageHeader } from "../components/PageHeader";
import { CrosshairPreview, CrosshairThumb } from "../components/CrosshairPreview";
import { ColorField, Field, Segmented, SliderField, ToggleRow } from "../components/controls";
import { Info, Link, Palette, RotateCcw, Save, Shapes, Sparkles, Unlink } from "../components/icons";
import { Modal } from "../components/Modal";
import { ShapeIcon } from "../components/ShapeIcon";
import { useBinder } from "../hooks/useBinder";
import { useI18n } from "../hooks/useI18n";
import { useLiveSpec } from "../hooks/useLiveSpec";
import { useResource } from "../hooks/useResource";
import { api } from "../services/api";
import { DEFAULT_CROSSHAIR, patchCrosshair, SHAPES, shapeFeatures, type DeepPartial } from "../services/spec";
import { store, useStore } from "../services/store";
import { toast } from "../services/toast";
import { PRESET_CATEGORIES, type CrosshairConfig, type PresetCategory } from "../types/models";

export function EditorPage() {
  const { t } = useI18n();
  const limits = useStore((s) => s.limits);
  const origin = useStore((s) => s.editorOrigin);
  const { draft, update } = useLiveSpec();
  const draftRef = useRef(draft);
  draftRef.current = draft;

  const applyPatch = useCallback(
    (patch: DeepPartial<CrosshairConfig>) => {
      const d = draftRef.current;
      if (d) update(patchCrosshair(d, patch));
    },
    [update],
  );
  const bind = useBinder<DeepPartial<CrosshairConfig>>(applyPatch);

  const [linked, setLinked] = useState(() => (draft ? draft.crosshair.lengthH === draft.crosshair.lengthV : true));
  const setLengthH = useCallback((v: number) => applyPatch(linked ? { lengthH: v, lengthV: v } : { lengthH: v }), [applyPatch, linked]);
  const setLengthV = useCallback((v: number) => applyPatch(linked ? { lengthH: v, lengthV: v } : { lengthV: v }), [applyPatch, linked]);

  const presets = useResource(() => api.listPresets());
  const quick = useMemo(
    () =>
      (presets.data ?? [])
        .filter((p) => p.spec.kind === "crosshair")
        .sort((a, b) => Number(b.favorite) - Number(a.favorite))
        .slice(0, 14),
    [presets.data],
  );
  const originPreset = origin ? presets.data?.find((p) => p.id === origin.presetId) : undefined;

  const [saving, setSaving] = useState<{ name: string; category: PresetCategory } | null>(null);

  if (!draft || !limits) return null;
  const c = draft.crosshair;
  const f = shapeFeatures(c);

  const savePreset = async () => {
    if (!saving) return;
    try {
      const p = await api.createPreset({ name: saving.name, category: saving.category, spec: { ...draft, kind: "crosshair" } });
      toast.success(t("editor.saved", { name: p.name }));
      setSaving(null);
      store.bumpData();
    } catch (e) {
      toast.error((e as Error).message);
    }
  };

  const updateOriginPreset = async () => {
    if (!originPreset) return;
    try {
      const p = await api.updatePreset(originPreset.id, {
        name: originPreset.name,
        category: originPreset.category,
        spec: { ...draft, kind: "crosshair" },
      });
      toast.success(t("editor.updated", { name: p.name }));
      store.bumpData();
    } catch (e) {
      toast.error((e as Error).message);
    }
  };

  return (
    <div className="page">
      <PageHeader
        title={t("nav.editor")}
        subtitle={origin ? t("editor.editingPreset", { name: origin.name }) : t("editor.subtitle")}
        actions={
          <>
            <button className="btn btn-ghost" onClick={() => applyPatch(DEFAULT_CROSSHAIR)}>
              <RotateCcw /> {t("editor.resetDefaults")}
            </button>
            {originPreset && !originPreset.builtin && (
              <button className="btn" onClick={updateOriginPreset}>
                <Save /> {t("editor.updatePreset", { name: originPreset.name })}
              </button>
            )}
            <button className="btn btn-primary" onClick={() => setSaving({ name: "", category: "custom" })}>
              <Save /> {t("editor.saveAsPreset")}
            </button>
          </>
        }
      />

      {draft.kind === "dot" && (
        <div className="banner info">
          <Info />
          <span className="grow">{t("editor.dotModeActive")}</span>
          <button className="btn btn-sm btn-primary" onClick={() => update({ ...draft, kind: "crosshair" })}>
            {t("editor.useCrosshair")}
          </button>
        </div>
      )}

      <div className="editor-layout">
        <div className="editor-stage">
          <CrosshairPreview spec={{ ...draft, kind: "crosshair" }} toolbar defaultZoom={4} />
          <Card title={t("editor.quickPresets")} icon={<Sparkles />}>
            <div className="quick-presets">
              {quick.map((p) => (
                <button
                  key={p.id}
                  className="quick-preset"
                  title={p.name}
                  onClick={() => api.applyPreset(p.id).catch((e: Error) => toast.error(e.message))}
                >
                  <div className="thumb" style={{ width: 76, height: 76 }}>
                    <CrosshairThumb spec={p.spec} size={64} />
                  </div>
                  <span className="truncate" style={{ maxWidth: "100%" }}>
                    {p.name}
                  </span>
                </button>
              ))}
            </div>
          </Card>
        </div>

        <div className="card editor-panel">
          <div className="card-body">
            <div className="section">
              <div className="section-title">
                <span className="row">
                  <Shapes size={14} /> {t("editor.shape")}
                </span>
              </div>
              <div className="shape-grid">
                {SHAPES.map((s) => (
                  <button key={s} className="shape-btn" aria-pressed={c.shape === s} onClick={() => applyPatch({ shape: s })}>
                    <ShapeIcon shape={s} />
                    {t(`shape.${s}`)}
                  </button>
                ))}
              </div>
              {f.custom && (
                <div className="element-grid">
                  {(["top", "right", "bottom", "left", "circle", "dot"] as const).map((k) => (
                    <label key={k} className="check">
                      <input
                        type="checkbox"
                        checked={c.custom[k]}
                        onChange={(e) => applyPatch({ custom: { [k]: e.target.checked } })}
                      />
                      {t(`editor.el.${k}`)}
                    </label>
                  ))}
                </div>
              )}
            </div>

            {f.arms && (
              <div className="section">
                <div className="section-title">
                  {t("editor.dimensions")}
                  <button
                    className={`icon-btn${linked ? " active" : ""}`}
                    title={t("editor.linkLengths")}
                    aria-pressed={linked}
                    style={linked ? { color: "var(--accent-text)" } : undefined}
                    onClick={() => setLinked((l) => !l)}
                  >
                    {linked ? <Link /> : <Unlink />}
                  </button>
                </div>
                <SliderField label={t("editor.lengthH")} value={c.lengthH} range={limits.length} unit="px" onChange={setLengthH} />
                <SliderField label={t("editor.lengthV")} value={c.lengthV} range={limits.length} unit="px" onChange={setLengthV} />
                <SliderField label={t("editor.thickness")} value={c.thickness} range={limits.thickness} unit="px" onChange={bind("thickness")} />
                {f.gap && <SliderField label={t("editor.gap")} value={c.gap} range={limits.gap} unit="px" onChange={bind("gap")} />}
                {f.dynamic && (
                  <SliderField label={t("editor.splitGap")} value={c.splitGap} range={limits.splitGap} unit="px" onChange={bind("splitGap")} />
                )}
              </div>
            )}

            {f.circle && (
              <div className="section">
                <div className="section-title">{t("editor.circle")}</div>
                <SliderField label={t("editor.circleRadius")} value={c.circleRadius} range={limits.circleRadius} unit="px" onChange={bind("circleRadius")} />
                <SliderField
                  label={t("editor.circleThickness")}
                  value={c.circleThickness}
                  range={limits.circleThickness}
                  unit="px"
                  onChange={bind("circleThickness")}
                />
              </div>
            )}

            {f.dot && (
              <div className="section">
                <div className="section-title">{t("editor.centerDot")}</div>
                <SliderField
                  label={t("editor.dotSize")}
                  value={c.dotSize}
                  range={c.shape === "dot" ? { ...limits.centerDotSize, min: 1 } : limits.centerDotSize}
                  unit="px"
                  onChange={bind("dotSize")}
                />
                <Field label={t("editor.dotShape")}>
                  <Segmented
                    block
                    value={c.dotShape}
                    onChange={bind("dotShape")}
                    options={[
                      { value: "square", label: t("dot.square") },
                      { value: "circle", label: t("dot.circle") },
                    ]}
                  />
                </Field>
              </div>
            )}

            <div className="section">
              <div className="section-title">
                <span className="row">
                  <Palette size={14} /> {t("editor.style")}
                </span>
              </div>
              <ColorField label={t("editor.color")} value={c.color} onChange={bind("color")} />
              <SliderField label={t("editor.opacity")} value={c.opacity} range={limits.opacity} unit="%" displayFactor={100} onChange={bind("opacity")} />
              <SliderField label={t("editor.rotation")} value={c.rotation} range={limits.rotation} unit="°" onChange={bind("rotation")} />
            </div>

            <div className="section">
              <ToggleRow title={t("editor.outline")} checked={c.outline.enabled} onChange={bind("outline.enabled")} />
              {c.outline.enabled && (
                <>
                  <ColorField label={t("editor.outlineColor")} value={c.outline.color} onChange={bind("outline.color")} />
                  <SliderField
                    label={t("editor.outlineThickness")}
                    value={c.outline.thickness}
                    range={limits.outlineThickness}
                    unit="px"
                    onChange={bind("outline.thickness")}
                  />
                </>
              )}
            </div>

            <div className="section">
              <ToggleRow title={t("editor.shadow")} checked={c.shadow.enabled} onChange={bind("shadow.enabled")} />
              {c.shadow.enabled && (
                <>
                  <ColorField label={t("editor.shadowColor")} value={c.shadow.color} onChange={bind("shadow.color")} />
                  <SliderField
                    label={t("editor.shadowOpacity")}
                    value={c.shadow.opacity}
                    range={limits.opacity}
                    unit="%"
                    displayFactor={100}
                    onChange={bind("shadow.opacity")}
                  />
                  <SliderField label={t("editor.shadowX")} value={c.shadow.offsetX} range={limits.shadowOffset} unit="px" onChange={bind("shadow.offsetX")} />
                  <SliderField label={t("editor.shadowY")} value={c.shadow.offsetY} range={limits.shadowOffset} unit="px" onChange={bind("shadow.offsetY")} />
                  <SliderField label={t("editor.shadowBlur")} value={c.shadow.blur} range={limits.shadowBlur} unit="px" onChange={bind("shadow.blur")} />
                </>
              )}
            </div>
          </div>
        </div>
      </div>

      {saving && (
        <Modal
          title={t("library.createTitle")}
          onClose={() => setSaving(null)}
          footer={
            <>
              <button className="btn" onClick={() => setSaving(null)}>
                {t("common.cancel")}
              </button>
              <button className="btn btn-primary" disabled={!saving.name.trim()} onClick={savePreset}>
                <Save /> {t("common.save")}
              </button>
            </>
          }
        >
          <Field label={t("common.name")}>
            <input
              className="input"
              value={saving.name}
              maxLength={limits.nameMaxChars}
              onChange={(e) => setSaving({ ...saving, name: e.target.value })}
              onKeyDown={(e) => e.key === "Enter" && saving.name.trim() && savePreset()}
            />
          </Field>
          <Field label={t("common.category")}>
            <select
              className="select"
              value={saving.category}
              onChange={(e) => setSaving({ ...saving, category: e.target.value as PresetCategory })}
            >
              {PRESET_CATEGORIES.map((cat) => (
                <option key={cat} value={cat}>
                  {t(`cat.${cat}`)}
                </option>
              ))}
            </select>
          </Field>
        </Modal>
      )}
    </div>
  );
}
