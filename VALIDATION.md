# Oritech — validation record

The commands that produce the evidence, and their verdicts, newest first. Every Rust, package and
in-game step ran inside a `ports/tools/build-slot.sh` lease (`BUILD_LOCAL=1`, `CARGO_INCREMENTAL=0`,
no `--target-dir`); the launcher build runs outside a slot because it has its own global build lock
and its own cache. Logs: `../_shared/evidence/oritech/` and `../_shared/evidence/oritech-e2e/`.

## 0.2.12 — slice 10, the Foundry (pin `0a875da`, 2026-11-23)

| # | Command (from the campaign root) | Verdict | Log |
|---|---|---|---|
| 1 | `build-slot.sh run --step test -- bash -c 'cd ports/oritech && cargo test --offline'` with the new bodies `todo!()` | **126 passed, 5 failed** — `not yet implemented` (four) and the addon-slot table (one) | `slice10-test-red.log` |
| 2 | six targeted mutations of the declarations/buffer/tank/state/line/core table, then `cargo test --offline foundry` (`tools/slice10-glue-counterfactual.py`) | **9 passed, 3 failed** — the declarations test catches three of the six facts; the file restored byte-identically (`6a79f38c…`) | `slice10-glue-red-counterfactual.log` |
| 3 | same, implemented | **138 passed, 0 failed**; the run regenerated `sdk-test.log` at 0.2.12 | `sdk-test.log` |
| 4 | `build-slot.sh run --step lint -- bash -c 'cargo fmt --check && cargo clippy --all-targets --offline -- -D warnings'` | exit 0, **0 warnings** | `slice10-fmt-clippy.log` |
| 5 | `cd ports/oritech && cargo horizon-mod build` twice, the artifact deleted between | **246,112 B**, sha256 `f2da80ce2a2e7487c473c7c561cd47aa2900d383c00b1bfe3059695585d1ec81`, `cmp` identical, **298 entries incl. `Cargo.lock`** (8,841 B) | `slice10-build.log` |
| 6 | `cargo horizon-mod check oritech.hmod` | exit 0, `oritech 0.2.12 (native-mod, format native-1/0a875da…/e59da90…, for 26.3)`, `246112 bytes, SHA-256 f2da80ce…` | `slice10-check-fits.log` |
| 7 | `cargo horizon-mod fits oritech.hmod --build 0a875da` | `0a875da  made for`, exit 0 | `slice10-check-fits.log` |
| 8 | `cargo horizon-mod fits oritech.hmod --build 0b14f8a` | `0b14f8a  fits`, exit 0 (a different build — the pin run alone is a tautology) | `slice10-check-fits.log` |
| 9 | `python3 ports/tools/verify-sdk-package.py oritech` | **1/1** | stdout |
| 10 | `python3 tools/carry_content.py --check` | `288 files, 0 differ` | stdout |
| 11 | `cargo test --offline every_carried_recipe_type` with `register_recipe_type(support::FOUNDRY_RECIPE)` removed | **1 failed** — `src/lib.rs does not run register_recipe_type(support::FOUNDRY_RECIPE)`; the pin for the load-time defect the first e2e run found | `slice10-registration-counterfactual.log` |
| 12 | `ports/tools/launcher-game.sh build oritech` (manifest 0.2.12; the stale `native-0a875da-40c13c1713ad` version folder and the export were removed first, because the launcher reuses a build by manifest alone) | `native-0a875da-9cd80bb5d51a`; `strings liboritech.so` shows `Foundry at `, `oritech:foundry` and `is not an ingredient of an alloying recipe` | — |
| 13 | `eval "$(ports/tools/launcher-game.sh env oritech)"; build-slot.sh run --step e2e -- bash ports/oritech/tools/e2e.sh` | **PASS** + `EXIT:0` (FA–FE the foundry, the foundry block's own craft in AM, its loot table in W) | `e2e-slice10.log` |
| 14 | same with `CONTROL=1` (the in-run differential) | **PASS** + `EXIT:0`; the package-less build has no steel ingot and the foundry id stays air | `e2e-slice10-differential.log` |
| 15 | same with `GAME=$CONTROL_GAME` (the whole scenario on the package-less build) | **EXIT:1** — `the generator did not place` | `e2e-slice10-control.log` |
| 16 | same with `SELFTEST=1` | **EXIT:1** — `91 ticks spent 2912 FE, expected 2944` | `e2e-slice10-selftest.log` |
| 17 | same with `SELFTEST_FOUNDRY=1` (the output count flipped to 2) | **EXIT:1** — `OUT0 is 'oritech:steel_ingot x1', expected 'oritech:steel_ingot x2'` | `e2e-slice10-selftest-foundry.log` |
| 18 | `python3 ports/tools/check-e2e-logs.py oritech-e2e --strict` | 47 logs, 47 ok, 0 bad | stdout |
| 19 | `python3 ports/tools/check-coverage.py oritech` | exit 0, `missing_families=0 missing=0` | stdout |
| 20 | `python3 ports/tools/check-skills.py --notes ports/oritech` | PASS | stdout |

The **first** run of 0.2.12 is worth keeping: it failed at load with
`Horizon crashed: recipe data/oritech/recipe/foundry/alloy/adamant.json, unregistered recipe
serializer oritech:foundry`. `check`, both `fits`, 137 unit tests and the carry check were green
while the package could not open a world; the fix is `register_recipe_type` + the raw serializer in
`src/lib.rs`, and step 11 is the counterfactual that proves the new pin catches it. The in-game
assertions and what each can fail on are in `notes.md` → *Slice 10*.

## 0.2.11 — slice 9, the Centrifuge and the Fluid Addon (pin `0a875da`, 2026-11-23)

| # | Command (from the campaign root) | Verdict | Log |
|---|---|---|---|
| 1 | `build-slot.sh run --step test -- bash -c 'cd ports/oritech && cargo test --offline'` with the new bodies `todo!()` | **109 passed, 8 failed** — every failure `panicked at src/runtime.rs:…: not yet implemented` | `slice9-test-red.log` |
| 2 | six targeted mutations of the declarations/tank/state/line/core table, then `cargo test --offline centrifuge` (`tools/slice9-glue-counterfactual.py`) | **10 passed, 3 failed** — the declarations test catches three of the six facts; the file restored byte-identically | `slice9-glue-red-counterfactual.log` |
| 3 | same, implemented | **122 passed, 0 failed**; the run regenerated `sdk-test.log` at 0.2.11 | `slice9-test-green.log` (= `sdk-test.log`) |
| 4 | `build-slot.sh run --step lint -- bash -c 'cargo fmt --check && cargo clippy --all-targets --offline -- -D warnings'` | exit 0, **0 warnings** | `slice9-fmt.log`, `slice9-clippy.log` |
| 5 | `cd ports/oritech && cargo horizon-mod build` twice, the artifact deleted between | **233,718 B**, sha256 `0fedc97c45d0e388edef0d251f91f14543a9fe22ea70a74fec375d94d693c1eb`, `cmp` identical, **286 entries incl. `Cargo.lock`** | `slice9-build.log` |
| 6 | `cargo horizon-mod check oritech.hmod` | exit 0, `oritech 0.2.11 (native-mod, format native-1/0a875da…/e59da90…, for 26.3)`, `233718 bytes, SHA-256 0fedc97c…` | `sdk-check.log` |
| 7 | `cargo horizon-mod fits oritech.hmod --build 0a875da` | `0a875da  made for`, exit 0 | `sdk-fits.log` |
| 8 | `cargo horizon-mod fits oritech.hmod --build 0b14f8a` | `0b14f8a  fits`, exit 0 (a different build — the pin run alone is a tautology) | `sdk-fits.log` |
| 9 | `python3 ports/tools/verify-sdk-package.py oritech` | **1/1** | stdout |
| 10 | `python3 tools/carry_content.py --check` | `276 files, 0 differ` | stdout |
| 11 | `ports/tools/launcher-game.sh build oritech` (manifest bumped to 0.2.11 so it recompiles) | `Compiling the mods against the prebuilt game`; build `native-0a875da-0ad952ad006f`; `strings liboritech.so` shows `Centrifuge at `, `oritech:centrifuge_fluid` and `is not an ingredient of a centrifuging recipe`; the export says `version = "0.2.11"` | `slice9-launcher-build.log` |
| 12 | `eval "$(ports/tools/launcher-game.sh env oritech)"; build-slot.sh run --step e2e -- bash ports/oritech/tools/e2e.sh` | **PASS** + `EXIT:0` (CA–CE the centrifuge, the coal recipe in O, the carbon core3 in AM) | `e2e-slice9.log` |
| 13 | same with `CONTROL=1` (the in-run differential) | **PASS** + `EXIT:0`; the package-less build has no coal dust and no carbon fibre, and the centrifuge and Fluid Addon ids stay air | `e2e-slice9-differential.log` |
| 14 | same with `GAME=$CONTROL_GAME` (the whole scenario on the package-less build) | **EXIT:1** — `the generator did not place` | `e2e-slice9-control.log` |
| 15 | same with `SELFTEST=1` | **EXIT:1** — `91 ticks spent 2912 FE, expected 2944` | `e2e-slice9-selftest.log` |
| 16 | same with `SELFTEST_CENTRIFUGE=1` (the output count flipped to 2) | **EXIT:1** — `OUT0 is 'oritech:carbon_fibre_strands x1', expected 'oritech:carbon_fibre_strands x2'` | `e2e-slice9-selftest-centrifuge.log` |
| 17 | `python3 ports/tools/check-e2e-logs.py oritech-e2e --strict` | 42 logs, 42 ok, 0 bad | stdout |
| 18 | `python3 ports/tools/check-coverage.py oritech` | exit 0, `missing_families=0 missing=0` | stdout |
| 19 | `python3 ports/tools/check-skills.py --notes ports/oritech` | PASS | stdout |

The in-game assertions and what each can fail on are listed in `notes.md` → *Slice 9* → *Verification*
and *In-game transcript*. The first CD run found a scenario error, not a port defect: the clay file's
recipe **id** is `oritech:centrifuge/fluid/clay` while its **type** is `oritech:centrifuge_fluid`.

## 0.2.10 — slice 8, the Assembler (pin `0a875da`, 2026-11-22)

| # | Command (from the campaign root) | Verdict | Log |
|---|---|---|---|
| 1 | `build-slot.sh run --step test -- bash -c 'cd ports/oritech && cargo test --offline'` with the six new bodies `todo!()` | **98 passed, 6 failed** — every failure `panicked at src/runtime.rs:…: not yet implemented` | `slice8-test-red.log` |
| 2 | five targeted mutations of the declarations/saved state/line/core table, then `cargo test --offline` (`tools/slice8-glue-counterfactual.py`) | **104 passed, 4 failed** — one per pinned glue fact; the file restored byte-identically | `slice8-glue-red-counterfactual.log` |
| 3 | same, implemented | **108 passed, 0 failed** | `slice8-test-green.log` (= `sdk-test.log`) |
| 4 | `build-slot.sh run --step lint -- bash -c 'cargo fmt --check && cargo clippy --all-targets --offline -- -D warnings'` | exit 0, **0 warnings** | `slice8-clippy.log` (= `sdk-clippy.log`) |
| 5 | `cd ports/oritech && cargo horizon-mod build` twice, the artifact deleted between | **212,959 B**, sha256 `8c39bc526b3dbb0457d4c4f3ea71126214c3267acc51e2f328990a82b107bb6e`, `cmp` identical, **258 entries incl. `Cargo.lock`** (rebuilt after the `README.md` edit; the e2e archive `19c28d50…`/212,651 B has the same entries — see `notes.md`) | `slice8-build.log`, `slice8-build-second.log` |
| 6 | `cargo horizon-mod check oritech.hmod` | exit 0, `oritech 0.2.10 (native-mod, format native-1/0a875da…/e59da90…, for 26.3)`, `212959 bytes, SHA-256 8c39bc52…` | `sdk-check.log` |
| 7 | `cargo horizon-mod fits oritech.hmod --build 0a875da` | `0a875da  made for`, exit 0 | `sdk-check.log` |
| 8 | `cargo horizon-mod fits oritech.hmod --build 0b14f8a` | `0b14f8a  fits`, exit 0 (a different build — the pin run alone is a tautology) | `sdk-fits.log` |
| 9 | `python3 ports/tools/verify-sdk-package.py oritech` | **1/1** | stdout |
| 10 | `python3 tools/carry_content.py --check` | `248 files, 0 differ` | stdout |
| 11 | `ports/tools/launcher-game.sh build oritech` (manifest bumped to 0.2.10 so it recompiles) | `Compiling the mods against the prebuilt game`; build `native-0a875da-d3f24692b8d1`; `strings liboritech.so` shows `Assembler at `, `oritech:assembler` and `is not an ingredient of an assembling recipe`; the export carries the five `data/oritech/recipe/assembler/` files | — |
| 12 | `eval "$(ports/tools/launcher-game.sh env oritech)"; build-slot.sh run --step e2e -- bash ports/oritech/tools/e2e.sh` | **PASS** + `EXIT:0` (AO–AS the assembler, AT the other four recipes, AM 19 crafts) | `e2e-slice8.log` |
| 13 | same with `CONTROL=1` (the in-run differential) | **PASS** + `EXIT:0`; the package-less build has no generator item, no motor and no adamant ingot, and every package block id stays air, the assembler included | `e2e-slice8-differential.log` |
| 14 | same with `GAME=$CONTROL_GAME` (the whole scenario on the package-less build) | **EXIT:1** — `the generator did not place` | `e2e-slice8-control.log` |
| 15 | same with `SELFTEST=1` | **EXIT:1** — `89 ticks spent 2848 FE, expected 2880` | `e2e-slice8-selftest.log` |
| 16 | same with `SELFTEST_ASSEMBLER=1` (the output count flipped to 3) | **EXIT:1** — `OUT0 is 'oritech:motor x2', expected 'oritech:motor x3'` | `e2e-slice8-selftest-assembler.log` |
| 17 | `python3 ports/tools/check-e2e-logs.py oritech-e2e --strict` | 37 logs, 37 ok, 0 bad | stdout |
| 18 | `python3 ports/tools/check-coverage.py oritech` | exit 0, `missing_families=0 missing=0` | stdout |
| 19 | `python3 ports/tools/check-skills.py --notes ports/oritech` | PASS | stdout |

The in-game assertions and what each can fail on are listed in `notes.md` → *Slice 8* → *Verification*
and *In-game transcript*. Two scenario errors were found and fixed by the leg itself: the assembler's own
craft pattern is `sbs` (two motors, one plating), and an empty input slot wins over a matching stack, so
the "two steel in one slot" leg needs no empty slot left.

## 0.2.9 — slice 7, the material chain and the machines' crafting recipes (pin `0a875da`, 2026-11-22)

| # | Command (from the campaign root) | Verdict | Log |
|---|---|---|---|
| 1 | `build-slot.sh run --step test -- bash -c 'cd ports/oritech && cargo test --offline carried_recipe'` with the slice's recipe files moved aside | **0 passed, 2 failed** — `the slice carries seventeen crafting recipes and one smelting recipe: 0 found ([])` and `no carried recipe to resolve`; the two tests are the data's red proof | `slice7-test-red.log` |
| 2 | same, implemented | **98 passed, 0 failed** | `slice7-test-green.log` (= `sdk-test.log`) |
| 3 | `build-slot.sh run --step lint -- bash -c 'cargo fmt --check && cargo clippy --all-targets --offline -- -D warnings'` | exit 0, **0 warnings** | `slice7-clippy.log` (= `sdk-clippy.log`) |
| 4 | `cd ports/oritech && cargo horizon-mod build` twice, the artifact deleted between | **195,181 B**, sha256 `8935cae7bb3f34c4564bbe83fc81869fcb33ca97a3205b6aff8a014eb4133276`, `cmp` identical, 240 entries **incl. `Cargo.lock`** | `slice7-build.log`, `slice7-build-second.log` |
| 5 | `cargo horizon-mod check oritech.hmod` | exit 0, `oritech 0.2.9 (native-mod, format native-1/0a875da…/e59da90…, for 26.3)` | `sdk-check.log` |
| 6 | `cargo horizon-mod fits oritech.hmod --build 0a875da` | `0a875da  made for`, exit 0 | `sdk-check.log`, `sdk-fits.log` |
| 7 | `cargo horizon-mod fits oritech.hmod --build 0b14f8a` | `0b14f8a  fits`, exit 0 (a different build — the pin run alone is a tautology) | `sdk-check.log`, `sdk-fits.log` |
| 8 | `python3 ports/tools/verify-sdk-package.py oritech` | **1/1** | stdout |
| 9 | `python3 tools/carry_content.py --check` | `230 files, 0 differ` | stdout |
| 10 | `ports/tools/launcher-game.sh build oritech` (manifest bumped to 0.2.9 so it recompiles) | `Compiling the mods against the prebuilt game`; build `native-0a875da-e42664391ac2`; the exported `horizon-native-mods/oritech/horizon-mod.toml` says `version = "0.2.9"` and carries `data/oritech/recipe/crafting/` (this slice adds **no code string** to `liboritech.so`, so `strings` is not its proof — the manifest version and the packed data are) | — |
| 11 | `eval "$(ports/tools/launcher-game.sh env oritech)"; build-slot.sh run --step e2e -- bash ports/oritech/tools/e2e.sh` | **PASS** + `EXIT:0` (AM the 17 crafts, AN the silicon smelt) | `e2e-slice7.log` |
| 12 | same with `CONTROL=1` (the in-run differential) | **PASS** + `EXIT:0`; the package-less build has no generator item and no motor, and every package block id stays air, the three plating blocks included | `e2e-slice7-differential.log` |
| 13 | same with `GAME=$CONTROL_GAME` (the whole scenario on the package-less build) | **EXIT:1** — `the generator did not place` | `e2e-slice7-control.log` |
| 14 | same with `SELFTEST=1` | **EXIT:1** — `91 ticks spent 2912 FE, expected 2944` | `e2e-slice7-selftest.log` |
| 15 | same with `SELFTEST_CRAFT=1` (the coil's count flipped to 5) | **EXIT:1** — `magnetic coil: 6 nickel + 3 steel: expected 5 oritech:magnetic_coil, got oritech:magnetic_coil x4` | `e2e-slice7-selftest-craft.log` |
| 16 | `python3 ports/tools/check-e2e-logs.py oritech-e2e --strict` | 32 logs, 32 ok, 0 bad | stdout |
| 17 | `python3 ports/tools/check-coverage.py oritech` | exit 0, `missing_families=0 missing=0` | stdout |
| 18 | `python3 ports/tools/check-skills.py --notes ports/oritech` | PASS | stdout |

The in-game assertions and what each can fail on are listed in `notes.md` → *Slice 7* → *Verification*
and *In-game transcript*. The e2e now launches `--offscreen`: at this pin a menu's widget layout needs
the renderer, and `--headless` makes `ctl screen` time out on an open crafting table (measured, and the
same note `powah` records).

## 0.2.8 — slice 6, the Control Unit Addon and the redstone gate (pin `0a875da`, 2026-11-22)

| # | Command (from the campaign root) | Verdict | Log |
|---|---|---|---|
| 1 | `build-slot.sh run --step test -- bash -c 'cd ports/oritech && cargo test --offline'` (`redstone_disables` body `todo!()`) | **92 passed, 1 failed** — the gate rule is red first (`panicked … not yet implemented`); the neutral-settings test passed at once (its arm landed in the same edit) | `slice6-test-red.log` |
| 2 | four targeted mutations of the flag/readout, then `cargo test --offline redstone` (`tools/slice6-glue-counterfactual.py`) | **2 passed, 2 failed** — one per new glue test; the file restored byte-identically | `slice6-glue-red-counterfactual.log` |
| 3 | same, implemented | **95 passed, 0 failed** | `sdk-test.log` |
| 4 | `build-slot.sh run --step lint -- bash -c 'cargo fmt --check && cargo clippy --all-targets --offline -- -D warnings'` | exit 0, **0 warnings** | `sdk-clippy.log` |
| 5 | `cargo generate-lockfile --offline` with the launcher's cargo home, then `cd ports/oritech && cargo horizon-mod build` twice, the artifact deleted between | **162,377 B**, sha256 `2d5c36276718c5868c69d218ff5e0a5e9973462b36e9efe59ea108f31189f491`, `cmp` identical, 158 entries **incl. `Cargo.lock`** (rebuilt after the `README.md` edit; the e2e ran the same entries, see `notes.md`) | `slice6-build.log`, `slice6-build-second.log`, `slice6-sha256.txt` |
| 6 | `cargo horizon-mod check oritech.hmod` | exit 0, `oritech 0.2.8 (native-mod, format native-1/0a875da…/e59da90…, for 26.3)` | `sdk-check.log` |
| 7 | `cargo horizon-mod fits oritech.hmod --build 0a875da` | `0a875da  made for`, exit 0 (delivered file, sha `6b3d628f…`) | `sdk-fits.log` |
| 8 | `cargo horizon-mod fits oritech.hmod --build 0b14f8a` | `0b14f8a  fits`, exit 0 (a different build — the pin run alone is a tautology) | `sdk-fits.log` |
| 9 | `python3 ports/tools/verify-sdk-package.py oritech` | **1/1** | stdout |
| 10 | `python3 tools/carry_content.py --check` | `148 files, 0 differ` | stdout |
| 11 | `ports/tools/launcher-game.sh build oritech` (manifest bumped to 0.2.8 so it recompiles) | `Compiling the mods against the prebuilt game`; `strings liboritech.so` shows `redstone enabled redstone disabled` and `oritech:machine_redstone_addon` | — |
| 12 | `eval "$(ports/tools/launcher-game.sh env oritech)"; build-slot.sh run --step e2e -- bash ports/oritech/tools/e2e.sh` | **PASS** + `EXIT:0` (AK the powered pause, AL the release) | `e2e-slice6.log` |
| 13 | same with `CONTROL=1` (the in-run differential) | **PASS** + `EXIT:0`; the package-less build's generator, furnace, core, chiller, lava generator, addon and **control unit** ids stay air | `e2e-slice6-differential.log` |
| 14 | same with `GAME=$CONTROL_GAME` (the whole scenario on the package-less build) | **EXIT:1** — `the generator did not place` | `e2e-slice6-control.log` |
| 15 | same with `SELFTEST=1` | **EXIT:1** — `89 ticks spent 2848 FE, expected 2880` | `e2e-slice6-selftest.log` |
| 16 | same with `SELFTEST_REDSTONE=1` (the powered buffer figure flipped) | **EXIT:1** — `ENERGY is '10000', expected '9968'` | `e2e-slice6-selftest-redstone.log` |
| 17 | `python3 ports/tools/check-e2e-logs.py oritech-e2e --strict` | 27 logs, 27 ok, 0 bad | stdout |
| 18 | `python3 ports/tools/check-coverage.py oritech` | exit 0, `missing_families=0 missing=0` | stdout |
| 19 | `python3 ports/tools/check-skills.py --notes ports/oritech` | PASS | stdout |

The in-game assertions and what each can fail on are listed in `notes.md` → *Slice 6* → *Verification*
and *In-game transcript*. The delivered package now packs `Cargo.lock` (this crate declares the registry
dependency `serde_json`; `verify-sdk-package.py` requires it — see *The `Cargo.lock` correction* in
`notes.md`).

## 0.2.7 — slice 5, the machine addon system (pin `0a875da`, 2026-11-21)

| # | Command (from the campaign root) | Verdict | Log |
|---|---|---|---|
| 1 | `build-slot.sh run --step test -- bash -c 'cd ports/oritech && cargo test --offline'` (new bodies `todo!()`) | **73 passed, 15 failed** — every addon rule is red first (`panicked … not yet implemented`) | `slice5-test-red.log` |
| 2 | same, implemented | **91 passed, 0 failed** | `sdk-test.log` |
| 3 | `build-slot.sh run --step lint -- bash -c 'cargo fmt --check && cargo clippy --all-targets --offline -- -D warnings'` | exit 0, **0 warnings** | `sdk-clippy.log` |
| 4 | `cd ports/oritech && rm -f Cargo.lock && cargo horizon-mod build` (twice, `Cargo.lock` removed and artifact deleted between) | **149,358 B**, sha256 `f1232e32e10c0d139d0e2681c06a970190f9ef6314b4bf736e73d4c5077c3753`, `cmp` identical, 150 entries, no packed `Cargo.lock` | `slice5-build.log` |
| 5 | `cargo horizon-mod check oritech.hmod` | exit 0, `oritech 0.2.7 (native-mod, format native-1/0a875da…/e59da90…, for 26.3)` | `sdk-check.log` |
| 6 | `cargo horizon-mod fits oritech.hmod --build 0a875da` | `0a875da  made for`, exit 0 (delivered file, sha `f1232e32…`) | `sdk-fits.log` |
| 7 | `cargo horizon-mod fits oritech.hmod --build 0b14f8a` | `0b14f8a  fits`, exit 0 (a different build — the pin run alone is a tautology) | `sdk-fits.log` |
| 8 | `python3 ports/tools/verify-sdk-package.py oritech` | **1/1** | stdout |
| 9 | `python3 tools/carry_content.py --check` | `141 files, 0 differ` | stdout |
| 10 | `ports/tools/launcher-game.sh build oritech` (manifest bumped to 0.2.7 so it recompiles) | `Compiling the mods against the prebuilt game`; `strings liboritech.so` shows `' has no open addon slot on this machine` and `oritech:machine_speed_addon` | `slice5-launcher-build.log` |
| 11 | `eval "$(ports/tools/launcher-game.sh env oritech)"; build-slot.sh run --step e2e -- bash ports/oritech/tools/e2e.sh` | **PASS** + `EXIT:0` | `e2e-slice5.log` |
| 12 | same with `CONTROL=1` (the in-run differential) | **PASS** + `EXIT:0`; the package-less build has no generator item and its generator, furnace, core, chiller, lava generator and both addon ids stay air | `e2e-slice5-differential.log` |
| 13 | same with `GAME=$CONTROL_GAME` (the whole scenario on the package-less build) | **EXIT:1** — `the generator did not place` | `e2e-slice5-control.log` |
| 14 | same with `SELFTEST=1` | **EXIT:1** — `89 ticks spent 2848 FE, expected 2880` | `e2e-slice5-selftest.log` |
| 15 | same with `SELFTEST_ADDONS=1` (one tick too much primed) | **EXIT:1** — `ENERGY is '58', expected '0'` | `e2e-slice5-selftest-addons.log` |
| 16 | `python3 ports/tools/check-e2e-logs.py oritech-e2e --strict` | 22 logs, 22 ok, 0 bad | stdout |
| 17 | `python3 ports/tools/check-coverage.py oritech` | exit 0, `missing_families=0 missing=0` | stdout |
| 18 | `python3 ports/tools/check-skills.py --notes ports/oritech` | PASS | stdout |

The in-game assertions and what each can fail on are listed in `notes.md` → *Slice 5* → *Verification*
and *In-game transcript*.

## 0.2.6 — slice 4, the Lava Generator (pin `0a875da`, 2026-11-21)

| # | Command (from the campaign root) | Verdict | Log |
|---|---|---|---|
| 1 | `build-slot.sh run --step test -- bash -c 'cd ports/oritech && cargo test --offline'` (new bodies `todo!()`) | **57 passed, 9 failed** — the lava generator's rules are red first (the tenth test pins the shared `tank_matches` and passed at once) | `slice4-test-red.log` |
| 2 | same, the new glue bodies `todo!()` | **68 passed, 5 failed** | `slice4-glue-red.log` |
| 3 | six targeted mutations of the glue, then the tests (`tools/slice4-glue-counterfactual.py`) | **68 passed, 5 failed** — one per new glue test; the file restored byte-identically | `slice4-glue-red-counterfactual.log` |
| 4 | same, implemented | **73 passed, 0 failed** | `sdk-test.log` |
| 5 | `build-slot.sh run --step lint -- bash -c 'cargo fmt --check && cargo clippy --all-targets --offline -- -D warnings'` | exit 0, **0 warnings** | `sdk-clippy.log` |
| 6 | `cd ports/oritech && rm -f Cargo.lock && cargo horizon-mod build` (twice, `Cargo.lock` removed and artifact deleted between) | **105,595 B**, sha256 `a42999768aba2e3083422a006c28da3154b93256bc98d6fd691cf1c607f06cac`, `cmp` identical, 107 entries, no packed `Cargo.lock` | `sdk-build.log`, `sdk-build-second.log`, `sdk-sha256.txt` |
| 7 | `cargo horizon-mod check oritech.hmod` | exit 0, `oritech 0.2.6 (native-mod, format native-1/0a875da…/e59da90…, for 26.3)` | `sdk-check.log` |
| 8 | `cargo horizon-mod fits oritech.hmod --build 0a875da` | `0a875da  made for`, exit 0 (delivered file, sha `a4299976…`) | `sdk-fits.log` |
| 9 | `cargo horizon-mod fits oritech.hmod --build 0b14f8a` | `0b14f8a  fits`, exit 0 (a different build — the pin run alone is a tautology) | `sdk-fits.log` |
| 10 | `python3 ports/tools/verify-sdk-package.py oritech` | **1/1** | stdout |
| 11 | `python3 tools/carry_content.py --check` | `98 files, 0 differ` | stdout |
| 12 | `ports/tools/launcher-game.sh build oritech` (manifest bumped to 0.2.6 so it recompiles) | `Compiling the mods against the prebuilt game`; `strings liboritech.so` shows the lava generator's readout and ids | — |
| 13 | `eval "$(ports/tools/launcher-game.sh env oritech)"; build-slot.sh run --step e2e -- bash ports/oritech/tools/e2e.sh` | **PASS** + `EXIT:0` | `e2e-slice4.log` |
| 14 | same with `CONTROL=1` (the in-run differential) | **PASS** + `EXIT:0`; the package-less build has no generator item and its generator, furnace, core, chiller and lava generator ids stay air | `e2e-slice4-differential.log` |
| 15 | same with `GAME=$CONTROL_GAME` (the whole scenario on the package-less build) | **EXIT:1** — `the generator did not place` | `e2e-slice4-control.log` |
| 16 | same with `SELFTEST=1` | **EXIT:1** — `89 ticks spent 2848 FE, expected 2880` | `e2e-slice4-selftest.log` |
| 17 | same with `SELFTEST_LAVA=1` | **EXIT:1** — `a 120-tick lava burn makes exactly 7680 FE, so the buffer should hold 7744, not 7680` | `e2e-slice4-selftest-lava.log` |
| 18 | `python3 ports/tools/check-e2e-logs.py oritech-e2e --strict` | 17 logs, 17 ok, 0 bad | stdout |
| 19 | `python3 ports/tools/check-coverage.py oritech` | exit 0, `missing_families=0 missing=0` | stdout |
| 20 | `python3 ports/tools/check-skills.py --notes ports/oritech` | PASS | stdout |

The in-game assertions and what each can fail on are listed in `notes.md` → *Slice 4* → *Verification*
and *In-game transcript*.

## 0.2.5 — slice 3, the Industrial Chiller (pin `0a875da`, 2026-11-21)

| # | Command (from the campaign root) | Verdict | Log |
|---|---|---|---|
| 1 | `build-slot.sh run --step test -- bash -c 'cd ports/oritech && cargo test --offline'` (new bodies `todo!()`) | **38 passed, 10 failed** — the chiller's rules are red first | `slice3-test-red.log` |
| 2 | nine targeted mutations of the glue, then the tests (`tools/slice3-glue-counterfactual.py`) | **48 passed, 8 failed** — one per new glue test; the file restored byte-identically | `slice3-glue-red-counterfactual.log` |
| 3 | same, implemented | **56 passed, 0 failed** | `sdk-test.log` |
| 4 | `build-slot.sh run --step lint -- bash -c 'cargo fmt --check && cargo clippy --all-targets --offline -- -D warnings'` | exit 0, **0 warnings** | `sdk-clippy.log` |
| 5 | `cd ports/oritech && rm -f Cargo.lock && cargo horizon-mod build` (twice, artifact deleted between) | **94,227 B**, sha256 `89d0ae6fa5cf537af02358c7c3fa517abca9b04a7af9cbb4b484e5652045ef3b`, `cmp` identical, 99 entries, no packed `Cargo.lock` | `sdk-build.log` |
| 6 | `cargo horizon-mod check oritech.hmod` | exit 0, `oritech 0.2.5 (native-mod, format native-1/0a875da…/e59da90…, for 26.3)` | `sdk-check.log` |
| 7 | `cargo horizon-mod fits oritech.hmod --build 0a875da` | `0a875da  made for`, exit 0 | `sdk-fits.log` |
| 8 | `cargo horizon-mod fits oritech.hmod --build 0b14f8a` | `0b14f8a  fits`, exit 0 (a different build — the pin run alone is a tautology) | `sdk-fits.log` |
| 9 | `python3 ports/tools/verify-sdk-package.py oritech` | **1/1** | stdout |
| 10 | `python3 tools/carry_content.py --check` | `90 files, 0 differ` | `slice3-carry-check.log` |
| 11 | `ports/tools/launcher-game.sh build oritech` (a tree `Cargo.lock` from the launcher's cargo home; manifest bumped to 0.2.5 so it recompiles) | game + control game built and exported; `strings liboritech.so` shows the chiller's readout | — |
| 12 | `eval "$(ports/tools/launcher-game.sh env oritech)"; build-slot.sh run --step e2e -- bash ports/oritech/tools/e2e.sh` | **PASS** + `EXIT:0` | `e2e-slice3.log` |
| 13 | same with `CONTROL=1` (the in-run differential) | **PASS** + `EXIT:0`; the package-less build has no chiller item and its chiller, core, furnace and generator ids stay air | `e2e-slice3-differential.log` |
| 14 | same with `GAME=$CONTROL_GAME` (the whole scenario on the package-less build) | **EXIT:1** — `the generator did not place` | `e2e-slice3-control.log` |
| 15 | same with `SELFTEST=1` | **EXIT:1** — `89 ticks spent 2848 FE, expected 2880` | `e2e-slice3-selftest.log` |
| 16 | same with `SELFTEST_COOLER=1` | **EXIT:1** — `148 ticks needed exactly 4736 FE, so the buffer should hold 32, not 0` | `e2e-slice3-selftest-cooler.log` |
| 17 | `python3 ports/tools/check-e2e-logs.py oritech-e2e --strict` | 12 logs, 12 ok, 0 bad | stdout |
| 18 | `python3 ports/tools/check-coverage.py oritech` | exit 0, `missing_families=0 missing=0` | stdout |
| 19 | `python3 ports/tools/check-skills.py --notes ports/oritech` | PASS | stdout |

The in-game assertions and what each can fail on are listed in `notes.md` → *Slice 3* → *Parity
coverage* and *Verification*.

## 0.2.2 — slice 2, the Powered Furnace and the machine cores (pin `0a875da`, 2026-11-21)

| # | Command (from the campaign root) | Verdict | Log |
|---|---|---|---|
| 1 | `build-slot.sh run --step test -- bash -c 'cd ports/oritech && cargo test --offline'` (new bodies `todo!()`) | **26 passed, 7 failed** — the slice's rules are red first | `slice2-test-red.log` |
| 2 | same, glue stubs | **35 passed, 3 failed** | `slice2-glue-red.log` |
| 3 | same, implemented | **38 passed, 0 failed** | `sdk-test.log` |
| 4 | `build-slot.sh run --step lint -- bash -c 'cargo fmt --check && cargo clippy --all-targets --offline -- -D warnings'` | exit 0, **0 warnings** | `sdk-clippy.log` |
| 5 | `cd ports/oritech && rm -f Cargo.lock && cargo horizon-mod build` (twice, artifact deleted between) | **80,939 B**, sha256 `9af384e8e4aca3fd7ca956187791ebd9c85341ab258a3d845a52dad2b037eefc`, `cmp` identical, 90 entries, no packed `Cargo.lock` | `slice2-build.log` |
| 6 | `cargo horizon-mod check oritech.hmod` | exit 0, `oritech 0.2.2 (native-mod, format native-1/0a875da…/e59da90…, for 26.3)` | `slice2-check.log` |
| 7 | `cargo horizon-mod fits oritech.hmod --build 0a875da` | `0a875da  made for`, exit 0 | `slice2-fits.log` |
| 8 | `cargo horizon-mod fits oritech.hmod --build 0b14f8a` | `0b14f8a  fits`, exit 0 (a different build — the pin run alone is a tautology) | `slice2-fits.log` |
| 9 | `python3 ports/tools/verify-sdk-package.py oritech` | **1/1** | stdout |
| 10 | `ports/tools/launcher-game.sh build oritech` (needs a tree `Cargo.lock`: see `notes.md`) | game + control game built and exported | — |
| 11 | `eval "$(ports/tools/launcher-game.sh env oritech)"; build-slot.sh run --step e2e -- bash ports/oritech/tools/e2e.sh` | **PASS** + `EXIT:0` | `e2e-slice2.log` |
| 12 | same with `GAME=$CONTROL_GAME` (the package-less build) | **EXIT:1** — `the generator did not place` | `e2e-slice2-control.log` |
| 13 | same with `CONTROL=1` (in-run differential) | generator item absent; generator, furnace and core ids `minecraft:air` | `e2e-slice2-differential.log` |
| 14 | same with `SELFTEST=1` | **EXIT:1** (`89 ticks spent 2848 FE, expected 2880`) | `e2e-slice2-selftest.log` |
| 15 | same with `SELFTEST_FURNACE=1` | **EXIT:1** (`90 ticks needed exactly 2880 FE, so the buffer should hold 32, not 0`) | `e2e-slice2-selftest-furnace.log` |
| 16 | `python3 ports/tools/check-e2e-logs.py oritech-e2e --strict` | 7 logs, 7 ok, 0 bad | stdout |
| 17 | `python3 ports/tools/carry_content.py --check` | `81 files, 0 differ` | `slice2-carry-check.log` |
| 18 | `python3 ports/tools/check-skills.py --notes ports/oritech` | PASS | stdout |

The in-game assertions and what each can fail on are listed in `notes.md` → *Parity coverage*.

## 0.2.1 — review fixes (pin `0b14f8a`, 2026-11-18)

See `notes.md` → *Review fixes (2026-11-18)*; evidence `e2e-repair.log` (`PASS`, `EXIT:0`).

## 0.2.0 — the SDK-format migration (pin `0b14f8a`, 2026-11-18)

See `notes.md` → *Migration to the SDK format*; no in-game run at that time (no launcher-built game
existed for the package yet).

## 0.1.0 — the retired `.hnmod` bundle (pin `1d63e0a`, 2026-11-11)

See `notes.md` → *Verification* and the historical logs `e2e.log`, `e2e-selftest.log`,
`e2e-embedded.log`, `e2e-embedded-selftest.log`.
