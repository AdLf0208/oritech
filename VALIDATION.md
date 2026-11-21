# Oritech — validation record

The commands that produce the evidence, and their verdicts, newest first. Every Rust, package and
in-game step ran inside a `ports/tools/build-slot.sh` lease (`BUILD_LOCAL=1`, `CARGO_INCREMENTAL=0`,
no `--target-dir`); the launcher build runs outside a slot because it has its own global build lock
and its own cache. Logs: `../_shared/evidence/oritech/` and `../_shared/evidence/oritech-e2e/`.

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
