# Crosshair Studio

Application Windows 11 qui affiche un viseur ou un point central au-dessus de l'écran,
grâce à une fenêtre overlay native, transparente et traversable par la souris.

* **Center Dot** : taille (préréglages Micro/Small/Medium/Large modifiables), couleur,
  opacité, forme ronde/carrée, contour, halo.
* **Crosshair Editor** : 8 formes (croix, croix avec espace, dynamique fixe, cercle,
  cercle + point, point, croix + point, personnalisée), tailles H/V, épaisseur, gap,
  couleur, opacité, rotation, contour, ombre. Modifications appliquées en direct.
* **Bibliothèque** : 28 presets intégrés (6 catégories), favoris, presets personnels,
  appliquer / modifier / dupliquer / supprimer.
* **Profils** : création, édition, duplication, activation, import/export JSON versionné.
* **Raccourcis globaux** : `Ctrl+Maj+H` (afficher/masquer), `Ctrl+Maj+D` (point central),
  `Ctrl+Maj+P` (profil suivant), modifiables, conflits détectés.
* **Multi-écran**, DPI, changement de résolution, mode « plein écran uniquement »,
  zone de notification, démarrage avec Windows, thème sombre/clair, FR/EN.

L'aperçu de l'interface est produit **par le même moteur de rendu Rust que l'overlay** :
ce que vous voyez dans l'éditeur est exactement ce qui s'affiche à l'écran.

## Sécurité et anti-triche

Crosshair Studio n'injecte aucun code, ne charge aucune DLL dans un autre processus, ne lit
ni ne modifie la mémoire des jeux et n'intercepte aucune communication. L'overlay est une
simple fenêtre Windows « toujours au premier plan ». Le seul hook utilisé
(`SetWinEventHook(EVENT_SYSTEM_FOREGROUND, WINEVENT_OUTOFCONTEXT)`) est hors processus :
il signale seulement quelle fenêtre est au premier plan.

**Compatibilité** : bureau, applications fenêtrées, jeux en mode fenêtré et sans bordure.
Le **plein écran exclusif** n'est pas compatible (Windows dessine le jeu au-dessus de
toutes les fenêtres) : l'application le détecte et prévient l'utilisateur. Vérifiez les
règles de chaque jeu : certains interdisent les viseurs externes en compétition.

## Prérequis (développement)

| Outil | Version testée |
|---|---|
| Windows 11 | 10.0.26200 |
| Rust (stable, MSVC) | 1.100 |
| Visual Studio Build Tools (C++) | 2026 |
| Node.js | 26 |
| WebView2 Runtime | inclus dans Windows 11 |

## Commandes

```powershell
npm install                 # dépendances frontend
npm run app:dev             # lance l'application (Vite + Rust, rechargement à chaud)
npm run app:build           # binaire release + installateurs .exe et .msi
npm test                    # tests frontend (Vitest)
npm run test:rust           # tests Rust (dont l'overlay Win32 réel)
npm run typecheck           # vérification TypeScript
```

Les installateurs sont générés dans `src-tauri/target/release/bundle/` :

* `nsis/Crosshair Studio_1.0.0_x64-setup.exe` — installation **par utilisateur**, sans droits
  administrateur, raccourci bureau facultatif, désinstallation propre (supprime aussi
  l'entrée « Démarrer avec Windows »).
* `msi/Crosshair Studio_1.0.0_x64_en-US.msi` et `_fr-FR.msi` — pour un déploiement par
  machine (le format MSI demande les droits administrateur).

## Données

Tout reste en local, dans `%APPDATA%\com.crosshairstudio.app\crosshair-studio.db` (SQLite).
Le schéma est versionné (`PRAGMA user_version`) et migré automatiquement. Les exports
JSON portent un numéro de format et sont validés champ par champ à l'import.

## Tests

* **Rust (79 tests)** : géométrie et alignement au pixel, rendu (couleurs, opacité, contour,
  halo, ombre), validation, presets, sérialisation, import/export JSON, migrations SQLite,
  services (profils, presets, réinitialisation), raccourcis, écrans réels, et **fenêtre
  overlay Win32 réelle** (styles click-through/topmost, affichage confirmé, centrage exact,
  redimensionnement sans recréation, repli d'écran).
* **Frontend (24 tests)** : capture des raccourcis (indépendante de la disposition
  AZERTY/QWERTY), aides de configuration, traductions complètes FR/EN, décodage des
  aperçus binaires, store.
* Un fichier commun `src/services/default-spec.json` est vérifié des deux côtés : les
  valeurs par défaut Rust et TypeScript ne peuvent pas diverger.

En développement, ouvrir `http://localhost:1420` dans un navigateur (hors Tauri) charge un
backend simulé (`src/dev/mockBackend.ts`) pour inspecter l'interface. Il n'est jamais inclus
dans le build de production.

## Architecture

Voir [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).
