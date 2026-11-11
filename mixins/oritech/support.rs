// Game-side glue for Oritech slice 1 (Basic Generator, Pulverizer).
//
// Injected into `crate::game` as `oritech_state`; it needs the game, so it is
// not part of the standalone library (only `runtime.rs` is).
//
// Energy: both blocks declare a `Capabilities.Energy.BLOCK` buffer (API 142)
// whose amount is the int `energy` in the block entity's saved compound. The
// generator's own tick adds 32 FE a burning tick; the pulverizer's own tick
// spends 32 FE a working tick. A block entity is out of the world during its own
// tick, so the generator queues an adjacent-pulverizer transfer and the engine
// helper (`native_move_energy`, the `EnergyHandlerUtil.move` counterpart) runs
// it at `level_tick_post`, where both blocks are in the world.
//
// Fuel: the burn time is the engine's `minecraft:cooking_fuel` value
// (`Providers::burn_time`, the counterpart of upstream's
// `FuelRegistry.get(stack)`), so no fuel table is invented.
//
// Recipes: `oritech:pulverizer` is a type outside `minecraft:`, which the pin
// keeps in its custom-recipe table as the file's JSON
// (`native_recipes_of_type`). The pure parser lives in `runtime.rs`; the
// ingredient is resolved by the engine's `native_recipe_ingredient` so tags
// match as vanilla matches them.
//
// Read from Oritech 1.2.13 (`BasicGeneratorEntity`,
// `UpgradableGeneratorBlockEntity`, `MachineBlockEntity`,
// `PulverizerBlockEntity`, `OritechConfig`). See `../../FINDINGS.md`.

use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use horizon_client::containers::{BlockEntity, ModBlockEntity, ModContainer, ModEnergyBuffer};
use horizon_client::crafting::Ingredient;
use horizon_client::items::ItemStack;
use horizon_client::text::Text;
use horizon_core::geometry::Direction;
use horizon_world::nbt::{Compound, JavaString, Tag};

use crate::game::native_lifecycle::WorldPhase;
use crate::game::native_server::{NativeCommand, NativeServerPlayer};
use crate::game::oritech::{
    GENERATOR_CAPACITY, GENERATOR_MAX_EXTRACT, Generator, PULVERIZER_CAPACITY,
    PULVERIZER_MAX_INSERT, Pulverizer, PulverizerRecipe, Step, Work, can_output, clamp_energy,
    is_fuel,
};

/// The generator block and item (`BlockContent.BASIC_GENERATOR_BLOCK`).
pub const GENERATOR: &str = "oritech:basic_generator_block";
/// The pulverizer block and item (`BlockContent.PULVERIZER_BLOCK`).
pub const PULVERIZER: &str = "oritech:pulverizer_block";
/// The recipe type the jar's `data/oritech/recipe/pulverizer/*.json` names.
const PULVERIZER_RECIPES: &str = "oritech:pulverizer";

/// Upstream slot order: the generator's fuel, the pulverizer's input and its
/// two outputs (`InventorySlotAssignment`).
const FUEL_SLOT: usize = 0;
const INPUT_SLOT: usize = 0;
const OUTPUT_ONE: usize = 1;
const OUTPUT_TWO: usize = 2;
const GENERATOR_SLOTS: usize = 1;
const PULVERIZER_SLOTS: usize = 3;

/// Saved keys: the API 142 buffer's own `energy`, upstream's machine progress,
/// the generator's `storedBurn`, and the recipe an operation belongs to.
const KEY_ENERGY: &str = "energy";
const KEY_PROGRESS: &str = "oritech.machine_progress";
const KEY_STORED_BURN: &str = "storedBurn";
const KEY_RECIPE: &str = "oritech.machine_recipe";

/// Rebuild the parsed recipe cache this often (ticks); a `/reload` lands within
/// two seconds, and the pure parser cost is paid once.
const RECIPE_CACHE_TICKS: u64 = 40;

fn nbt_key(name: &str) -> JavaString {
    JavaString::from(name)
}

