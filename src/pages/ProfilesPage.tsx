import { useState } from "react";
import { PageHeader } from "../components/PageHeader";
import { CrosshairThumb } from "../components/CrosshairPreview";
import { Field, Segmented } from "../components/controls";
import { Check, Copy, Download, Pencil, Play, Plus, Save, Trash, Upload } from "../components/icons";
import { ConfirmDialog, Modal } from "../components/Modal";
import { useI18n } from "../hooks/useI18n";
import { useResource } from "../hooks/useResource";
import { api } from "../services/api";
import { defaultSpec } from "../services/spec";
import { useStore } from "../services/store";
import { toast } from "../services/toast";
import type { ImportReport, Profile } from "../types/models";

interface Draft {
  id?: string;
  name: string;
  description: string;
  base: "current" | "default";
}

export function ProfilesPage() {
  const { t, date } = useI18n();
  const limits = useStore((s) => s.limits);
  const activeId = useStore((s) => s.activeProfile?.id);
  const profiles = useResource(() => api.listProfiles());
  const [draft, setDraft] = useState<Draft | null>(null);
  const [toDelete, setToDelete] = useState<Profile | null>(null);
  const [report, setReport] = useState<ImportReport | null>(null);
  const list = profiles.data ?? [];

  const run = async <T,>(fn: () => Promise<T>, success?: (r: T) => string) => {
    try {
      const r = await fn();
      if (success) toast.success(success(r));
      profiles.reload();
      return r;
    } catch (e) {
      toast.error((e as Error).message);
      return undefined;
    }
  };

  const save = async () => {
    if (!draft) return;
    const input = { name: draft.name, description: draft.description };
    const r = draft.id
      ? await run(() => api.updateProfile(draft.id!, input))
      : await run(
          () => api.createProfile({ ...input, spec: draft.base === "default" ? defaultSpec() : undefined }),
          (p) => t("profiles.created", { name: p.name }),
        );
    if (r) setDraft(null);
  };

  const exportProfiles = async (ids?: string[]) => {
    try {
      const path = await api.exportProfiles(t("profiles.exportTitle"), ids);
      if (path) toast.success(t("profiles.exported", { path }));
    } catch (e) {
      toast.error((e as Error).message);
    }
  };

  const importProfiles = async () => {
    try {
      const r = await api.importProfiles(t("profiles.importTitle"));
      if (!r) return;
      profiles.reload();
      if (r.skipped.length) setReport(r);
      else toast.success(t("profiles.imported", { profiles: r.profilesImported, presets: r.presetsImported }));
    } catch (e) {
      toast.error((e as Error).message, 9000);
    }
  };

  return (
    <div className="page">
      <PageHeader
        title={t("nav.profiles")}
        subtitle={t("profiles.subtitle", { count: list.length })}
        actions={
          <>
            <button className="btn" onClick={importProfiles}>
              <Upload /> {t("profiles.importBtn")}
            </button>
            <button className="btn" onClick={() => exportProfiles()} disabled={list.length === 0}>
              <Download /> {t("profiles.exportAll")}
            </button>
            <button className="btn btn-primary" onClick={() => setDraft({ name: "", description: "", base: "current" })}>
              <Plus /> {t("profiles.new")}
            </button>
          </>
        }
      />

      {profiles.error && <div className="banner danger">{t("common.error", { message: profiles.error })}</div>}

      <div className="profile-list">
        {list.map((p) => {
          const active = p.id === activeId;
          return (
            <article key={p.id} className={`card profile-card${active ? " active" : ""}`}>
              <div className="thumb">
                <CrosshairThumb spec={p.spec} size={84} />
              </div>
              <div className="info">
                <div className="title">
                  <span className="truncate">{p.name}</span>
                  {active && (
                    <span className="badge badge-success">
                      <Check /> {t("common.active")}
                    </span>
                  )}
                </div>
                <div className="row wrap" style={{ gap: 6 }}>
                  <span className="badge badge-accent">{t(p.spec.kind === "dot" ? "kind.dot" : "kind.crosshair")}</span>
                </div>
                {p.description && <p className="desc">{p.description}</p>}
                <span className="dates">
                  {t("common.created", { date: date(p.createdAt) })} · {t("common.updated", { date: date(p.updatedAt) })}
                </span>
                <div className="actions">
                  {!active && (
                    <button
                      className="btn btn-sm btn-primary"
                      onClick={() => run(() => api.activateProfile(p.id), (r) => t("profiles.activated", { name: r.name }))}
                    >
                      <Play /> {t("common.activate")}
                    </button>
                  )}
                  <button
                    className="icon-btn"
                    title={t("common.edit")}
                    onClick={() => setDraft({ id: p.id, name: p.name, description: p.description, base: "current" })}
                  >
                    <Pencil />
                  </button>
                  <button
                    className="icon-btn"
                    title={t("common.duplicate")}
                    onClick={() => run(() => api.duplicateProfile(p.id), (r) => t("profiles.created", { name: r.name }))}
                  >
                    <Copy />
                  </button>
                  <button className="icon-btn" title={t("profiles.exportOne")} onClick={() => exportProfiles([p.id])}>
                    <Download />
                  </button>
                  <button
                    className="icon-btn danger"
                    title={list.length <= 1 ? t("profiles.lastProfile") : t("common.delete")}
                    disabled={list.length <= 1}
                    onClick={() => setToDelete(p)}
                  >
                    <Trash />
                  </button>
                </div>
              </div>
            </article>
          );
        })}
      </div>

      {draft && limits && (
        <Modal
          title={draft.id ? t("profiles.editTitle") : t("profiles.newTitle")}
          onClose={() => setDraft(null)}
          footer={
            <>
              <button className="btn" onClick={() => setDraft(null)}>
                {t("common.cancel")}
              </button>
              <button className="btn btn-primary" disabled={!draft.name.trim()} onClick={save}>
                <Save /> {draft.id ? t("common.save") : t("common.create")}
              </button>
            </>
          }
        >
          <Field label={t("common.name")}>
            <input
              className="input"
              value={draft.name}
              maxLength={limits.nameMaxChars}
              onChange={(e) => setDraft({ ...draft, name: e.target.value })}
            />
          </Field>
          <Field label={`${t("common.description")} (${t("common.optional")})`} hint={`${draft.description.length}/${limits.descriptionMaxChars}`}>
            <textarea
              className="textarea"
              value={draft.description}
              maxLength={limits.descriptionMaxChars}
              onChange={(e) => setDraft({ ...draft, description: e.target.value })}
            />
          </Field>
          {!draft.id && (
            <div className="field">
              <span className="field-label">{t("profiles.base")}</span>
              <Segmented
                block
                value={draft.base}
                onChange={(base) => setDraft({ ...draft, base })}
                options={[
                  { value: "current", label: t("profiles.baseCurrent") },
                  { value: "default", label: t("profiles.baseDefault") },
                ]}
              />
            </div>
          )}
        </Modal>
      )}

      {toDelete && (
        <ConfirmDialog
          title={t("profiles.deleteTitle", { name: toDelete.name })}
          body={t("profiles.deleteBody")}
          confirmLabel={t("common.delete")}
          danger
          onCancel={() => setToDelete(null)}
          onConfirm={async () => {
            await run(() => api.deleteProfile(toDelete.id));
            setToDelete(null);
          }}
        />
      )}

      {report && (
        <Modal
          title={t("profiles.importReport")}
          onClose={() => setReport(null)}
          footer={
            <button className="btn btn-primary" onClick={() => setReport(null)}>
              {t("common.close")}
            </button>
          }
        >
          <p>{t("profiles.imported", { profiles: report.profilesImported, presets: report.presetsImported })}</p>
          <p className="muted">{t("profiles.skipped", { count: report.skipped.length })}</p>
          <ul className="skipped-list">
            {report.skipped.map((s, i) => (
              <li key={i}>
                <strong>{s.name || "—"}</strong> ({s.kind}) — {s.reason}
              </li>
            ))}
          </ul>
        </Modal>
      )}
    </div>
  );
}
