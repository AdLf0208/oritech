# Oritech for Horizon — slice 1

The **Basic Generator** and the **Pulverizer** of [Oritech
1.2.13](https://github.com/Rearth/Oritech) (CC0-1.0), rewritten in Rust for
Horizon at SDK nightly `0b14f8acfd97fa2c750e42795249dcc981178ca4` (mod API 0.2,
26.3).

One flat `.hmod` carries both halves — `src/lib.rs` (the one `impl Hooks`), the
two block-entity Parts and the rules in `src/runtime.rs`, beside the carried
`assets/` and `data/` (blocks, items, models, textures, recipes, loot, lang,
tags).

## What it does

- **Basic Generator** (`oritech:basic_generator_block`) — burns any fuel the
  game's fuel table knows, making **32 FE a tick** while it burns, into a
  **50,000 FE** buffer that accepts no inserts and gives up to **256 FE** a
  call. The buffer pauses the burn when it has no room for a full tick, as
  upstream's `canFitEnergy` does. One coal is 1,600 burning ticks.
- **Pulverizer** (`oritech:pulverizer_block`) — a **25,000 FE** machine that
  spends **32 FE a tick** while it works and turns one input into a recipe's
  results in the recipe's `time` ticks. The recipes are the jar's own
  `oritech:pulverizer` files: bone → 6 bone meal, blaze rod → 3 blaze powder,
  dripstone block → 4 pointed dripstone, glowstone → 3 glowstone dust,
  `#minecraft:coal_ores` → 2 coal, `#minecraft:diamond_ores` → 1 diamond,
  `#minecraft:lapis_ores` → 6 lapis lazuli (all 100 ticks).
- **The energy link** — a fuelled generator pushes its FE to the pulverizers
  beside it (`move_energy`, 256 FE a call), so a machine next to a burner runs
  and one with no source idles with its input untouched.
- **Using it** — right-click a generator with fuel, or a pulverizer with a
  matching item, to insert it; right-click with an empty hand to read the
  machine. `/oritech info <x> <y> <z>` and `/oritech energy <x> <y> <z>
  <amount>` (op) read and prime a machine.

## What it does not do (yet)

No GUI, no GeckoLib models (the machines are drawn as a full cube of their own
texture), no crafting recipe for the two blocks (they need Oritech components
that are not in this slice — use `/give`), no addons, no energy pipes, no
redstone control, no comparator output, and no multiblock machines (the powered
furnace, assembler, atomic forge, foundry, refinery, centrifuge), no augment
stations, no accelerator ring or reactor, no ore processing chain and no ores.
`notes.md` lists every gate and deviation; `FINDINGS.md` has the upstream
behaviour and the jar numbers.

## Building

```sh
python3 tools/carry_content.py              # assets/ + data/ from the ATM10 jar (deterministic)
cargo test --offline                       # the rules: 18 runtime tests, 7 glue tests
cargo clippy --all-targets --offline -- -D warnings
cargo fmt --check                          # NOT --all: that checks the fetched SDK
cargo horizon-mod build                    # target/horizon-mod/oritech.hmod
cargo horizon-mod check oritech.hmod
cargo horizon-mod fits oritech.hmod --build nightly
```

`cargo horizon-mod` fetches the pinned SDK into `.horizon/sdk/` (gitignored,
never packed); the tree here already carries a copy of the campaign cache. The
package's one registry dependency is `serde_json` (the carried rules module
parses the recipe JSON), so its `Cargo.lock` travels **inside** the package.

The in-game leg needs a game the launcher built with this package —
`horizon-launcher native-build . --instance <instance> --trust-native-code` —
because a native `.hmod` cannot load on an unmodded build. `tools/e2e.sh` is
the scenario and its assertions, driven with `GAME=` (see the file's header);
it is **not** a staged build any more. `SELFTEST=1` must fail.

Every build/test step goes through `../tools/build-slot.sh` in the campaign
tree (`BUILD_LOCAL=1`, since the far box has no copy of this port). Pin
`0b14f8acfd97fa2c750e42795249dcc981178ca4`, mixins
`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`.

Licence: CC0-1.0 (`LICENSE`, `LICENSE-NOTICE.md`).