fn read_int(nbt: &Compound, key: &str) -> i32 {
    match nbt.get(&nbt_key(key)) {
        Some(Tag::Int(value)) => *value,
        _ => 0,
    }
}

fn write_int(nbt: &mut Compound, key: &str, value: i32) {
    nbt.insert(nbt_key(key), Tag::Int(value));
}

fn read_string(nbt: &Compound, key: &str) -> String {
    match nbt.get(&nbt_key(key)) {
        Some(Tag::String(value)) => value.to_string_lossy(),
        _ => String::new(),
    }
}

fn write_string(nbt: &mut Compound, key: &str, value: &str) {
    nbt.insert(nbt_key(key), Tag::String(JavaString::from(value)));
}

fn state_name(
    game: &crate::game::Game<'_>,
    state: horizon_world::registry::BlockStateId,
) -> String {
    game.res.facts().get(state).name.to_string()
}

fn block_at(game: &crate::game::Game<'_>, pos: [i32; 3]) -> String {
    state_name(game, game.world.block(pos[0], pos[1], pos[2]))
}

/// `BlockEntity.setChanged`: the chunk is written at the next save.
fn mark_modified(game: &mut crate::game::Game<'_>, pos: [i32; 3]) {
    game.world.modified.insert((pos[0] >> 4, pos[2] >> 4));
}

/// The engine's fuel value (`minecraft:cooking_fuel.burn_time`), 0 if none.
fn fuel_burn_time(game: &crate::game::Game<'_>, stack: &ItemStack) -> u32 {
    let providers = game.res.providers();
    let items = game.res.items();
    providers.burn_time(items, stack, false).max(0) as u32
}

/// `ItemInfo::max_stack`, 64 when unknown (vanilla's default).
fn max_stack(game: &crate::game::Game<'_>, item: &str) -> i32 {
    game.res
        .items()
        .get(item)
        .map_or(64, |info| info.max_stack as i32)
}

pub fn generator_state(entity: &ModBlockEntity) -> Generator {
    Generator {
        energy: clamp_energy(read_int(&entity.nbt, KEY_ENERGY), GENERATOR_CAPACITY),
        progress: read_int(&entity.nbt, KEY_PROGRESS).max(0),
        max_burn: read_int(&entity.nbt, KEY_STORED_BURN).max(0),
    }
}

fn store_generator(entity: &mut ModBlockEntity, state: &Generator) {
    write_int(&mut entity.nbt, KEY_ENERGY, state.energy);
    write_int(&mut entity.nbt, KEY_PROGRESS, state.progress);
    write_int(&mut entity.nbt, KEY_STORED_BURN, state.max_burn);
}

fn pulverizer_state(entity: &ModBlockEntity) -> Pulverizer {
    Pulverizer {
        energy: clamp_energy(read_int(&entity.nbt, KEY_ENERGY), PULVERIZER_CAPACITY),
        progress: read_int(&entity.nbt, KEY_PROGRESS).max(0) as u32,
        recipe: read_string(&entity.nbt, KEY_RECIPE),
    }
}

fn store_pulverizer(entity: &mut ModBlockEntity, state: &Pulverizer) {
    write_int(&mut entity.nbt, KEY_ENERGY, state.energy);
    write_int(&mut entity.nbt, KEY_PROGRESS, state.progress as i32);
    write_string(&mut entity.nbt, KEY_RECIPE, &state.recipe);
}

// ---------------------------------------------------------------------------
// Declaration hooks
// ---------------------------------------------------------------------------

/// Both machines have a block entity (`native_block_entity`, API 15/129).
#[allow(
    private_interfaces,
    reason = "the injected bundle is this crate's own module"
)]
pub fn on_block_entity(block: &str, entity: &mut Option<(String, Compound)>) {
    if block == GENERATOR || block == PULVERIZER {
        *entity = Some((block.to_owned(), Compound::new()));
    }
}

