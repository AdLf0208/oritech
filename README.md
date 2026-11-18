# Oritech — a Horizon port

**Oritech** is a Minecraft mod. Minecraft fabric/neoforge 1.21 tech mod. This repository is a **port of it to [Horizon](https://github.com/horizon-rust/horizon)**, the Rust engine for Minecraft Java 26.3. It carries the complete source of the port plus the content files it ships, and installs as a single `.hmod` package.

Upstream: [Rearth/Oritech](https://github.com/Rearth/Oritech) — licence **CC0-1.0**.

| | |
|---|---|
| Upstream mod | Oritech ([source](https://github.com/Rearth/Oritech)) |
| Ported to | Horizon 0b14f8a, Minecraft Java 26.3, mod API `0.2` |
| Package | `oritech.hmod` (install it with the Horizon launcher) |
| Licence | **CC0-1.0** — the upstream mod's licence; see [Licence and credit](#licence-and-credit) |

## What this port implements

- **The embedded-content control leg is gone** (no gap: the format carries the content). The
- **Usage messages for a malformed command now come from the dispatcher.** The retired
- **`tools/e2e.sh` now launches a launcher-built game.** `GAME=` is required and there is no

## Not yet ported

- **The energy buffer's key is the API's `energy` (int), not upstream's
- **Progress is saved with the recipe id** (`oritech.machine_recipe`).
- **No GUI/screen.** Upstream opens a machine screen with an energy bar, a
- **No GeckoLib rendering.** The machines are drawn as a full cube of their own
- **The machines have no crafting recipes.** They need Oritech components
- **117 of the 124 pulverizer recipes are not carried**: they name Oritech
- **No dust auto-combine** (`PulverizerBlockEntity.combineSmallDusts`: 9 small
- **The energy link is adjacency.** Upstream pushes to its six neighbours

## Installing

1. Install the [Horizon launcher](https://github.com/horizon-rust/horizon/releases) (0.2.70 or newer).
2. Add `oritech.hmod` to the instance's mods.
3. The package is pinned to a specific Horizon build; the launcher checks that pin.

## Licence and credit

Oritech was created by its upstream authors; the code, models and assets in the original mod
remain theirs and are used here under **CC0-1.0**. The Rust in this repository is a clean-room
rewrite for Horizon, not a translation of the original source.

- `LICENSE` — the upstream licence text.
- `LICENSE-NOTICE.md` — what this port carries and where it came from.
- The upstream project: https://github.com/Rearth/Oritech

## Technical details

```
horizon-mod.toml   package manifest (id, version, api, [native] pin)
Cargo.toml         the crate; depends on `horizon-mod-api` from the fetched SDK
src/               lib.rs (the one `impl Hooks`), runtime.rs (pure logic + tests), glue
assets/ data/      the content the package ships
tools/             content generators and the e2e scenario
```

- Build: `cargo horizon-mod build` (packages; it does not compile against the game).
- Test: `cargo test`, `cargo clippy --all-targets -- -D warnings`.
- In-game: `ports/tools/launcher-game.sh run oritech` in the campaign tree, or install the
  `.hmod` and play; the harness scenario is `tools/e2e.sh`.

This repository is generated from the campaign's port at `ports/oritech/`; the port's own
`notes.md` is the full record (decisions, traps, evidence).
