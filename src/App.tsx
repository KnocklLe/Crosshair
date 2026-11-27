import { useEffect } from "react";
import logo from "./assets/logo.svg";
import { Sidebar } from "./components/Sidebar";
import { TitleBar } from "./components/TitleBar";
import { Toasts } from "./components/Toasts";
import { getLanguage, resolveLanguage, setLanguage, translate } from "./i18n";
import { AboutPage } from "./pages/AboutPage";
import { CenterDotPage } from "./pages/CenterDotPage";
import { DashboardPage } from "./pages/DashboardPage";
import { EditorPage } from "./pages/EditorPage";
import { LibraryPage } from "./pages/LibraryPage";
import { ProfilesPage } from "./pages/ProfilesPage";
import { SettingsPage } from "./pages/SettingsPage";
import { api } from "./services/api";
import { subscribeBackendEvents } from "./services/events";
import { store, useStore } from "./services/store";
import type { PageId } from "./types/models";

const PAGES: Record<PageId, () => React.JSX.Element | null> = {
  dashboard: DashboardPage,
  editor: EditorPage,
  dot: CenterDotPage,
  library: LibraryPage,
  profiles: ProfilesPage,
  settings: SettingsPage,
  about: AboutPage,
};

let booted = false;

async function boot() {
  // React StrictMode runs effects twice in development: start only once.
  if (booted) return;
  booted = true;
  try {
    // Listen first so no event emitted during startup is missed.
    await subscribeBackendEvents();
    const b = await api.getBootstrap();
    const lang = resolveLanguage(b.settings.language);
    setLanguage(lang);
    store.set({
      ready: true,
      app: b.app,
      settings: b.settings,
      limits: b.limits,
      overlay: b.state.overlay,
      activeProfile: b.state.activeProfile,
      profileCount: b.state.profileCount,
      shortcuts: b.shortcuts,
      displays: b.displays,
    });
    // Shows the window once the interface is painted (no white flash).
    requestAnimationFrame(() => api.appReady(lang).catch(() => undefined));
  } catch (e) {
    store.set({ fatalError: e instanceof Error ? e.message : String(e) });
    api.appReady("en").catch(() => undefined);
  }
}

function useThemeAndMode() {
  const theme = useStore((s) => s.settings?.theme ?? "dark");
  const lite = useStore((s) => s.settings?.performance.liteMode ?? false);

  useEffect(() => {
    const root = document.documentElement;
    if (theme !== "system") {
      root.dataset.theme = theme;
      return;
    }
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const apply = () => (root.dataset.theme = mq.matches ? "dark" : "light");
    apply();
    mq.addEventListener("change", apply);
    return () => mq.removeEventListener("change", apply);
  }, [theme]);

  useEffect(() => {
    document.documentElement.dataset.lite = String(lite);
  }, [lite]);
}

export function App() {
  const ready = useStore((s) => s.ready);
  const fatal = useStore((s) => s.fatalError);
  const page = useStore((s) => s.page);
  const languageSetting = useStore((s) => s.settings?.language);

  // Resolve the language before children render so they use it immediately.
  const lang = resolveLanguage(languageSetting);
  if (ready && getLanguage() !== lang) setLanguage(lang);

  useEffect(() => {
    if (ready) api.setUiLanguage(lang).catch(() => undefined);
  }, [ready, lang]);

  useEffect(() => {
    boot();
    // The WebView context menu ("Reload", "Inspect"…) is not useful here.
    const onMenu = (e: MouseEvent) => {
      const el = e.target as HTMLElement;
      if (!el.closest("input, textarea")) e.preventDefault();
    };
    window.addEventListener("contextmenu", onMenu);
    return () => window.removeEventListener("contextmenu", onMenu);
  }, []);

  useThemeAndMode();

  if (fatal) {
    return (
      <div className="splash">
        <img src={logo} alt="" />
        <h2>{translate("app.fatal")}</h2>
        <pre>{fatal}</pre>
      </div>
    );
  }
  if (!ready) {
    return (
      <div className="splash">
        <img src={logo} alt="" />
        <span>{translate("app.loading")}</span>
      </div>
    );
  }

  const Page = PAGES[page];
  return (
    <div className="app">
      <TitleBar />
      <div className="shell">
        <Sidebar />
        <main className="main" key={page}>
          <Page />
        </main>
      </div>
      <Toasts />
    </div>
  );
}