/// ... and tick on the server (`native_block_entity_ticks`, API 15).
#[allow(
    private_interfaces,
    reason = "the injected bundle is this crate's own module"
)]
pub fn on_block_entity_ticks(block: &str, ticks: &mut bool) {
    if block == GENERATOR || block == PULVERIZER {
        *ticks = true;
    }
}

/// The generator's fuel slot; the pulverizer's input and two outputs
/// (`native_block_entity_container`, API 16). Automation may fill the inputs and
/// take the pulverizer's outputs; the fuel slot is insert-only, as upstream's
/// `InventorySlotAssignment(0, 1, 1, 0)`.
#[allow(
    private_interfaces,
    reason = "the injected bundle is this crate's own module"
)]
pub fn on_container(block: &str, container: &mut Option<ModContainer>) {
    let (slots, inputs, outputs) = match block {
        GENERATOR => (GENERATOR_SLOTS, vec![FUEL_SLOT], vec![]),
        PULVERIZER => (
            PULVERIZER_SLOTS,
            vec![INPUT_SLOT],
            vec![OUTPUT_ONE, OUTPUT_TWO],
        ),
        _ => return,
    };
    *container = Some(ModContainer {
        slots,
        faces: std::array::from_fn(|_| (0..slots).collect()),
        inputs,
        outputs,
        menu: None,
    });
}

/// The FE buffers (`native_block_entity_energy`, API 142): the generator's
/// `BasicMachineConfig(50_000, 0, 256, 32)` and the pulverizer's
/// `(25_000, 256, 0, 32)`.
#[allow(
    private_interfaces,
    reason = "the injected bundle is this crate's own module"
)]
pub fn on_block_entity_energy(block: &str, energy: &mut Option<ModEnergyBuffer>) {
    let Some((capacity, max_insert, max_extract)) = (match block {
        GENERATOR => Some((GENERATOR_CAPACITY, 0, GENERATOR_MAX_EXTRACT)),
        PULVERIZER => Some((PULVERIZER_CAPACITY, PULVERIZER_MAX_INSERT, 0)),
        _ => None,
    }) else {
        return;
    };
    *energy = Some(ModEnergyBuffer {
        capacity,
        max_insert,
        max_extract,
        insert_faces: [max_insert > 0; 6],
        extract_faces: [max_extract > 0; 6],
    });
}

// ---------------------------------------------------------------------------
// The machine ticks
// ---------------------------------------------------------------------------

/// One server tick of either machine (`native_block_entity_tick`, API 17).
#[allow(
    private_interfaces,
    reason = "the injected bundle is this crate's own module"
)]
pub fn on_block_entity_tick(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    entity: &mut ModBlockEntity,
) {
    if entity.id == GENERATOR {
        tick_generator(game, pos, entity);
    } else if entity.id == PULVERIZER {
        tick_pulverizer(game, pos, entity);
    }
}

/// The generator: load one fuel item when upstream would, burn one tick for
/// 32 FE, then queue the adjacent transfer for `level_tick_post`.
fn tick_generator(game: &mut crate::game::Game<'_>, pos: [i32; 3], entity: &mut ModBlockEntity) {
    entity.items.resize(GENERATOR_SLOTS, None);
    let mut state = generator_state(entity);
    if state.wants_fuel()
        && let Some(stack) = entity.items[FUEL_SLOT].clone()
        && state.load_fuel(fuel_burn_time(game, &stack))
    {
        consume_fuel(game, entity, &stack);
    }
    if state.tick() > 0 {
        mark_modified(game, pos);
    }
    store_generator(entity, &state);
    if state.energy > 0 {
        queue_push(game.native_dimension(), pos);
    }
}

/// `ItemStack.shrink(1)`, with the crafting remainder (a lava bucket's bucket)
/// taking the last item's place, as upstream's generators do.
fn consume_fuel(game: &crate::game::Game<'_>, entity: &mut ModBlockEntity, stack: &ItemStack) {
    let remainder = game
        .res
        .items()
        .get(&stack.item)
        .and_then(|info| info.remainder.clone());
    let left = stack.count.saturating_sub(1);
    entity.items[FUEL_SLOT] = if left > 0 {
        let mut kept = stack.clone();
        kept.count = left;
        Some(kept)
    } else {
        remainder.map(|item| ItemStack::new(item.as_str(), 1))
    };
}

