import { useCallback, useRef, useState } from "react";
import { Card, PageHeader } from "../components/PageHeader";
import { CrosshairPreview } from "../components/CrosshairPreview";
import { ColorField, Field, Segmented, SliderField, ToggleRow } from "../components/controls";
import { Check, CircleDot, Info, Pencil, RotateCcw } from "../components/icons";
import { useBinder } from "../hooks/useBinder";
import { useI18n } from "../hooks/useI18n";
import { useLiveSpec } from "../hooks/useLiveSpec";
import { api } from "../services/api";
import { DEFAULT_DOT, patchDot, type DeepPartial } from "../services/spec";
import { store, useStore } from "../services/store";
import { toast } from "../services/toast";
import type { DotConfig, DotSizePreset } from "../types/models";

export function CenterDotPage() {
  const { t } = useI18n();
  const limits = useStore((s) => s.limits);
  const settings = useStore((s) => s.settings);
  const { draft, update } = useLiveSpec();
  const draftRef = useRef(draft);
  draftRef.current = draft;

  const applyPatch = useCallback(
    (patch: DeepPartial<DotConfig>) => {
      const d = draftRef.current;
      if (d) update(patchDot(d, patch));
    },
    [update],
  );
  const bind = useBinder<DeepPartial<DotConfig>>(applyPatch);

  const [editing, setEditing] = useState<DotSizePreset[] | null>(null);

  if (!draft || !limits || !settings) return null;
  const d = draft.dot;
  const active = draft.kind === "dot";

  const saveSizes = async () => {
    if (!editing) return;
    try {
      const applied = await api.updateSettings({ ...settings, dotPresets: editing });
      store.set({ settings: applied });
      setEditing(null);
      toast.success(t("dot.sizesSaved"));
    } catch (e) {
      toast.error((e as Error).message);
    }
  };

  return (
    <div className="page">
      <PageHeader
        title={t("nav.dot")}
        subtitle={t("dot.subtitle")}
        actions={
          <>
            <button className="btn btn-ghost" onClick={() => applyPatch(DEFAULT_DOT)}>
              <RotateCcw /> {t("editor.resetDefaults")}
            </button>
            {active ? (
              <span className="badge badge-success" style={{ height: 36, padding: "0 14px" }}>
                <CircleDot /> {t("dot.isActive")}
              </span>
            ) : (
              <button className="btn btn-primary" onClick={() => update({ ...draft, kind: "dot" })}>
                <CircleDot /> {t("dot.activate")}
              </button>
            )}
          </>
        }
      />

      {!active && (
        <div className="banner info">
          <Info />
          <span className="grow">{t("dot.inactive")}</span>
        </div>
      )}

      <div className="editor-layout">
        <div className="editor-stage">
          <CrosshairPreview spec={{ ...draft, kind: "dot" }} toolbar defaultZoom={4} />
        </div>

        <div className="card editor-panel">
          <div className="card-body">
            <div className="section">
              <div className="section-title">
                {t("dot.sizes")}
                {editing ? (
                  <button className="btn btn-sm btn-primary" onClick={saveSizes}>
                    <Check /> {t("dot.doneSizes")}
                  </button>
                ) : (
                  <button className="btn btn-sm btn-ghost" onClick={() => setEditing(structuredClone(settings.dotPresets))}>
                    <Pencil /> {t("dot.editSizes")}
                  </button>
                )}
              </div>
              {editing ? (
                <div className="dot-sizes">
                  {editing.map((p, i) => (
                    <div key={p.id} className="dot-size-edit">
                      <input
                        className="input"
                        value={p.name}
                        maxLength={limits.nameMaxChars}
                        aria-label={t("common.name")}
                        onChange={(e) => setEditing(editing.map((x, j) => (j === i ? { ...x, name: e.target.value } : x)))}
                      />
                      <input
                        className="input num"
                        type="number"
                        min={limits.dotSize.min}
                        max={limits.dotSize.max}
                        step={limits.dotSize.step}
                        value={p.size}
                        aria-label={t("dot.size")}
                        onChange={(e) => setEditing(editing.map((x, j) => (j === i ? { ...x, size: Number(e.target.value) } : x)))}
                      />
                    </div>
                  ))}
                </div>
              ) : (
                <div className="dot-sizes">
                  {settings.dotPresets.map((p) => (
                    <button key={p.id} className="dot-size" aria-pressed={d.size === p.size} onClick={() => applyPatch({ size: p.size })}>
                      <span className="sample">
                        <i style={{ width: Math.min(p.size, 18), height: Math.min(p.size, 18) }} />
                      </span>
                      <span className="title">{p.name}</span>
                      <span className="px num">{p.size} px</span>
                    </button>
                  ))}
                </div>
              )}
            </div>

            <div className="section">
              <SliderField label={t("dot.size")} value={d.size} range={limits.dotSize} unit="px" onChange={bind("size")} />
              <Field label={t("dot.shape")}>
                <Segmented
                  block
                  value={d.shape}
                  onChange={bind("shape")}
                  options={[
                    { value: "circle", label: t("dot.circle") },
                    { value: "square", label: t("dot.square") },
                  ]}
                />
              </Field>
              <ColorField label={t("editor.color")} value={d.color} onChange={bind("color")} />
              <SliderField label={t("editor.opacity")} value={d.opacity} range={limits.opacity} unit="%" displayFactor={100} onChange={bind("opacity")} />
            </div>

            <div className="section">
              <ToggleRow title={t("editor.outline")} checked={d.outline.enabled} onChange={bind("outline.enabled")} />
              {d.outline.enabled && (
                <>
                  <ColorField label={t("editor.outlineColor")} value={d.outline.color} onChange={bind("outline.color")} />
                  <SliderField
                    label={t("editor.outlineThickness")}
                    value={d.outline.thickness}
                    range={limits.outlineThickness}
                    unit="px"
                    onChange={bind("outline.thickness")}
                  />
                </>
              )}
            </div>

            <div className="section">
              <ToggleRow title={t("dot.halo")} checked={d.halo.enabled} onChange={bind("halo.enabled")} />
              {d.halo.enabled && (
                <>
                  <ColorField label={t("dot.haloColor")} value={d.halo.color} onChange={bind("halo.color")} />
                  <SliderField label={t("dot.haloSize")} value={d.halo.size} range={limits.haloSize} unit="px" onChange={bind("halo.size")} />
                  <SliderField
                    label={t("dot.haloOpacity")}
                    value={d.halo.opacity}
                    range={limits.opacity}
                    unit="%"
                    displayFactor={100}
                    onChange={bind("halo.opacity")}
                  />
                </>
              )}
            </div>
          </div>
        </div>
      </div>

      <Card>
        <p className="field-hint">{t("preview.exact")}</p>
      </Card>
    </div>
  );
}
