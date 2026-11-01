# Create pour Horizon

Un portage en Rust du mod [Create](https://github.com/Creators-of-Create/Create) pour
[Horizon](https://github.com/horizon-rust/horizon), la réécriture de Minecraft Java 26.3 en Rust.

**État : étape 1, le cœur cinétique.** Le système de rotation et de stress de Create est porté
fidèlement. Il est livré avec les premiers blocs : arbre de transmission, roues dentées, boîte de
vitesses, moteur créatif, manivelle, compteurs de vitesse et de stress, presse et meule (pour
l'instant uniquement comme charges de stress). Le reste de Create (contraptions, tapis roulants,
trains, fluides, traitement des recettes...) viendra par étapes : voir [Feuille de route](#feuille-de-route).

## Ce qui est porté

Le dossier `mixins/create_horizon/kinetics` est une crate Rust sans dépendance. C'est une traduction
ligne à ligne des classes Java de Create (code sous licence MIT) :

| Create (Java) | Ici (Rust) |
|---|---|
| `RotationPropagator` : propagation, rapports d'engrenages, cassure si sens opposés, trop vite ou scintillement | `engine.rs` |
| `KineticNetwork`, `TorquePropagator` : capacité et stress par réseau, réseaux rechargés depuis la sauvegarde | `engine.rs` (`Network`, `Networks`) |
| `KineticBlockEntity` : tick, validation toutes les 60 ticks, surcharge, sauvegarde NBT avec les clés de Create | `engine.rs`, `state.rs` |
| `GeneratingKineticBlockEntity`, `CreativeMotorBlockEntity`, `HandCrankBlockEntity` | `engine.rs` |
| `IRotate.hasShaftTowards`, `ICogWheel`, valeurs de stress par défaut (`CStress`) | `blocks.rs` |
| Angles des rendus (`KineticBlockEntityRenderer`, décalage des dents), cadrans des compteurs | `render.rs` |

Les règles sont celles de Create :
- deux roues dentées qui s'engrènent tournent en sens opposés ;
- une grande roue fait tourner une petite roue deux fois plus vite ;
- une boîte de vitesses inverse le sens en ligne droite et transmet sur les côtés ;
- deux sources qui tournent en sens contraire cassent un bloc ;
- un réseau dont le stress dépasse la capacité s'arrête.

Les valeurs sont celles de Create : moteur créatif de 16 384 su/rpm, réglable jusqu'à 256 rpm ;
manivelle de 8 su/rpm à 32 rpm, qui tourne 10 ticks par clic ; presse de 8 su/rpm ; meule de 4 su/rpm.

## Structure

```
mixins/create_horizon/         le mod natif (bundle horizon-mixins)
  horizon-mixin.toml           12 hooks nommés de Horizon (NATIVE_HOOKS_API 80)
  runtime.rs                   la liaison : NBT, réseaux par dimension, rendu, clics, placement
  *.rs                         un petit adaptateur par hook
  kinetics/                    la crate create_kinetics (le moteur porté), avec ses tests
content/                       le mod de contenu (.hmod) : blocs, modèles, textures, recettes, butin
tools/build_content.py         génère content/ (textures dessinées par le script)
tools/stage.py                 applique le mod à une copie de Horizon pour le vérifier avec cargo check
```

Il y a deux moitiés, comme Horizon le prévoit :
- **le contenu** (`content/`, id `create`) déclare les blocs et les objets ;
- **le mod natif** (`create_horizon`) leur donne leur comportement : block entities qui tournent,
  ticks serveur, synchronisation client et rendu animé.

## Utilisation

Génère le contenu (il faut Pillow) :

```bash
python3 tools/build_content.py
```

Empaquette le mod de contenu avec l'outil de Horizon :

```bash
cargo install --git https://github.com/horizon-rust/horizon cargo-horizon-mod
```

```bash
cd content && cargo horizon-mod build
```

Le fichier `content/target/horizon-mod/create.hmod` va dans le dossier des mods de l'instance.

Pour le mod natif, ajoute le dossier `mixins/create_horizon` comme mod natif local dans le
lanceur de Horizon. Il le compile dans une version du jeu qui lui est propre
(voir `docs/native-mods.md` de Horizon).

Les deux moitiés sont nécessaires. Sans le mod natif, les blocs existent mais restent immobiles.

### En jeu

- **Moteur créatif** : clic droit pour augmenter la vitesse, accroupi + clic droit pour la baisser
  (par paliers de -256 à 256 rpm, affichés dans la barre d'action). Dans Create, on le règle à la
  molette.
- **Manivelle** : clic droit pour tourner, accroupi + clic droit pour tourner à l'envers.
- **Boîtier en andésite** : clic droit avec un alliage d'andésite sur une bûche ou un bois écorcé,
  comme dans Create.
- Recettes de Create : alliage, arbres de transmission par 8, roues dentées, boîte de vitesses, etc.

## Vérifications faites

```bash
cargo test --manifest-path mixins/create_horizon/kinetics/Cargo.toml
```

- **23 tests du moteur** : lignes d'arbres, engrenages, rapports grande/petite roue, boîte de
  vitesses, casse en sens opposés et en survitesse, prise de contrôle par une source plus rapide,
  surcharge, manivelle, rechargement d'une sauvegarde. Ils passent tous, et clippy ne signale rien.
- **Compilation contre Horizon** (commit `183b53f`) : le mod est appliqué avec `tools/stage.py`, puis
  `cargo check -p horizon-client` passe sans erreur ni avertissement.
- **Contenu** : le lecteur de contenu de Horizon (`horizon_mods::content`) accepte les 11 blocs,
  l'objet et l'onglet créatif.

**Pas encore fait : un essai en jeu.** Il faut une copie de Minecraft et un GPU, absents de
l'environnement où ce code a été écrit. Le comportement en monde réel (rendu, placement,
synchronisation) reste à confirmer en jouant.

## Limites connues

- Les blocs copient le modèle `minecraft:conduit` : il a une block entity, aucune face pleine et
  pas de menu. Les formes de collision de Horizon ne dépendent que des propriétés du modèle copié,
  donc l'arbre de transmission a une forme en croix sur les trois axes, quel que soit son axe.
- Le moteur créatif se règle au clic, pas à la molette (Horizon n'a pas encore de hook pour ça).
- La roue hydraulique, l'embrayage, le changeur de vitesse, le tapis roulant, etc. ne sont pas encore
  portés.
- Les textures et modèles sont originaux et simples : ceux de Create sont « tous droits réservés ».

## Feuille de route

1. **Cœur cinétique** (fait) : arbre, roues, boîte de vitesses, moteur, manivelle, compteurs, stress.
2. Sources : roue hydraulique, grande roue, moulin à vent (contraption), moteur à vapeur.
3. Transmission : embrayage, inverseur, arbre enfermé, chaîne, tapis roulant, changeur de vitesse.
4. Traitement : presse, mélangeur, meule, broyeur, ventilateur (recettes de Create).
5. Contraptions : piston mécanique, roulement, portique, chariots.
6. Logistique, fluides, trains.

## Licence

MIT. Voir `LICENSE` et `NOTICE.md` (code de Create sous MIT, ressources de Create non incluses).