/// The pulverizer: find the matching recipe, check the output room, and let the
/// pure state machine spend FE and advance. A finished operation adds the
/// results, shrinks one input and marks the chunk.
fn tick_pulverizer(game: &mut crate::game::Game<'_>, pos: [i32; 3], entity: &mut ModBlockEntity) {
    entity.items.resize(PULVERIZER_SLOTS, None);
    let input = entity.items[INPUT_SLOT].clone();
    let recipe = input.as_ref().and_then(|stack| find_recipe(game, stack));
    let work = recipe.as_ref().and_then(|recipe| {
        let outputs = [
            entity.items[OUTPUT_ONE]
                .as_ref()
                .map(|stack| (&*stack.item, stack.count as i32)),
            entity.items[OUTPUT_TWO]
                .as_ref()
                .map(|stack| (&*stack.item, stack.count as i32)),
        ];
        can_output(recipe, outputs, |item| max_stack(game, item)).then_some(Work {
            id: &recipe.id,
            time: recipe.time,
        })
    });
    let mut state = pulverizer_state(entity);
    let step = state.tick(work);
    if step == Step::Finished
        && let Some(recipe) = recipe.as_ref()
    {
        finish_operation(game, entity, recipe);
        mark_modified(game, pos);
    }
    if step != Step::Idle {
        mark_modified(game, pos);
    }
    store_pulverizer(entity, &state);
}

/// `MachineBlockEntity.craftItem`: every result into its own output slot, then
/// one item out of the input slot.
fn finish_operation(
    game: &mut crate::game::Game<'_>,
    entity: &mut ModBlockEntity,
    recipe: &PulverizerRecipe,
) {
    for (index, result) in recipe.results.iter().enumerate() {
        grow_output(game, entity, OUTPUT_ONE + index, result);
    }
    let Some(mut input) = entity.items[INPUT_SLOT].take() else {
        return;
    };
    input.count = input.count.saturating_sub(1);
    if input.count > 0 {
        entity.items[INPUT_SLOT] = Some(input);
    }
}

fn grow_output(
    game: &crate::game::Game<'_>,
    entity: &mut ModBlockEntity,
    slot: usize,
    result: &crate::game::oritech::RecipeResult,
) {
    let limit = max_stack(game, &result.item);
    let count = result.count.max(0) as u32;
    let merged = match entity.items.get(slot).and_then(Option::as_ref) {
        Some(stack) if &*stack.item == result.item => {
            let mut stack = stack.clone();
            stack.count = stack.count.saturating_add(count).min(limit.max(0) as u32);
            stack
        }
        Some(_) => return,
        None => ItemStack::new(result.item.as_str(), count.min(limit.max(0) as u32)),
    };
    entity.items[slot] = Some(merged);
}

// ---------------------------------------------------------------------------
// The recipe cache
// ---------------------------------------------------------------------------

/// The parsed custom recipes with their engine-resolved ingredients.
struct Resolved {
    ingredient: Ingredient,
    recipe: PulverizerRecipe,
}

static RECIPES: Mutex<Option<Vec<Resolved>>> = Mutex::new(None);
static RECIPE_TICKS: AtomicU64 = AtomicU64::new(0);

