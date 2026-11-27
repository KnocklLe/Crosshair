# Crosshair Studio — Architecture technique

Ce document décrit l'architecture de Crosshair Studio, les choix techniques et le
plan d'implémentation. Il est tenu à jour à chaque phase.

## 1. Vue d'ensemble

```
┌──────────────────────────── Processus Crosshair Studio ────────────────────────────┐
│                                                                                    │
│  ┌──────────── WebView2 (React + TS) ────────────┐                                 │
│  │ pages/  components/  hooks/  services/api.ts   │                                 │
│  │   • UI uniquement : aucune logique métier       │                                 │
│  │   • invoke() typé  ←→  commandes Tauri          │                                 │
│  │   • listen("app://state") pour la synchro       │                                 │
│  └──────────────────────┬─────────────────────────┘                                 │
│                         │ IPC Tauri (JSON + binaire pour les aperçus)               │
│  ┌──────────────────────▼──────────────── Rust (Tauri 2) ─────────────────────────┐ │
│  │ commands/   couche mince : validation des entrées → services                   │ │
│  │ crosshair/  modèle, validation, géométrie, RENDU (tiny-skia), presets          │ │
│  │ profiles/   service profils + import/export JSON versionné                     │ │
│  │ settings/   paramètres, démarrage Windows (registre HKCU\…\Run)                │ │
│  │ displays/   énumération des écrans (Win32), sélection, calcul du centre         │ │
│  │ shortcuts/  parsing / validation / enregistrement des raccourcis globaux       │ │
│  │ storage/    SQLite (rusqlite, bundled) + migrations (PRAGMA user_version)      │ │
│  │ overlay/    contrôleur (logique pure, testable) + moteur Win32                 │ │
│  └──────────────────────┬─────────────────────────────────────────────────────────┘ │
│                         │ canal mpsc + PostMessageW (réveil)                        │
│  ┌──────────────────────▼───────── Thread overlay dédié ──────────────────────────┐ │
│  │ Fenêtre Win32 native (pas une fenêtre Tauri) :                                  │ │
│  │ WS_POPUP + WS_EX_LAYERED | TRANSPARENT | TOPMOST | TOOLWINDOW | NOACTIVATE      │ │
│  │ Rendu ARGB prémultiplié → UpdateLayeredWindow (une seule fois par changement)   │ │
│  │ WM_DISPLAYCHANGE / WM_DPICHANGED → recalcul de la position                      │ │
│  │ SetWinEventHook(EVENT_SYSTEM_FOREGROUND, OUTOFCONTEXT) → mode « plein écran »   │ │
│  └─────────────────────────────────────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────────────────────────────────┘
```

### Principes

* **Un seul moteur de rendu.** Le viseur est dessiné par Rust (`crosshair::render`).
  L'overlay et les aperçus de l'interface utilisent exactement les mêmes pixels :
  l'aperçu est ce que l'overlay affichera. Les aperçus transitent en binaire brut
  (`tauri::ipc::Response`), sans encodage JSON.
* **Pas de boucle de rendu.** La fenêtre overlay n'est redessinée que lorsque la
  configuration, l'écran ou le DPI change. Au repos, le thread overlay est bloqué
  dans `GetMessageW` : 0 % CPU.
* **La fenêtre n'est jamais reconstruite.** Elle est créée une fois ; seuls sa taille,
  sa position et son bitmap changent.
* **L'état affiché est l'état confirmé.** Toute commande envoyée à l'overlay attend
  l'accusé de réception du thread overlay, qui renvoie l'état réel
  (`IsWindowVisible`, rectangle réel). L'UI n'affiche « actif » que sur cette base.
* **Aucune interaction avec les jeux.** Pas d'injection, pas de lecture mémoire, pas
  de hook dans un autre processus. Le hook de premier plan est `WINEVENT_OUTOFCONTEXT`
  (aucune DLL chargée ailleurs). Le plein écran exclusif est détecté via
  `SHQueryUserNotificationState` et signalé à l'utilisateur.

## 2. Modèle de données

```
CrosshairSpec { kind: "dot" | "crosshair", dot: DotConfig, crosshair: CrosshairConfig }

DotConfig       taille, couleur, opacité, forme (cercle/carré), contour, halo
CrosshairConfig forme (8 types), longueurs H/V, épaisseur, gap, couleur, opacité,
                rotation, contour, ombre, point central, cercle, split dynamique,
                éléments personnalisés (bras haut/bas/gauche/droite, cercle, point)

Profile  { id, name, description, spec, createdAt, updatedAt, sortIndex }
Preset   { id, name, category, spec, createdAt, favorite, builtin, lastUsedAt }
Settings { language, theme, autostart, tray, overlay{…}, performance{…}, dotSizes[] }
Shortcut { action, accelerator, enabled }
```

Un profil conserve **les deux** configurations (point et viseur) : `Ctrl+Shift+D`
bascule entre elles sans rien perdre.

### SQLite

