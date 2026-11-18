> **Historical (0.1.0, retired format).** This is the slice's original plan, written for the
> `.hnmod` container (`mixins/`, `tools/horizon.py`, `tools/package.py`). Since 0.2.0 the port is
> one flat SDK `.hmod`; see `notes.md` → "Migration to the SDK format" for the file map and the
> current ladder. The scenario names and the jar numbers below still describe the slice.

# Oritech slice 1 — implementation plan

**Goal:** one Basic Generator and the Pulverizer it powers, on Horizon at pin
`1d63e0a5`, with the jar's FE/t and tick numbers, shipped as one `.hnmod`
(content embedded), verified in game with a control that the machine idles
without power.

**Architecture:** `runtime.rs` is a pure, unit-tested state machine (two
structs: `Generator`, `Pulverizer`, plus the pulverizer recipe parser).
`support.rs` is the game glue (NBT, slots, engine fuel/recipe helpers, the
energy queue). One helper file per injection.

**Tech Stack:** Rust 2024 (1.95), `serde_json` for the recipe JSON, Horizon
native API 8/15/16/17/21/36/142/23 (`native-hooks.json` at the pin).

**Spec:** `FINDINGS.md` (upstream behaviour and the jar numbers) and
`ports/_shared/queue/brief-oritech.md` (the slice boundary).

## Global constraints

- Numbers only from `FINDINGS.md`: generator 32 FE/t, 50,000 FE, extract 256;
  pulverizer 32 FE/t, 25,000 FE, insert 256; recipe times from the files
  (100 ticks for every carried recipe).
- Stamped hooks only. The ten named hooks are all present in
  `ports/_shared/pins/horizon-1d63e0a/native-hooks.json`; `content_root` is the
  raw target the AE2 pattern uses (as in `charginggadgets`/`actuallyadditions`).
- No GUI, no multiblock, no addons, no GeckoLib renderer: recorded gaps.
- One write owner (this port and its evidence only).

## File structure

| File | Role |
|---|---|
| `mixins/oritech/runtime.rs` | pure: `Generator`, `Pulverizer`, `PulverizerRecipe` parse, `Step` |
| `mixins/oritech/support.rs` | glue: NBT state, slots, fuel, recipe cache, energy queue, commands |
| `mixins/oritech/block_entity.rs` etc. | one private function per `[[injection]]` |
| `content/` | the content mod: 2 blocks/items + models + textures + 7 pulverizer recipes + loot + lang + tags |
| `tools/carry_content.py` | deterministic carry of the jar assets/data into `content/` |
| `tools/e2e.sh` | fail-capable game scenario, embedded leg, control leg, selftest |

## Tasks

### Task 1: runtime TDD (red → green)

Files: `mixins/oritech/runtime.rs` (also the crate lib), `Cargo.toml`.

Tests first, then implement:

1. `a_burn_tick_makes_32_fe_and_spends_one_tick` — `Generator{progress:1600}` →
   +32 FE, progress 1599.
2. `a_buffer_without_room_for_a_whole_tick_does_not_burn` — at 49,984 FE a tick
   adds nothing and leaves progress.
3. `one_coal_is_1600_ticks_of_32_fe_up_to_the_capacity` — 1600 ticks with a
   drain-free buffer stops at 50,000 and keeps the remaining progress; with a
   drain it completes 51,200 FE.
4. `a_fuel_item_is_refused_when_zero_burn_time` — `load_fuel(0)` is false.
5. `the_pulverizer_spends_32_fe_a_tick_and_finishes_at_the_recipe_time` —
   `Pulverizer{energy:10_000}.tick(Some(100))` 100 times → `Finished`, energy
   6,800.
6. `the_pulverizer_keeps_progress_when_starved` / `resets_when_there_is_no_work`
   — the two upstream branches.
7. `the_recipe_parser_reads_the_jar_shape` — `{type, ingredients, results, time}`
   → ingredient list + results + time; refusals: missing time, empty results,
   bad count.
8. `the_output_room_rule` — a result must fit its slot (same item, count within
   the stack limit); two results use slots 1 and 2.
9. `ingredient_json_maps_item_tag_and_list` — a 1.21.1 ingredient → the pin's
   `"id"`/`"#tag"`/list form.

### Task 2: glue (`support.rs`)

- `on_block_entity`, `on_block_entity_ticks`, `on_block_entity_container`,
  `on_block_entity_energy` (the two buffers, upstream's faces),
  `on_block_entity_tick` (generator burn; pulverizer recipe → pure tick →
  finish), `on_level_tick_post` (queued generator → adjacent pulverizer through
  `native_move_energy`, 256/call), `on_right_click_block` (insert fuel/input,
  read status), `on_register_commands`/`on_perform_command`
  (`/oritech info|energy`), `on_world_lifecycle` (drop the recipe cache).

### Task 3: content

`tools/carry_content.py` writes `content/` from the jar: the two machine
textures, the seven vanilla-only pulverizer recipes verbatim, a lang subset,
and the authored files (blockstate → a `cube_all` model of the machine's own
texture, item models, 26.3 loot with the singular `condition`, block
declarations on the `minecraft:iron_block` template with `occludes: false`,
creative tab, pickaxe/stone-tool tags). `CONTENT.md` lists every file.

### Task 4: ladder in a slot

`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
`cargo test`; then `python3 tools/horizon.py stage|check|test|build`; then
`tools/package.sh`; then `cargo horizon-mod check` on both artifacts.

### Task 5: e2e

A: generator + pulverizer placed, pulverizer has input, generator has no fuel →
0 FE, progress 0, input kept (control).
B: one coal in the generator → FE appears at 32/t; the adjacent pulverizer
takes it and finishes `#minecraft:coal_ores` → 2 coal.
C: a pre-charged pulverizer with no generator finishes exactly 100 ticks later
and loses exactly 3,200 FE (the jar's numbers).
D: a non-fuel item in the generator does nothing; a non-recipe item in the
pulverizer does nothing.
E: `EMBEDDED=1` — the same run with no `--dev-mod`, plus a `CONTROL=1` leg where
`.horizon-cache` is a plain file and the ids are unknown.
F: `SELFTEST=1` flips C's tick count and exits non-zero.

## Review focus

- The generator's capacity gate (`canFitEnergy`) must stop the burn, not clamp
  silently — a wrong rule shows up as FE appearing above 50,000.
- The pulverizer must never consume its input when it cannot finish (blocked
  output) and must not spend FE with no recipe.
- The energy move must conserve: generator loss == pulverizer gain, per tick.
- `ctl container` inserts into `inputs` only; outputs must be read by the
  command, not `ctl container`.