fn find_recipe(game: &crate::game::Game<'_>, input: &ItemStack) -> Option<PulverizerRecipe> {
    let due = RECIPE_TICKS
        .fetch_add(1, Ordering::Relaxed)
        .is_multiple_of(RECIPE_CACHE_TICKS);
    let mut cache = RECIPES.lock().unwrap_or_else(|poison| poison.into_inner());
    if cache.is_none() || due {
        let mut resolved = Vec::new();
        let mut refused = 0;
        for recipe in game.native_recipes_of_type(PULVERIZER_RECIPES) {
            let Ok(parsed) = PulverizerRecipe::parse(&recipe.id, &recipe.json.to_string()) else {
                refused += 1;
                continue;
            };
            match game.native_recipe_ingredient(&parsed.ingredient) {
                Some(ingredient) => resolved.push(Resolved {
                    ingredient,
                    recipe: parsed,
                }),
                None => refused += 1,
            }
        }
        if refused > 0 {
            log::debug!(
                "[oritech] {refused} pulverizer recipe(s) of {} were unreadable",
                game.native_recipes_of_type(PULVERIZER_RECIPES).len()
            );
        }
        *cache = Some(resolved);
    }
    let recipes = cache.as_ref()?;
    recipes
        .iter()
        .find(|resolved| resolved.ingredient.test(input))
        .map(|resolved| resolved.recipe.clone())
}

fn reset_recipes() {
    *RECIPES.lock().unwrap_or_else(|poison| poison.into_inner()) = None;
    RECIPE_TICKS.store(0, Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// The generator's energy push
// ---------------------------------------------------------------------------

/// Generator positions queued for an energy push, by dimension.
static PUSH: Mutex<Vec<(String, [i32; 3])>> = Mutex::new(Vec::new());

fn push_queue() -> &'static Mutex<Vec<(String, [i32; 3])>> {
    &PUSH
}

fn queue_push(dimension: &str, pos: [i32; 3]) {
    let mut queue = push_queue()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    if !queue.iter().any(|(dim, at)| dim == dimension && *at == pos) {
        queue.push((dimension.to_owned(), pos));
    }
}

/// The six neighbours, their direction from the generator, and the opposite
/// direction as the pulverizer sees it.
const NEIGHBOURS: [([i32; 3], Direction, Direction); 6] = [
    ([0, -1, 0], Direction::Down, Direction::Up),
    ([0, 1, 0], Direction::Up, Direction::Down),
    ([0, 0, -1], Direction::North, Direction::South),
    ([0, 0, 1], Direction::South, Direction::North),
    ([-1, 0, 0], Direction::West, Direction::East),
    ([1, 0, 0], Direction::East, Direction::West),
];

/// After a level's tick: move the queued generator energy into adjacent
/// pulverizers through the engine's energy handlers (`native_move_energy`, API
/// 142). Both block entities are in the world here, unlike inside their own
/// ticks. The generator gives at most 256 FE a call (upstream's
/// `maxEnergyExtraction`) and the pulverizer takes at most 256 (its
/// `maxEnergyInsertion`).
#[allow(
    private_interfaces,
    reason = "the injected bundle is this crate's own module"
)]
pub fn on_level_tick_post(game: &mut crate::game::Game<'_>, dimension: &str) {
    let queued = {
        let mut queue = push_queue()
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        std::mem::take(&mut *queue)
    };
    if queued.is_empty() {
        return;
    }
    let mut later = Vec::new();
    for (dim, generator) in queued {
        if dim != dimension {
            later.push((dim, generator));
            continue;
        }
        if block_at(game, generator) != GENERATOR {
            continue;
        }
        for (offset, from_face, to_face) in NEIGHBOURS {
            let pulverizer = [
                generator[0] + offset[0],
                generator[1] + offset[1],
                generator[2] + offset[2],
            ];
            if block_at(game, pulverizer) == PULVERIZER {
                game.native_move_energy(
                    dimension,
                    generator,
                    from_face,
                    pulverizer,
                    to_face,
                    GENERATOR_MAX_EXTRACT,
                );
            }
        }
    }
    if !later.is_empty() {
        push_queue()
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .extend(later);
    }
}

// ---------------------------------------------------------------------------
// The player surface: insert by right-click, read with the command
// ---------------------------------------------------------------------------

/// The player a player hook acts for: the guest swapped in, or, in single
/// player, the owner (`native_acting_player` is none there).
fn acting_uuid(game: &crate::game::Game<'_>) -> Option<u128> {
    game.native_acting_player()
        .map(|player| player.uuid)
        .or_else(|| {
            game.native_server_players()
                .first()
                .map(|player| player.uuid)
        })
}

