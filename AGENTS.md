# Working on Oritech (native Horizon mod, SDK format)

> **Build slots (v2):** every build/test step goes through the campaign runner
> (`ports/tools/build-slot.sh` from the campaign root). It **blocks in the kernel** until a slot
> is free — never poll, never background a build, never end a turn holding a lease. Compile steps
> forward automatically to the second box (`AdLf-PC`): call
> `build-slot.sh run --job <j> --port ports/oritech --step test -- bash -c '<cmd>'` from the campaign
> root; the remote runner already `cd`s into the port, so **do not** repeat `cd ports/oritech` in the
> command and **do not** set `BUILD_LOCAL=1`. Never pass `--target-dir`. A timeout exits **75**.
> Read `../_shared/build-slots-usage.md` before running anything.

This repository is the port, not Horizon and not horizon-mixins. Never modify the original
checkout or player resources; do not commit `target/`, `.horizon/`, exported sources or Minecraft
assets.

**There is no staging step and no mixin plan.** One flat `.hmod` holds `horizon-mod.toml` (the only
identity, `api = "0.2"`, `[native]` pins), `Cargo.toml` (whose **only** dependency is
`horizon-mod-api` from the fetched `.horizon/sdk/`; the registry refuses a registry crate, R9),
`src/lib.rs` (the one `#[mod_api::horizon_mod] impl Hooks` and the block-entity Parts),
`src/support.rs` (the glue: declarations, saved state, ticks, the right-click, the energy move and
the commands), `src/runtime.rs` (the pure logic, unit-tested) and `src/json.rs` (a small,
dependency-free JSON reader: the recipe parsers read their files through it). `assets/` and `data/`
are the carried content and travel in the same file. The retired `.hnmod` layout is history:
`../_shared/evidence/oritech/retired-hnmod-0.1.0/`.

Keep every gameplay rule and number in `src/runtime.rs` with its test; the glue only reads the game,
calls `runtime`, and writes the result back. `src/runtime.rs`'s **slice-1 rules** are unchanged from
the retired port up to and including `after_consuming` — do not "tidy" them; the only edits since are
the recipe-JSON type (`crate::json::Value` instead of `serde_json::Value`, 0.2.14) and slice 2's
appended Powered Furnace rules (`FURNACE_*`, `Furnace`, `SmeltingRecipe`, `CORES`, `core_quality`,
`furnace_ticks`, `furnace_energy_usage`, and the shared `advance` step the two machines use), slice
3 the Industrial Chiller's (`COOLER_*`, `FluidInput`, `tank_matches`, `cooler_output_fits`,
`after_draining`, `CoolerRecipe`, `Cooler`, on the same `advance`) and slice 4 the Lava Generator's
(`LAVA_*`, `EnergyTarget`, `lava_output_targets`, `lavagen_ticks`, `LavaGenerator`, `LavaGenRecipe`,
`parse_fluid_input`), slice 5 the addon system's (`*_ADDON`, `ADDON_BLOCKS`, `AddonSettings`,
`Addons`, `Burst`, `energy_usage`, `processing_ticks`, …), slice 6 the Control Unit Addon's
(`REDSTONE_ADDON`, its neutral settings, `redstone_disables`) and slice 8 the Assembler's
(`ASSEMBLER_*`, `craft_plan`, `insert_slot`, `AssemblerRecipe`, `Assembler`), slice 9 the
Centrifuge's (`CENTRIFUGE_*`, `centrifuge_energy_usage`, `centrifuge_ticks`, `centrifuge_can_run`,
`centrifuge_output_fits`, `CentrifugeRecipe`, `Centrifuge`, `FLUID_ADDON`) and slice 10 the
Foundry's (`FOUNDRY_*`, `foundry_energy_usage`, `foundry_ticks`, `FoundryRecipe`, `Foundry`). A declaration or
readout test belongs next to its code in `src/support.rs`.

Validate, in this order, each inside a lease:

```sh
BUILD_LOCAL=1 ../tools/build-slot.sh run --job oritech-sdk --port ports/oritech --step test \
    -- bash -c 'cd ports/oritech && rm -f Cargo.lock && CARGO_INCREMENTAL=0 cargo test --offline'
BUILD_LOCAL=1 ../tools/build-slot.sh run --job oritech-sdk --port ports/oritech --step lint \
    -- bash -c 'cd ports/oritech && cargo fmt --check && CARGO_INCREMENTAL=0 cargo clippy --all-targets --offline -- -D warnings'
BUILD_LOCAL=1 ../tools/build-slot.sh run --job oritech-sdk --port ports/oritech --step build \
    -- bash -c 'cd ports/oritech && rm -f Cargo.lock && CARGO_INCREMENTAL=0 cargo horizon-mod build'
cp target/horizon-mod/oritech.hmod ./oritech.hmod
cargo horizon-mod check oritech.hmod && cargo horizon-mod fits oritech.hmod --build 0a875da
cargo horizon-mod fits oritech.hmod --build 0b14f8a        # a *different* build: the pin run is a tautology
```

The in-game leg (launcher 0.2.92, nightly `0a875da`) needs a tree `Cargo.lock` for `native-build`
while the package must not pack one, so:

```sh
cd ports/oritech
CARGO_HOME=$HOME/.cache/horizon-e2e/root/data/horizon-launcher/cargo cargo generate-lockfile --offline
../tools/launcher-game.sh build oritech     # game + control game, one global launcher lock
rm -f Cargo.lock
eval "$(../tools/launcher-game.sh env oritech)"
BUILD_LOCAL=1 ../tools/build-slot.sh run --job oritech-e2e --port ports/oritech --step e2e \
    -- bash ports/oritech/tools/e2e.sh      # PASS + EXIT:0; CONTROL=1, SELFTEST=1, SELFTEST_FURNACE=1
```

`cargo horizon-mod build` packs; it does not compile against the game. The native compile is
`horizon-launcher native-build . --instance <i> --trust-native-code`. The in-game leg needs the
launcher-built game carrying this package: do not claim an in-game result without one. The game is
reused when `horizon-mod.toml` is byte-identical, so **bump the version in the manifest before any e2e
meant to exercise new code**, then check the build really recompiled (`strings
~/.cache/horizon-e2e/<pin>/<mod>/lib<mod>.so | grep <a new string>`).

The package declares **no registry dependency**, so **no `Cargo.lock` is packed**: `rm -f Cargo.lock`
before every build/test, and `verify-sdk-package.py` fails a package that packs one. `cargo fmt
--check`, never `--all`: that formats the fetched SDK.
