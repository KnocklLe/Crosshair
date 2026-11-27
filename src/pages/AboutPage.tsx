import logo from "../assets/logo.svg";
import { Card } from "../components/PageHeader";
import { CircleCheck, Database, Monitor, ShieldCheck, TriangleExclamationPoint, Zap } from "../components/icons";
import { useI18n } from "../hooks/useI18n";
import { useStore } from "../services/store";

const TECH = ["Rust", "Tauri 2", "Win32 API", "tiny-skia", "SQLite", "React 19", "TypeScript", "Vite", "Lucide"];

export function AboutPage() {
  const { t } = useI18n();
  const app = useStore((s) => s.app);
  return (
    <div className="page">
      <section className="card about-hero">
        <img src={logo} alt="" />
        <div className="stack" style={{ gap: 6 }}>
          <h1 style={{ fontSize: 26, fontWeight: 750, letterSpacing: "-0.02em" }}>Crosshair Studio</h1>
          <span className="muted">
            {t("about.subtitle", { version: app?.version ?? "" })} · Tauri {app?.tauriVersion}
          </span>
          <p style={{ maxWidth: 640 }}>{t("about.description")}</p>
        </div>
      </section>

      <div className="about-grid">
        <Card title={t("about.safety")} icon={<ShieldCheck />}>
          <ul className="check-list">
            {(["about.safety1", "about.safety2", "about.safety3", "about.safety4"] as const).map((k) => (
              <li key={k} className="ok">
                <CircleCheck />
                {t(k)}
              </li>
            ))}
          </ul>
        </Card>
        <Card title={t("about.compat")} icon={<Monitor />}>
          <ul className="check-list">
            <li className="ok">
              <CircleCheck />
              {t("about.compatOk")}
            </li>
            <li className="no">
              <TriangleExclamationPoint />
              {t("about.compatNo")}
            </li>
            <li className="no">
              <TriangleExclamationPoint />
              {t("about.compatAc")}
            </li>
          </ul>
        </Card>
        <Card title={t("about.tech")} icon={<Zap />}>
          <div className="tech-list">
            {TECH.map((x) => (
              <span key={x} className="badge badge-accent">
                {x}
              </span>
            ))}
          </div>
        </Card>
        <Card title={t("about.data")} icon={<Database />}>
          <p className="muted" style={{ marginBottom: 8 }}>
            {t("about.dataBody")}
          </p>
          <div className="path-box">
            <code>{app?.dbPath}</code>
          </div>
        </Card>
      </div>
    </div>
  );
}
