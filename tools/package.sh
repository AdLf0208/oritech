#!/usr/bin/env bash
# Package both artifacts, from the port's own sources:
#  - the `.hmod` in content/ via the pin's cargo-horizon-mod (build + check)
#  - a copy at mixins/oritech/data/oritech-content.hmod, which the bundle compiles in
#  - the source-only `.hnmod` in the port root (tools/package.py, deterministic bytes)
# Run inside a build slot (it compiles the packaging tool):
#   ports/tools/build-slot.sh run --job oritech-pack --port ports/oritech \
#       --step package -- env CARGO_INCREMENTAL=0 tools/package.sh
set -euo pipefail

PORT=$(cd "$(dirname "$0")/.." && pwd)
HORIZON=${HORIZON:-${CAMPAIGN_ROOT:-$(cd "$(dirname "$0")/../../.." && pwd)}/ports/_shared/pins/horizon-1d63e0a}
LOGS=$PORT/target/ladder
mkdir -p "$LOGS"
cd "$PORT"

JAR=$PORT/../_shared/licence-cache/oritech-neoforge-1.21.1-1.2.13.jar
if [ -f "$JAR" ]; then
    python3 tools/carry_content.py --check
else
    echo "carry check skipped: $JAR is not on this machine"
fi
export CARGO_TARGET_DIR=$PORT/target/carry-tools
(cd content && cargo run --locked --offline --quiet \
    --manifest-path "$HORIZON/crates/cargo-horizon-mod/Cargo.toml" -- horizon-mod build) \
    2>&1 | tee "$LOGS/package-hmod.log"
cp content/target/horizon-mod/oritech.hmod content/oritech.hmod
(cd content && cargo run --locked --offline --quiet \
    --manifest-path "$HORIZON/crates/cargo-horizon-mod/Cargo.toml" -- horizon-mod check oritech.hmod) \
    2>&1 | tee -a "$LOGS/package-hmod.log"

# The bundle's data/ carries the built content; package.py refuses a stale copy.
mkdir -p mixins/oritech/data
cp content/oritech.hmod mixins/oritech/data/oritech-content.hmod
python3 tools/package.py | tee "$LOGS/package-hnmod.log"
cp target/oritech-0.1.0.hnmod oritech.hnmod
(cd content && cargo run --locked --offline --quiet \
    --manifest-path "$HORIZON/crates/cargo-horizon-mod/Cargo.toml" -- horizon-mod check ../oritech.hnmod --kind native-mod) \
    2>&1 | tee -a "$LOGS/package-hnmod.log"

echo "== artifacts =="
for f in content/oritech.hmod oritech.hnmod; do
    [ -f "$f" ] || { echo "missing $f"; exit 1; }
    size=$(stat -c %s "$f")
    sha=$(sha256sum "$f" | cut -d' ' -f1)
    entries=$(python3 -c "import zipfile,sys; print(len(zipfile.ZipFile(sys.argv[1]).namelist()))" "$f")
    echo "$f size=$size sha256=$sha entries=$entries"
done
