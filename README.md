# oritech (Horizon port)

A Horizon port of [oritech](Rearth/Oritech) — CC0-1.0.

**Status: first slices.** What is below is what the port actually does today.

## Implemented so far

- | Behaviour | Upstream file | This port |
- |---|---|---|
- | generator buffer | `OritechConfig.BasicMachineConfig(50_000, 0, 32*8, 32)` | API 142 buffer: capacity 50,000 FE, insert 0, extract 256 FE/call |
- | generator production | `UpgradableGeneratorBlockEntity.produceEnergy` | **32 FE** a burning tick |
- | generator fuel | `FuelRegistry.get(item)` | the engine's `minecraft:cooking_fuel` (`Providers::burn_time`), so modded fuels work |
- | generator burn | `progress = burnTime`, one per tick | same; a coal is 1,600 burning ticks |
- | buffer-full rule | `canFitEnergy` (room for a whole tick) | same: at 49,984 FE the burn pauses and keeps its remaining progress |
- | generator output | `outputEnergy` to the six neighbours | queued, then `native_move_energy` at `level_tick_post` (256 FE/call) |

## Not yet ported

- The energy buffer's key is the API's `energy` (int), not upstream's
- `oritech.machine_energy` (long).** API 142 reads and writes the int `energy`;
- every value here fits, and the buffer is a real `Capabilities.Energy.BLOCK`
- to other mods.
- Progress is saved with the recipe id** (`oritech.machine_recipe`).
- Upstream does not save `currentRecipe`, so its first tick after a reload
- resets `progress`; here a reload continues the operation. The visible rule
- (a different input restarts the count) is upstream's.

## Install

Build the bundle with a native-mod capable launcher, or add the file from the
registry once it is listed. Pinned to Horizon `1d63e0a` and horizon-mixins
`5a64af93` for game 26.3.

## Build from source

```sh
cargo test
python3 tools/horizon.py check --horizon <horizon checkout>
python3 tools/horizon.py build --horizon <horizon checkout>
python3 tools/package.py
```

## Credits and licence

Original mod: Rearth/Oritech. Port: AdLf0208. Licence: CC0-1.0 — see [LICENSE](LICENSE);
the original notices are preserved.