| Table       | Contenu                                                         |
|-------------|-----------------------------------------------------------------|
| `settings`  | ligne unique, JSON versionné                                    |
| `profiles`  | profils (spec en JSON validé)                                   |
| `presets`   | bibliothèque (presets intégrés + personnalisés, favoris)        |
| `shortcuts` | raccourcis par action                                           |
| `recents`   | historique des viseurs appliqués (instantanés)                  |
| `app_state` | profil actif, visibilité de l'overlay                           |

Migrations : liste ordonnée de scripts SQL appliqués selon `PRAGMA user_version`,
dans une transaction. Base : `%APPDATA%\com.crosshairstudio.app\crosshair-studio.db`.

### Export JSON versionné

```json
{ "format": "crosshair-studio", "version": 1, "exportedAt": 1764200000000,
  "appVersion": "1.0.0", "profiles": [ … ], "presets": [ … ] }
```

L'import vérifie : taille du fichier (≤ 2 Mo), format, version (migration des
versions antérieures), nombre d'éléments, chaque champ (plages, couleurs `#RRGGBB`,
longueur des noms). Les éléments importés reçoivent de nouveaux identifiants.

## 3. Commandes et événements

Commandes (toutes validées côté Rust) : `get_bootstrap`, `get_dashboard`,
`set_overlay_visible`, `set_active_spec`, `render_preview`, CRUD profils, CRUD presets,
`apply_preset`, `export_*`/`import_*`, `get_settings`/`update_settings`,
`list_displays`, `set_shortcut`, `get_performance`, `reset_all_data`…

Événements émis par Rust :
`app://state` (profil actif, overlay, raccourci déclenché, tray),
`app://displays` (configuration d'écrans modifiée), `app://overlay-warning`.

## 4. Sécurité

* Aucun droit administrateur (installateur NSIS en mode `currentUser`).
* Capacités Tauri minimales : la WebView n'a accès qu'aux commandes de l'application
  et aux contrôles de sa propre fenêtre. Pas d'accès au système de fichiers depuis
  le JavaScript : les boîtes de dialogue et la lecture/écriture de fichiers sont
  faites en Rust.
* CSP stricte, aucune ressource distante (polices embarquées).
* Aucune télémétrie, aucune requête réseau.

## 5. Organisation du code

```
src/                          Interface (React + TypeScript)
  components/                 design system : contrôles, aperçu, modales, barre latérale…
  pages/                      Dashboard, Editor, CenterDot, Library, Profiles, Settings, About
  hooks/                      useLiveSpec (édition en direct), useResource, useI18n…
  services/                   api.ts (seul appel à invoke), store, events, preview, spec
  i18n/                       en.ts / fr.ts (clés vérifiées par TypeScript)
  styles/                     tokens, base, components, layout, pages
  types/                      miroir TypeScript des modèles Rust
  dev/                        backend simulé (développement navigateur uniquement)

src-tauri/src/                Backend (Rust)
  main.rs, lib.rs             point d'entrée, plugins, état, cycle de vie
  core.rs                     services (règles métier)
  commands/                   commandes Tauri (couche mince)
  crosshair/                  modèle, validation, géométrie, rendu, presets
  overlay/                    controller.rs (logique pure) + engine.rs (Win32)
  displays/                   énumération des écrans + placement
  profiles/                   profils + transfer.rs (import/export JSON)
  library/                    presets et historique
  settings/                   paramètres + autostart.rs (registre HKCU)
  shortcuts/                  parsing/validation + manager.rs (enregistrement OS)
  storage/                    SQLite + migrations
  tray.rs, events.rs, diagnostics.rs, error.rs, util.rs
```

### Flux d'une modification en direct

1. Un curseur change → `useLiveSpec.update()` met à jour le brouillon (aperçu immédiat).
2. Au plus une requête par frame (`requestAnimationFrame`), une seule en vol :
   `set_active_spec(spec, persist=false)` → le thread overlay coalesce les demandes,
   redessine une fois et renvoie l'état réel.
3. 350 ms après la dernière modification : `set_active_spec(spec, persist=true)` écrit en
   base et émet `app://state` (resynchronisation des autres vues, de la zone de notification).

## 6. Plan d'implémentation

| Phase | Contenu | État |
|------:|---------|------|
| 1  | Architecture, initialisation Tauri 2 + React + Vite | ✅ |
| 2  | Design system, layout, navigation                   | ✅ |
| 3  | Dashboard (données réelles)                         | ✅ |
| 4  | Moteur d'overlay Win32                              | ✅ |
| 5  | Center Dot                                          | ✅ |
| 6  | Crosshair Editor                                    | ✅ |
| 7  | Library (28 presets)                                | ✅ |
| 8  | Profiles + import/export                            | ✅ |
| 9  | Raccourcis globaux                                  | ✅ |
| 10 | Multi-écran                                         | ✅ |
| 11 | Settings                                            | ✅ |
| 12 | Tests (Rust + Vitest)                               | ✅ |
| 13 | Optimisation                                        | ✅ |
| 14 | Packaging (.exe NSIS, .msi WiX)                     | ✅ |
