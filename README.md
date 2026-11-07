# c3rs — Construct Rust Edition

Runtime et éditeur **natifs en Rust** pour jeux **Construct 3 (.c3p)** — sans technologie web,
rendu **OpenGL** (compatible avec tous les GPU/pilotes, y compris VM et machines sans Vulkan).
Projet original indépendant, non affilié à Scirra Ltd.

> **v0.4.0 — correction de l'écran noir** : le rendu passe de wgpu (Vulkan/DX12) à
> OpenGL (glow). Si la fenêtre restait noire avec une ligne magenta, cette version corrige
> le problème.

## Ce que ça fait

- **Ouvre et exécute les projets .c3p** (archive ZIP au format projet C3 multi-fichiers :
  `project.c3proj`, `objectTypes/`, `layouts/`, `eventSheets/`, `images/`)
- **Éditeur natif** (`c3rs edit`) : arborescence projet, canvas de layout avec zoom/déplacement,
  sélection et placement d'instances, propriétés éditables, feuilles d'événements lues,
  bibliothèque de médias, aperçu jouable intégré (F5)
- **Import universel** : glissez-déposez (ou Importer…) **n'importe quel fichier** :
  - images PNG/JPEG/BMP/GIF/WebP → nouvel objet sprite plaçable
  - .c3p → ouverture de projet
  - audio WAV/OGG/MP3/FLAC/MOD…, polices TTF/OTF (appliquées à l'UI), archives, données → bibliothèque
- **Runtime** : comportements Platform, 8Direction, Bullet, Sine, Fade, Solid, ScrollTo,
  DestroyOutsideLayout, Wrap, Timer, Pin, **Tourelle (acquisition de cible, rotation, tir)**,
  **LigneDeMire (raycast contre les solides)**, **DéplacerVers**, Flash, Ancre ;
  interpréteur d'événements (conditions, actions, boucles for/for-each, sinon, groupes,
  includes) ; **évaluateur d'expressions C3** (opérateurs, `Objet.Propriété`, `Self`,
  `random()`, `floor()`, `true`/`false`, `&` concaténation…)
- Conditions clés : on-clicked (souris sur l'objet), on-shoot, has-los-to-object,
  is-on-screen, compare-boolean-eventvar, collisions, variables, clavier, timers…
- Actions clés : go-to-layout, restart-layout, spawn-another-object (**points d'image**,
  angle hérité), add-object-to-target, move-to-object, pin-to-object-properties, flash,
  reset-eventvar, create-object, destroy, variables (set/add/subtract/reset)…
- Textures : PNG RGBA/RGB/gris/**indexés (palette+tRNS)**, sprites animés multi-frames,
  TiledBackground via le champ `image`, points d'image, origines par frame
- Rendu logiciel intégré : sprites avec rotation/miroir, tiling, parallaxe, texte,
  caméra (ScrollTo + clamp), clignotement (Flash)
- **Enregistrement .c3p** : le projet (avec vos images importées) peut être réécrit en .c3p
- Rendu logiciel intégré : sprites avec rotation, tiling, parallaxe, texte, caméra

## Utilisation

```
c3rs                     ouvre l'éditeur (ou : c3rs edit [fichier.c3p])
c3rs run    jeu.c3p      exécute le jeu en fenêtre (flèches/WASD + Espace, Échap pour quitter)
c3rs render jeu.c3p out.png --ticks 240 --keys 39   capture headless (CI, tests)
c3rs render jeu.c3p out.png --click 430,414 --at 5 --ticks 150   simule un clic (tests boutons)
c3rs inspect jeu.c3p     structure du projet
c3rs validate jeu.c3p    vérifie les références
c3rs resave jeu.c3p out.c3p   re-sauvegarde (round-trip)
```

Dans l'éditeur : **F5** aperçu/arrêt · molette zoom · clic-milieu/droit déplacer · clic sélectionner
· Suppr supprimer · sélectionnez un objet dans l'arborescence puis cliquez dans le layout pour le placer.

## Dépendances d'exécution

- **Windows/macOS** : aucune (binaire autonome)
- **Linux** : bibliothèques graphiques standard, présentes sur toutes les distributions de bureau.
  Si nécessaire : `sudo apt install libxkbcommon-x11-0 libgl1`

## Compilation

```
cargo build --release      # binaire unique, aucune dépendance système imposée
```

## Limites connues

- sous-ensemble du runtime C3 (les événements/actions non supportés sont signalés dans la console)
- pas de lecture audio ni de familles complètes, pas de fonctions C3
- `wait` n'interrompt pas le bloc (les actions suivantes s'exécutent immédiatement)
- angles : le format C3 stocke des radians, convertis en degrés par c3rs

## Licence

MIT. « Construct 3 » est une marque de Scirra Ltd — ce projet n'est pas un produit Scirra.
