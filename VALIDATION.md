# Oritech — validation record

The commands that produce the evidence, and their verdicts, newest first. Every Rust, package and
in-game step ran inside a `ports/tools/build-slot.sh` lease (`BUILD_LOCAL=1`, `CARGO_INCREMENTAL=0`,
no `--target-dir`); the launcher build runs outside a slot because it has its own global build lock
and its own cache. Logs: `../_shared/evidence/oritech/` and `../_shared/evidence/oritech-e2e/`.

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
