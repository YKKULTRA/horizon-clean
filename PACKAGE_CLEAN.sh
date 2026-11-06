#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$(readlink -f "$0")")"
root="$PWD"
out="${1:-$(dirname "$PWD")/dist/horizon-clean}"
if [[ -e "$out" ]]; then
    printf 'Refusing to overwrite %s; choose a fresh output folder.\n' "$out" >&2
    exit 1
fi
mkdir -p "$out/game-build/crates" "$out/game-build/mods/horizon/src/test"
cp launcher/target/release/horizon-launcher "$out/"
for bin in horizon-client horizon-ctl horizon-prepare horizon-server; do
    cp "target/fast/$bin" "$out/game-build/"
done
# Same original preparer sources shipped by upstream's xtask, without Mojang files.
cp -a tools "$out/game-build/"
cp -a crates/horizon-prepare "$out/game-build/crates/"
cp -a mods/horizon/src/test/java "$out/game-build/mods/horizon/src/test/"
cp research-lock.json LICENSE THIRD_PARTY_NOTICES.md "$out/game-build/"
cp FORK.md "$out/README.md"
base="4aa6854a886183440256f4f4a4a9d1ed853227a1"
printf 'Source base: %s; clean fork %s\n' "$base" "$(git rev-parse HEAD)" > "$out/game-build/.installed"
cp RUN_CLEAN.sh "$out/run-clean"
chmod 755 "$out/run-clean"
# Every change from the upstream base: this fork's commits, then any uncommitted edits.
git diff --binary "$base" > "$out/source-changes.patch"
# Include files not yet added to Git as well.
while IFS= read -r -d '' file; do
    rc=0
    git diff --no-index --binary -- /dev/null "$file" >> "$out/source-changes.patch" || rc=$?
    if [[ "$rc" -gt 1 ]]; then exit "$rc"; fi
done < <(git ls-files --others --exclude-standard -z)
(cd "$out" && sha256sum horizon-launcher game-build/horizon-{client,ctl,prepare,server} > SHA256SUMS)
printf 'Packaged %s\n' "$out"
