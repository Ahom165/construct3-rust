# C3R — Construct 3 Rust Edition

**C3R** est une réimplémentation **open source et indépendante** d'un éditeur de jeux 2D de type Construct 3, écrite en **Rust**, avec un éditeur web complet, un **import universel par glisser-déposer** (n'importe quel fichier) et un **moteur HTML5 intégré** pour jouer aux projets.

> ⚠️ Projet indépendant : « Construct » et « Construct 3 » sont des marques de **Scirra Ltd**. C3R n'est ni affilié à, ni approuvé par Scirra, et ne contient **aucune portion de code original** de Construct. Les captures d'écran dans `docs/ui-reference/` restent la propriété de Scirra et servent uniquement de référence de design.

---

## 🚀 Démarrage sans compilation (recommandé)

1. Téléchargez l'archive de votre plateforme depuis la page **Releases** du dépôt :
   - `construct3-rust-linux-x64.zip`
   - `construct3-rust-windows-x64.zip`
   - `construct3-rust-macos-arm64.zip`
2. Décompressez, puis lancez le binaire :
   - Linux/macOS : `./c3-editor`
   - Windows : double-cliquez `c3-editor.exe`
3. Ouvrez **http://localhost:8080** dans votre navigateur.
4. Glissez-déposez `example.c3p` (inclus) dans l'éditeur, puis cliquez **▶ Play**.

Aucune compilation, aucune dépendance : un seul binaire autonome (l'interface web est embarquée dedans).

## ✨ Fonctionnalités

- **Import de n'importe quoi** : glissez-déposez (ou Ctrl+I) *n'importe quel fichier* dans l'éditeur :
  - `.c3p` / `.capx` → ouverture du projet (formats C3 tolérés, format natif `c3r` complet) ;
  - images (png, jpg, gif, webp, svg…) → ressource + bouton **Créer un Sprite** ;
  - audio (wav, ogg, mp3…) → utilisable dans l'action *Play sound* ;
  - vidéos, polices, JSON, txt, zip → stockés comme ressources (les zip/.c3p sont explorés et leurs médias extraits) ;
- **Éditeur web type Construct** : barre de menus, ruban, arbre de projet, vue Layout (sélection, déplacement, propriétés, zoom), éditeur d'*Event Sheets* visuel, panneau de propriétés ;
- **Moteur HTML5 intégré** : sprite, tiled background, texte, clavier, souris, audio + behaviors *Platform, Solid, 8Direction, Bullet, DestroyOutside, Fade, Sine, Wrap, Pin, ScrollTo, DragNDrop* ;
- **Event sheets** : conditions (au démarrage du layout, every tick, clavier, collision, variables, timers, clic…) et actions (créer/détruire/déplacer, textes, sons, variables, changer de layout…) ;
- **Export** : `.c3p` (ré-importable) ou **un seul fichier `.html`** jouable hors-ligne, double-cliquable ;
- **Serveur local Rust** : API REST (`/api/open`, `/api/import`, `/api/save`, `/api/export`), UI embarquée dans le binaire.

## 🕹️ Runtime natif bonus (`c3rs`)

En plus de l'éditeur web, le dépôt inclut un **runtime natif Rust** (`crates/c3rs-native`) qui ouvre les `.c3p` hors navigateur :

```bash
./c3rs run demo.c3p              # fenêtre de jeu (minifb)
./c3rs render demo.c3p out.png   # rendu headless (capture)
./c3rs inspect demo.c3p          # structure du projet
./c3rs validate demo.c3p         # vérification des références
```

## 🧪 Exemple livré

`demo/demo-platformer.c3p` — mini-plateforme originale : héros (behavior Platform), sols (Solid + tiled), pièces (Sine, +score, son), ennemis (Bullet + DestroyOutside), score texte mis à jour via variable globale, restart à la collision.

## 🛠️ Compilation (optionnelle)

```bash
cargo build --release
./target/release/c3-editor [port]   # défaut : 8080
```

## 🤖 CI/CD

`.github/workflows/build.yml` : build **linux-x64 / windows-x64 / macos-arm64** à chaque push, artefacts uploadés, et **Release GitHub automatique** (binaires + pack d'images de référence + démo) à chaque tag `v*`.

## 📋 Limitations honnêtes (v0.1)

- L'import `.c3p` gère les structures courantes (objets, layouts, instances, animations, textures inline data-URI, behaviors, event sheets) avec **normalisation tolérante** ; les projets C3 très complexes (familles, effets, behaviors exotiques, paramètres avancés) peuvent être partiellement importés — un avertissement listera ce qui a été ignoré.
- Le moteur couvre un sous-ensemble des plugins/behaviors de Construct 3 (liste ci-dessus).
- Éditeur mono-utilisateur local (pas de collaboration temps réel).

## 📄 Licence

MIT — voir `LICENSE`.
