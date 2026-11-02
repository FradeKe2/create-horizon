#!/bin/bash
# Installs Create for Horizon on macOS (Apple Silicon) into a Horizon launcher
# instance: the content mod (blocks, items) and the native mod (kinetics),
# then asks the launcher to build the native mod.
#
#   curl -fsSL https://raw.githubusercontent.com/FradeKe2/create-horizon/main/tools/install-mac.sh | bash
#
# Optional: INSTANCE="My instance" to pick the instance (default: the only
# one, or you are asked).
set -euo pipefail

REPO="FradeKe2/create-horizon"
HOME_DIR="$HOME/Library/Application Support/horizon"   # the game home: the default instance
INSTANCES="$HOME_DIR/instances"                          # instances made in the launcher
SRC="$HOME/create-horizon"

say() { printf '\n\033[1m%s\033[0m\n' "$*"; }
die() { printf '\n\033[31mErreur : %s\033[0m\n' "$*" >&2; exit 1; }
ask() { local a; read -r -p "$1" a </dev/tty; printf '%s' "$a"; }

[ "$(uname)" = "Darwin" ] || die "ce script est pour macOS."
[ "$(uname -m)" = "arm64" ] || die "le lanceur de Horizon ne compile les mods natifs que sur Mac Apple Silicon (M1, M2...)."

# 1. The launcher.
LAUNCHER=""
for app in "/Applications/Horizon.app" "$HOME/Applications/Horizon.app"; do
  if [ -x "$app/Contents/MacOS/horizon-launcher" ]; then LAUNCHER="$app/Contents/MacOS/horizon-launcher"; fi
done
[ -n "$LAUNCHER" ] || die "Horizon.app introuvable dans /Applications. Installe Horizon d'abord."

# 2. The instance: the game home (default) or one under instances/.
[ -f "$HOME_DIR/options.txt" ] || [ -d "$HOME_DIR" ] || die "dossier du jeu introuvable ($HOME_DIR). Ouvre le lanceur et prépare le jeu une fois."
FLAG=()
if [ -n "${INSTANCE:-}" ]; then
  GAME="$INSTANCES/$INSTANCE"; FLAG=(--instance "$INSTANCE")
else
  names=("(par défaut)")
  if [ -d "$INSTANCES" ]; then
    while IFS= read -r d; do names+=("$(basename "$d")"); done < <(find "$INSTANCES" -mindepth 1 -maxdepth 1 -type d | sort)
  fi
  choice=0
  if [ ${#names[@]} -gt 1 ]; then
    say "Instances :"
    for i in "${!names[@]}"; do echo "  $((i + 1)). ${names[$i]}"; done
    n="$(ask "Numéro de l'instance où tu joues : ")"
    choice=$((n - 1))
  fi
  if [ "$choice" -eq 0 ]; then
    GAME="$HOME_DIR"; INSTANCE="(par défaut)"
  else
    INSTANCE="${names[$choice]}"; GAME="$INSTANCES/$INSTANCE"; FLAG=(--instance "$INSTANCE")
  fi
fi
[ -d "$GAME" ] || die "instance introuvable : $GAME"
say "Instance : $INSTANCE ($GAME)"

# 3. The mod's files.
say "Téléchargement de Create pour Horizon..."
tmp="$(mktemp -d)"
curl -fsSL "https://github.com/$REPO/archive/refs/heads/main.tar.gz" -o "$tmp/src.tar.gz" \
  || die "téléchargement impossible (le dépôt $REPO doit être public)."
rm -rf "$SRC"
mkdir -p "$SRC"
tar -xzf "$tmp/src.tar.gz" -C "$SRC" --strip-components 1
rm -rf "$tmp"

# 4. The content mod: horizon-mods/create, turned on.
say "Installation du mod de contenu (blocs et objets)..."
mkdir -p "$GAME/horizon-mods"
rm -rf "$GAME/horizon-mods/create"
cp -R "$SRC/content" "$GAME/horizon-mods/create"
SWITCHES="$GAME/horizon-mods.txt"
touch "$SWITCHES"
grep -v '^mod\.create:' "$SWITCHES" > "$SWITCHES.new" || true
echo "mod.create:true" >> "$SWITCHES.new"
mv "$SWITCHES.new" "$SWITCHES"

# 5. The native mod: built by the launcher with its own Rust.
say "Le mod natif (la rotation) doit être compilé par le lanceur."
cat <<'EOF'
Les mods natifs ne sont pas isolés : leur code tourne avec tes droits.
Le code de ce mod est public : https://github.com/FradeKe2/create-horizon
La première compilation télécharge Rust et prend 20 à 30 minutes.
EOF
ok="$(ask "Compiler le mod natif maintenant ? (o/n) ")"
if [ "$ok" = "o" ] || [ "$ok" = "O" ] || [ "$ok" = "y" ]; then
  "$LAUNCHER" native-build "$SRC/mixins/create_horizon" ${FLAG[@]+"${FLAG[@]}"} --trust-native-code --allow-version-mismatch
  say "Terminé."
else
  say "Mod natif non compilé. Pour le faire plus tard :"
  echo "  \"$LAUNCHER\" native-build \"$SRC/mixins/create_horizon\" ${FLAG[*]:-} --trust-native-code"
fi

cat <<EOF

Ensuite :
  1. Lance l'instance « $INSTANCE » depuis le lanceur.
  2. Options > Mods : vérifie que « Create » est sur on (sinon active-le et redémarre).
  3. Inventaire créatif : onglet « Create », ou cherche « shaft ».
Si une ligne est en rouge dans Options > Mods, envoie-moi le texte affiché.
EOF