/// A right click on a machine: insert fuel into the generator or a matching
/// input into the pulverizer; an empty hand reads the status. There is no GUI in
/// slice 1, so the click is always claimed and never opens a menu.
#[allow(
    private_interfaces,
    reason = "the injected bundle is this crate's own module"
)]
pub fn on_right_click_block(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    _face: [i32; 3],
    canceled: &mut bool,
) {
    if *canceled {
        return;
    }
    let name = block_at(game, pos);
    if name != GENERATOR && name != PULVERIZER {
        return;
    }
    *canceled = true;
    let Some(held) = game.held().cloned() else {
        if let Some(uuid) = acting_uuid(game) {
            game.native_action_bar(uuid, Text::literal(readout(game, pos)));
        }
        return;
    };
    if name == GENERATOR {
        if !is_fuel(fuel_burn_time(game, &held)) {
            if let Some(uuid) = acting_uuid(game) {
                game.native_action_bar(uuid, Text::literal(format!("{} is not a fuel", held.item)));
            }
            return;
        }
    } else if find_recipe(game, &held).is_none() {
        if let Some(uuid) = acting_uuid(game) {
            game.native_action_bar(
                uuid,
                Text::literal(format!("{} has no pulverizing recipe", held.item)),
            );
        }
        return;
    }
    let slot = if name == GENERATOR {
        FUEL_SLOT
    } else {
        INPUT_SLOT
    };
    let limit = max_stack(game, &held.item).max(0) as u32;
    let key = (pos[0], pos[1], pos[2]);
    let Some(BlockEntity::Mod(entity)) = game.world.block_entities.get_mut(&key) else {
        return;
    };
    if entity.items.get(slot).is_none() {
        entity.items.resize(slot + 1, None);
    }
    let inserted = match entity.items[slot].take() {
        None => {
            let inserted = held.count.min(limit);
            let mut stack = held.clone();
            stack.count = inserted;
            entity.items[slot] = Some(stack);
            inserted
        }
        Some(mut stack) if stack.item == held.item => {
            let room = limit.saturating_sub(stack.count);
            let inserted = held.count.min(room);
            stack.count += inserted;
            entity.items[slot] = Some(stack);
            inserted
        }
        Some(stack) => {
            entity.items[slot] = Some(stack);
            0
        }
    };
    if inserted > 0 {
        if !game.creative {
            let _ = game.native_take_items(&held.item, inserted);
        }
        mark_modified(game, pos);
    }
}

// ---------------------------------------------------------------------------
// The `/oritech` harness controls
// ---------------------------------------------------------------------------

/// Upstream has no commands of its own: these exist so the harness (and a
/// player) can read a machine and prime a buffer without a GUI (precedent:
/// `/powah info|set`, `/mininggadgets energy`).
#[allow(
    private_interfaces,
    reason = "the injected bundle is this crate's own module"
)]
pub fn on_register_commands(commands: &mut Vec<NativeCommand>) {
    commands.push(NativeCommand::new("oritech info <pos:block_pos>"));
    commands.push(NativeCommand::op(
        "oritech energy <pos:block_pos> <amount:integer>",
        2,
    ));
}

/// `/oritech info|energy` (`native_perform_command`, API 8).
#[allow(
    private_interfaces,
    reason = "the injected bundle is this crate's own module"
)]
pub fn on_perform_command(
    game: &mut crate::game::Game<'_>,
    _player: &NativeServerPlayer,
    command: &str,
    ran: &mut bool,
) {
    let words: Vec<&str> = command.split_whitespace().collect();
    if words.first() != Some(&"oritech") {
        return;
    }
    *ran = true;
    match words.get(1).copied() {
        Some("info") => {
            let Some(pos) = parse_pos(&words[2..]) else {
                game.native_feedback(Text::literal("usage: /oritech info <x> <y> <z>"));
                return;
            };
            game.native_feedback(Text::literal(readout(game, pos)));
        }
        Some("energy") => {
            let Some((pos, amount)) = parse_pos_amount(&words[2..]) else {
                game.native_feedback(Text::literal("usage: /oritech energy <x> <y> <z> <amount>"));
                return;
            };
            let changed = game
                .native_with_mod_block_entity(pos, |entity| {
                    let capacity = match entity.id.as_str() {
                        GENERATOR => GENERATOR_CAPACITY,
                        PULVERIZER => PULVERIZER_CAPACITY,
                        _ => return None,
                    };
                    let amount = amount.clamp(0, capacity);
                    write_int(&mut entity.nbt, KEY_ENERGY, amount);
                    Some((entity.id.clone(), amount, capacity))
                })
                .flatten();
            match changed {
                Some((id, amount, capacity)) => game.native_feedback(Text::literal(format!(
                    "{id} at {pos:?}: {amount}/{capacity} FE."
                ))),
                None => game
                    .native_feedback(Text::literal(format!("{pos:?} is not an Oritech machine."))),
            }
        }
        _ => game.native_feedback(Text::literal(
            "usage: /oritech info|energy <x> <y> <z> [amount]",
        )),
    }
}

fn parse_pos(words: &[&str]) -> Option<[i32; 3]> {
    Some([
        words.first()?.parse().ok()?,
        words.get(1)?.parse().ok()?,
        words.get(2)?.parse().ok()?,
    ])
}

fn parse_pos_amount(words: &[&str]) -> Option<([i32; 3], i32)> {
    let pos = parse_pos(words)?;
    let amount = words.get(3)?.parse().ok()?;
    Some((pos, amount))
}

/// The machine's state, as `/oritech info` prints it.
fn readout(game: &crate::game::Game<'_>, pos: [i32; 3]) -> String {
    let Some(BlockEntity::Mod(entity)) = game.world.block_entities.get(&(pos[0], pos[1], pos[2]))
    else {
        return format!("{pos:?} is not a mod block entity.");
    };
    match entity.id.as_str() {
        GENERATOR => {
            let state = generator_state(entity);
            format!(
                "Basic Generator at {pos:?}: {}/{} FE, burning {}/{} ticks, fuel {}, t={}",
                state.energy,
                GENERATOR_CAPACITY,
                state.progress.max(0),
                state.max_burn,
                slot_text(entity.items.first().and_then(Option::as_ref)),
                game.world.game_time
            )
        }
        PULVERIZER => {
            let state = pulverizer_state(entity);
            let recipe = if state.recipe.is_empty() {
                "none".to_owned()
            } else {
                state.recipe.clone()
            };
            format!(
                "Pulverizer at {pos:?}: {}/{} FE, progress {} recipe {recipe}, input {}, out0 {}, out1 {}, t={}",
                state.energy,
                PULVERIZER_CAPACITY,
                state.progress,
                slot_text(entity.items.first().and_then(Option::as_ref)),
                slot_text(entity.items.get(OUTPUT_ONE).and_then(Option::as_ref)),
                slot_text(entity.items.get(OUTPUT_TWO).and_then(Option::as_ref)),
                game.world.game_time
            )
        }
        other => format!("{pos:?} is a {other}, not an Oritech machine."),
    }
}

fn slot_text(stack: Option<&ItemStack>) -> String {
    match stack {
        Some(stack) => format!("{} x{}", stack.item, stack.count),
        None => "empty".to_owned(),
    }
}

// ---------------------------------------------------------------------------
// World lifecycle
// ---------------------------------------------------------------------------

/// A world opened or closed: the parsed recipe cache and the push queue belong
/// to one world.
#[allow(
    private_interfaces,
    reason = "the injected bundle is this crate's own module"
)]
pub fn on_world_lifecycle(_game: &mut crate::game::Game<'_>, _phase: WorldPhase) {
    reset_recipes();
    push_queue()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
        .clear();
}
