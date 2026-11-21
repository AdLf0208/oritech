//! Oritech slice 1's game glue: the two machines' Parts, their declarations, the
//! saved state, the right-click, the adjacent energy move and the `/oritech`
//! command bodies.
//!
//! Every rule lives in [`crate::runtime`] (the retired bundle's `runtime.rs`,
//! byte-identical); this module reads the game, calls it and writes the result
//! back. Its tests pin what it *declares* (the containers and the two buffers),
//! that the state it writes reads back, and the exact lines the readout prints,
//! so the retired in-game assertions still match.
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use mod_api::{
    BlockUpdateFlags, Compound, Direction, ENERGY, Flow, Host, HostSide, HostTransaction,
    Identifier, Ingredient, InteractionResult, ItemPlace, ItemResource, ItemStack, LevelRef,
    ModContainer, ModEnergyBuffer, RightClickBlockEvent, ServerHost, Tag, Text, move_energy,
};

use crate::runtime::{
    CORE_OFFSET, FURNACE_CAPACITY, FURNACE_MAX_INSERT, Furnace, GENERATOR_CAPACITY,
    GENERATOR_MAX_EXTRACT, Generator, PULVERIZER_CAPACITY, PULVERIZER_MAX_INSERT, Pulverizer,
    PulverizerRecipe, RecipeResult, SmeltingRecipe, Step, Work, after_consuming, can_output,
    clamp_energy, core_quality, furnace_ticks, is_fuel, output_fits,
};

/// The furnace block and item (`BlockContent.POWERED_FURNACE_BLOCK`).
pub(crate) const FURNACE: &str = "oritech:powered_furnace_block";
/// The vanilla recipe table the furnace reads (upstream's
/// `RecipeType.SMELTING`); nothing is registered, the game already has it.
pub(crate) const SMELTING_RECIPE: &str = "minecraft:smelting";

/// The generator block and item (`BlockContent.BASIC_GENERATOR_BLOCK`).
pub(crate) const GENERATOR: &str = "oritech:basic_generator_block";
/// The pulverizer block and item (`BlockContent.PULVERIZER_BLOCK`).
pub(crate) const PULVERIZER: &str = "oritech:pulverizer_block";
/// The recipe type the jar's `data/oritech/recipe/pulverizer/*.json` names.
pub(crate) const PULVERIZER_RECIPE: &str = "oritech:pulverizer";

/// The block entity type names, in this mod's namespace; the block ids are the
/// types' own `.blocks(…)` lists.
pub(crate) const GENERATOR_TYPE: &str = "basic_generator";
pub(crate) const PULVERIZER_TYPE: &str = "pulverizer";
pub(crate) const FURNACE_TYPE: &str = "powered_furnace";

/// Upstream slot order: the generator's fuel, the pulverizer's input and its
/// two outputs, the furnace's input and its one output
/// (`InventorySlotAssignment`).
const FUEL_SLOT: usize = 0;
const INPUT_SLOT: usize = 0;
const OUTPUT_ONE: usize = 1;
const OUTPUT_TWO: usize = 2;
const GENERATOR_SLOTS: usize = 1;
const PULVERIZER_SLOTS: usize = 3;
const FURNACE_SLOTS: usize = 2;

/// Saved keys: the API 142 buffer's own `energy`, upstream's machine progress,
/// the generator's `storedBurn`, the recipe an operation belongs to, and the
/// multiblock facts upstream keeps in `connectedCores` / `coreQuality` plus the
/// assembled flag (`machine_assembled` upstream, which is a blockstate there).
const KEY_ENERGY: &str = "energy";
const KEY_PROGRESS: &str = "oritech.machine_progress";
const KEY_STORED_BURN: &str = "storedBurn";
const KEY_RECIPE: &str = "oritech.machine_recipe";
const KEY_ASSEMBLED: &str = "oritech.multiblock.assembled";
const KEY_CORES: &str = "connectedCores";
const KEY_QUALITY: &str = "coreQuality";

/// Rebuild the parsed recipe cache this often (ticks); a `/reload` lands within
/// two seconds, and the pure parser cost is paid once.
const RECIPE_CACHE_TICKS: u64 = 40;

// ---------------------------------------------------------------------------
// Saved-compound reads and writes
// ---------------------------------------------------------------------------

/// A saved int, 0 when absent or of another type.
fn read_int(nbt: &Compound, key: &str) -> i32 {
    match nbt.get(&key.into()) {
        Some(Tag::Int(value)) => *value,
        _ => 0,
    }
}

fn write_int(nbt: &mut Compound, key: &str, value: i32) {
    nbt.insert(key.into(), Tag::Int(value));
}

/// A saved string, empty when absent or of another type.
fn read_string(nbt: &Compound, key: &str) -> String {
    match nbt.get(&key.into()) {
        Some(Tag::String(value)) => value.to_string_lossy(),
        _ => String::new(),
    }
}

fn write_string(nbt: &mut Compound, key: &str, value: &str) {
    nbt.insert(key.into(), Tag::String(value.into()));
}

// ---------------------------------------------------------------------------
// The world reads
// ---------------------------------------------------------------------------

/// The block at `pos` in `level`, as its id: the retired bundle's
/// `game.res.facts().get(state).name`.
fn block_at(host: &dyn Host, level: LevelRef, pos: [i32; 3]) -> Option<String> {
    Some(level.block(host, pos)?.id(host))
}

/// The level the server ticks now, for a block-entity tick that is not told its
/// level: a tick always runs in the level in place.
pub(crate) fn in_place_level(host: &dyn Host) -> Option<LevelRef> {
    host.level(host.dimension())
}

/// `BlockPos.offset(offset)`.
fn offset_pos(pos: [i32; 3], offset: [i32; 3]) -> [i32; 3] {
    [pos[0] + offset[0], pos[1] + offset[1], pos[2] + offset[2]]
}

/// The engine's fuel value (`minecraft:cooking_fuel.burn_time`), 0 if none.
fn burn_time(host: &dyn Host, stack: &ItemStack) -> u32 {
    host.item_burn_time(stack.item()).unwrap_or(0).max(0) as u32
}

/// `ItemInfo::max_stack`, 64 when unknown (vanilla's default).
fn max_stack(host: &dyn Host, item: &str) -> i32 {
    Identifier::parse(item)
        .ok()
        .and_then(|id| host.item_key(&id))
        .and_then(|key| host.max_stack_size(key))
        .map_or(64, |size| i32::try_from(size).unwrap_or(64))
}

/// `ItemStack.shrink(1)` with the crafting remainder: one item is consumed and a
/// remainder (a lava bucket's bucket) takes the last item's place.
fn consume(host: &dyn Host, stack: &ItemStack) -> Option<ItemStack> {
    stack
        .clone()
        .shrink(1)
        .or_else(|| host.item_crafting_remainder(stack))
}

// ---------------------------------------------------------------------------
// Declarations (the retired `block_entity_container` / `block_entity_energy`)
// ---------------------------------------------------------------------------

/// The generator's fuel slot, or the pulverizer's input and two outputs:
/// upstream's `InventorySlotAssignment(0, 1, 1, 0)` / `(0, 1, 1, 2)`. A
/// declaration keyed by the block, not a hook's shared output, so a foreign
/// block's container cannot be touched.
pub(crate) fn container(block: &str) -> Option<ModContainer> {
    let (slots, inputs, outputs) = match block {
        GENERATOR => (GENERATOR_SLOTS, vec![FUEL_SLOT], vec![]),
        PULVERIZER => (
            PULVERIZER_SLOTS,
            vec![INPUT_SLOT],
            vec![OUTPUT_ONE, OUTPUT_TWO],
        ),
        // `InventorySlotAssignment(0, 1, 1, 1)`: one input, one output.
        FURNACE => (FURNACE_SLOTS, vec![INPUT_SLOT], vec![OUTPUT_ONE]),
        _ => return None,
    };
    Some(ModContainer {
        slots,
        faces: std::array::from_fn(|_| (0..slots).collect()),
        inputs,
        outputs,
        menu: None,
    })
}

/// The FE buffers: the generator's `BasicMachineConfig(50_000, 0, 256, 32)`,
/// the pulverizer's `(25_000, 256, 0, 32)` and the furnace's
/// `FurnaceConfig(10_000, 256, —, 32)`.
pub(crate) fn battery(block: &str) -> Option<ModEnergyBuffer> {
    let (capacity, max_insert, max_extract) = match block {
        GENERATOR => (GENERATOR_CAPACITY, 0, GENERATOR_MAX_EXTRACT),
        PULVERIZER => (PULVERIZER_CAPACITY, PULVERIZER_MAX_INSERT, 0),
        FURNACE => (FURNACE_CAPACITY, FURNACE_MAX_INSERT, 0),
        _ => return None,
    };
    Some(ModEnergyBuffer {
        capacity,
        max_insert,
        max_extract,
        insert_faces: [max_insert > 0; 6],
        extract_faces: [max_extract > 0; 6],
    })
}

// ---------------------------------------------------------------------------
// Saved state
// ---------------------------------------------------------------------------

/// The generator saved in a block entity's compound; a fresh one is empty.
pub(crate) fn generator_state(nbt: &Compound) -> Generator {
    Generator {
        energy: clamp_energy(read_int(nbt, KEY_ENERGY), GENERATOR_CAPACITY),
        progress: read_int(nbt, KEY_PROGRESS).max(0),
        max_burn: read_int(nbt, KEY_STORED_BURN).max(0),
    }
}

/// Write the generator's tick result back.
pub(crate) fn store_generator(nbt: &mut Compound, state: &Generator) {
    write_int(nbt, KEY_ENERGY, state.energy);
    write_int(nbt, KEY_PROGRESS, state.progress);
    write_int(nbt, KEY_STORED_BURN, state.max_burn);
}

/// The pulverizer saved in a block entity's compound; a fresh one is empty.
pub(crate) fn pulverizer_state(nbt: &Compound) -> Pulverizer {
    Pulverizer {
        energy: clamp_energy(read_int(nbt, KEY_ENERGY), PULVERIZER_CAPACITY),
        progress: read_int(nbt, KEY_PROGRESS).max(0) as u32,
        recipe: read_string(nbt, KEY_RECIPE),
    }
}

/// Write the pulverizer's tick result back.
pub(crate) fn store_pulverizer(nbt: &mut Compound, state: &Pulverizer) {
    write_int(nbt, KEY_ENERGY, state.energy);
    write_int(nbt, KEY_PROGRESS, state.progress as i32);
    write_string(nbt, KEY_RECIPE, &state.recipe);
}

/// A float saved the way a vanilla compound saves one (`Float` holds its bits).
fn read_float(nbt: &Compound, key: &str) -> f32 {
    match nbt.get(&key.into()) {
        Some(Tag::Float(bits)) => f32::from_bits(*bits),
        _ => 0.0,
    }
}

fn write_float(nbt: &mut Compound, key: &str, value: f32) {
    nbt.insert(key.into(), Tag::Float(value.to_bits()));
}

/// The Powered Furnace saved in a block entity's compound: the machine's three
/// values plus the multiblock facts (upstream's `connectedCores`, saved as the
/// one core's offset, and `coreQuality`).
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct FurnaceSaved {
    /// The buffer, progress and recipe the pure state machine keeps.
    pub(crate) machine: Furnace,
    /// `MultiblockMachine.ASSEMBLED` — a blockstate upstream, saved here.
    pub(crate) assembled: bool,
    /// The connected core's offset from the controller (`connectedCores`).
    pub(crate) core: Option<[i32; 3]>,
    /// `coreQuality`: the average of the connected cores' tiers.
    pub(crate) core_quality: f32,
}

/// The furnace saved in a block entity's compound; a fresh one is unassembled.
pub(crate) fn furnace_state(nbt: &Compound) -> FurnaceSaved {
    let cores = match nbt.get(&KEY_CORES.into()) {
        Some(Tag::List(entries)) => entries.as_slice(),
        _ => &[],
    };
    FurnaceSaved {
        machine: Furnace {
            energy: clamp_energy(read_int(nbt, KEY_ENERGY), FURNACE_CAPACITY),
            progress: read_int(nbt, KEY_PROGRESS).max(0) as u32,
            recipe: read_string(nbt, KEY_RECIPE),
        },
        assembled: matches!(nbt.get(&KEY_ASSEMBLED.into()), Some(Tag::Byte(1))),
        core: cores.iter().find_map(core_offset),
        core_quality: read_float(nbt, KEY_QUALITY),
    }
}

/// One `connectedCores` entry (`RelativePosition.toOffset`) as an offset.
fn core_offset(entry: &Tag) -> Option<[i32; 3]> {
    let Tag::Compound(compound) = entry else {
        return None;
    };
    let mut offset = [0; 3];
    for (axis, slot) in offset.iter_mut().enumerate() {
        let key = ["x", "y", "z"][axis];
        match compound.get(&key.into()) {
            Some(Tag::Int(value)) => *slot = *value,
            _ => return None,
        }
    }
    Some(offset)
}

/// Write the furnace's tick result back.
pub(crate) fn store_furnace(nbt: &mut Compound, saved: &FurnaceSaved) {
    write_int(nbt, KEY_ENERGY, saved.machine.energy);
    write_int(nbt, KEY_PROGRESS, saved.machine.progress as i32);
    write_string(nbt, KEY_RECIPE, &saved.machine.recipe);
    let assembled = i8::from(saved.assembled);
    nbt.insert(KEY_ASSEMBLED.into(), Tag::Byte(assembled));
    let cores = saved.core.map_or_else(Vec::new, |[x, y, z]| {
        vec![Tag::compound([
            ("x", Tag::Int(x)),
            ("y", Tag::Int(y)),
            ("z", Tag::Int(z)),
        ])]
    });
    nbt.insert(KEY_CORES.into(), Tag::List(cores));
    write_float(nbt, KEY_QUALITY, saved.core_quality);
}

/// What the readout prints about a furnace: its saved state plus what the world
/// says about its core and its slots (the core is resolved from the block, so
/// the line cannot claim a core that is gone).
pub(crate) struct FurnaceView<'a> {
    pub(crate) saved: &'a FurnaceSaved,
    pub(crate) core: Option<(&'a str, i32)>,
    pub(crate) input: Option<(&'a str, u32)>,
    pub(crate) output: Option<(&'a str, u32)>,
}

// ---------------------------------------------------------------------------
// The machine ticks
// ---------------------------------------------------------------------------

/// One server tick of the generator: load one fuel item when upstream would,
/// burn one tick for 32 FE, then queue the adjacent transfer for
/// `level_tick_post`, where the block entity is back in the world. True when
/// anything changed, so an idle machine does not dirty its chunk.
pub(crate) fn tick_generator(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
    nbt: &mut Compound,
    items: &mut Vec<Option<ItemStack>>,
) -> bool {
    let resized = items.len() != GENERATOR_SLOTS;
    if resized {
        items.resize(GENERATOR_SLOTS, None);
    }
    let before = generator_state(nbt);
    let mut state = before;
    let mut fuel_used = false;
    if state.wants_fuel()
        && let Some(stack) = items[FUEL_SLOT].clone()
        && state.load_fuel(burn_time(host, &stack))
    {
        items[FUEL_SLOT] = consume(host, &stack);
        fuel_used = true;
    }
    state.tick();
    store_generator(nbt, &state);
    if state.energy > 0 {
        queue_push(level, pos);
    }
    resized || fuel_used || state != before
}

/// One server tick of the pulverizer: find the matching recipe, check the
/// output room, and let the pure state machine spend FE and advance. A finished
/// operation adds the results, shrinks one input and marks the chunk.
pub(crate) fn tick_pulverizer(
    host: &mut dyn ServerHost,
    _level: LevelRef,
    _pos: [i32; 3],
    nbt: &mut Compound,
    items: &mut Vec<Option<ItemStack>>,
) -> bool {
    let resized = items.len() != PULVERIZER_SLOTS;
    if resized {
        items.resize(PULVERIZER_SLOTS, None);
    }
    let before = pulverizer_state(nbt);
    let input = items[INPUT_SLOT].clone();
    let recipe = input.as_ref().and_then(|stack| find_recipe(host, stack));
    let work = recipe.as_ref().and_then(|recipe| {
        let slot = |index: usize| {
            items.get(index).and_then(Option::as_ref).map(|stack| {
                (
                    stack.item().id(host),
                    i32::try_from(stack.count()).unwrap_or(i32::MAX),
                )
            })
        };
        let output_one = slot(OUTPUT_ONE);
        let output_two = slot(OUTPUT_TWO);
        let outputs = [
            output_one.as_ref().map(|(id, count)| (id.as_str(), *count)),
            output_two.as_ref().map(|(id, count)| (id.as_str(), *count)),
        ];
        can_output(recipe, outputs, |item| max_stack(host, item)).then_some(Work {
            id: &recipe.id,
            time: recipe.time,
        })
    });
    let mut state = before.clone();
    let step = state.tick(work);
    let finished = step == Step::Finished;
    if finished && let Some(recipe) = recipe.as_ref() {
        finish_operation(host, items, recipe);
    }
    store_pulverizer(nbt, &state);
    resized || finished || step != Step::Idle || state != before
}

/// One server tick of the Powered Furnace: a machine whose core is gone is
/// disassembled (`MultiblockMachineController.onCoreBroken`), an unassembled one
/// does nothing (`PoweredFurnaceBlockEntity.serverTick`'s `isActive` gate, which
/// keeps the progress), and an assembled one runs one vanilla smelting recipe.
pub(crate) fn tick_furnace(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
    nbt: &mut Compound,
    items: &mut Vec<Option<ItemStack>>,
) -> bool {
    let resized = items.len() != FURNACE_SLOTS;
    if resized {
        items.resize(FURNACE_SLOTS, None);
    }
    let before = furnace_state(nbt);
    let mut saved = before.clone();
    // The core is the assembly: a controller whose core block is gone (broken,
    // replaced, or never there) is not assembled, and its progress is kept —
    // upstream's `onCoreBroken` clears the assembly, not the operation.
    if saved.assembled {
        let core_alive = saved.core.is_some_and(|offset| {
            block_at(host, level, offset_pos(pos, offset))
                .is_some_and(|id| core_quality(&id).is_some())
        });
        if !core_alive {
            saved.assembled = false;
            saved.core = None;
            saved.core_quality = 0.0;
        }
    }
    let mut step = Step::Idle;
    if saved.assembled {
        let input = items[INPUT_SLOT].clone();
        let recipe = input.as_ref().and_then(|stack| find_smelting(host, stack));
        let work = recipe.as_ref().and_then(|recipe| {
            let output = items
                .get(OUTPUT_ONE)
                .and_then(Option::as_ref)
                .map(|stack| (stack.item().id(host), count_of(stack)));
            let fits = output_fits(
                output.as_ref().map(|(id, count)| (id.as_str(), *count)),
                &recipe.result,
                max_stack(host, &recipe.result.item),
            );
            fits.then_some(Work {
                id: &recipe.id,
                time: furnace_ticks(recipe.cooking_time),
            })
        });
        step = saved.machine.tick(work);
        if step == Step::Finished
            && let Some(recipe) = recipe.as_ref()
        {
            finish_furnace(host, items, &recipe.result);
        }
    }
    store_furnace(nbt, &saved);
    resized || step != Step::Idle || saved != before
}

/// `ItemStack.getCount` as an `i32`.
fn count_of(stack: &ItemStack) -> i32 {
    i32::try_from(stack.count()).unwrap_or(i32::MAX)
}

/// `MachineBlockEntity.craftItem`: every result into its own output slot, then
/// one item out of the input slot.
fn finish_operation(host: &dyn Host, items: &mut [Option<ItemStack>], recipe: &PulverizerRecipe) {
    if !consume_one(items, INPUT_SLOT) {
        return;
    }
    for (index, result) in recipe.results.iter().enumerate() {
        grow_output(host, items, OUTPUT_ONE + index, result);
    }
}

/// `PoweredFurnaceBlockEntity.craftFurnaceItem`: one item out of the input, then
/// the result into the one output slot.
fn finish_furnace(host: &dyn Host, items: &mut [Option<ItemStack>], result: &RecipeResult) {
    if !consume_one(items, INPUT_SLOT) {
        return;
    }
    grow_output(host, items, OUTPUT_ONE, result);
}

/// Take one item out of `slot`, with the crafting remainder: false, and the slot
/// unchanged, when it is empty or the stack refuses the new count.
///
/// Consume before granting the output: the SDK represents an empty slot as
/// `None` and refuses a stack of zero, so the last item must leave the slot
/// rather than be written back. A write-back would keep the item beside the
/// output it made, and the machine would produce from the same item again on
/// every operation (the 0.2.1 review's defect).
fn consume_one(items: &mut [Option<ItemStack>], slot: usize) -> bool {
    let Some(mut input) = items.get_mut(slot).and_then(Option::take) else {
        return false;
    };
    if let Some(left) = after_consuming(input.count()) {
        if input.set_count(left).is_err() {
            // A count the stack refuses cannot be consumed, so nothing is granted.
            items[slot] = Some(input);
            return false;
        }
        items[slot] = Some(input);
    }
    true
}

fn grow_output(
    host: &dyn Host,
    items: &mut [Option<ItemStack>],
    slot: usize,
    result: &RecipeResult,
) {
    let limit = u32::try_from(max_stack(host, &result.item).max(0)).unwrap_or(0);
    let count = u32::try_from(result.count.max(0)).unwrap_or(0);
    let merged = match items.get(slot).and_then(Option::as_ref) {
        Some(stack) if stack.item().id(host) == result.item => {
            let mut stack = stack.clone();
            stack
                .set_count(stack.count().saturating_add(count).min(limit))
                .ok();
            stack
        }
        Some(_) => return,
        None => {
            let item = Identifier::parse(&result.item)
                .ok()
                .and_then(|id| host.item_key(&id));
            let Some(item) = item else {
                return;
            };
            let Some(stack) = ItemStack::new(item, count.min(limit)) else {
                return;
            };
            stack
        }
    };
    if let Some(cell) = items.get_mut(slot) {
        *cell = Some(merged);
    }
}

// ---------------------------------------------------------------------------
// The recipe caches
// ---------------------------------------------------------------------------

/// A parsed recipe and its engine-resolved ingredient.
struct Resolved<T> {
    ingredient: Ingredient,
    recipe: T,
}

static PULVERIZER_RECIPES: Mutex<Option<Vec<Resolved<PulverizerRecipe>>>> = Mutex::new(None);
static SMELTING_RECIPES: Mutex<Option<Vec<Resolved<SmeltingRecipe>>>> = Mutex::new(None);
static RECIPE_TICKS: AtomicU64 = AtomicU64::new(0);

/// The first recipe of `kind` whose ingredient takes `input`: the pin's
/// recipe table, parsed by [`crate::runtime`] and matched by the engine's own
/// ingredient test, so tags match as vanilla matches them. `parse` answers the
/// recipe and the ingredient JSON text the engine resolves.
fn cached_recipe<T: Clone>(
    cache: &Mutex<Option<Vec<Resolved<T>>>>,
    host: &dyn Host,
    kind: &str,
    parse: impl Fn(&str, &str) -> Option<(T, String)>,
    input: &ItemStack,
) -> Option<T> {
    let due = RECIPE_TICKS
        .fetch_add(1, Ordering::Relaxed)
        .is_multiple_of(RECIPE_CACHE_TICKS);
    let mut cache = cache.lock().unwrap_or_else(|poison| poison.into_inner());
    if cache.is_none() || due {
        let mut resolved = Vec::new();
        for recipe in host.recipes_of_type(kind).iter() {
            let Some((parsed, ingredient_json)) = parse(&recipe.id, &recipe.json.to_string())
            else {
                continue;
            };
            let Ok(Some(ingredient)) = host.recipe_ingredient(&ingredient_json) else {
                continue;
            };
            resolved.push(Resolved {
                ingredient,
                recipe: parsed,
            });
        }
        *cache = Some(resolved);
    }
    let recipes = cache.as_ref()?;
    recipes
        .iter()
        .find(|resolved| resolved.ingredient.test(host, input))
        .map(|resolved| resolved.recipe.clone())
}

/// The pulverizing recipe an input starts, if one matches.
fn find_recipe(host: &dyn Host, input: &ItemStack) -> Option<PulverizerRecipe> {
    cached_recipe(
        &PULVERIZER_RECIPES,
        host,
        PULVERIZER_RECIPE,
        |id, json| {
            let recipe = PulverizerRecipe::parse(id, json).ok()?;
            let ingredient = recipe.ingredient.to_string();
            Some((recipe, ingredient))
        },
        input,
    )
}

/// The vanilla smelting recipe an input starts, if one matches
/// (`getRecipeFor(RecipeType.SMELTING, …)`; the game's own table, nothing
/// registered).
fn find_smelting(host: &dyn Host, input: &ItemStack) -> Option<SmeltingRecipe> {
    cached_recipe(
        &SMELTING_RECIPES,
        host,
        SMELTING_RECIPE,
        |id, json| {
            let recipe = SmeltingRecipe::parse(id, json).ok()?;
            let ingredient = recipe.ingredient.to_string();
            Some((recipe, ingredient))
        },
        input,
    )
}

// ---------------------------------------------------------------------------
// The generator's energy push
// ---------------------------------------------------------------------------

/// Generator positions queued for an energy push, by level. A ticked block
/// entity is out of the world during its own tick, so the push happens at
/// `level_tick_post`, where both machines are in the world again.
static PUSH: Mutex<Vec<(LevelRef, [i32; 3])>> = Mutex::new(Vec::new());

fn queue_push(level: LevelRef, pos: [i32; 3]) {
    let mut queue = PUSH.lock().unwrap_or_else(|poison| poison.into_inner());
    if !queue
        .iter()
        .any(|(queued, at)| *queued == level && *at == pos)
    {
        queue.push((level, pos));
    }
}

/// After a level's tick: move the queued generator energy into adjacent
/// pulverizers through the engine's energy handlers (`move_energy`, the
/// `EnergyHandlerUtil.move` counterpart of the retired `native_move_energy`).
/// The generator gives at most 256 FE a call (upstream's `maxEnergyExtraction`)
/// and the pulverizer takes at most 256 (its `maxEnergyInsertion`).
pub(crate) fn on_level_tick_post(host: &mut dyn ServerHost, level: LevelRef) {
    let queued = {
        let mut queue = PUSH.lock().unwrap_or_else(|poison| poison.into_inner());
        std::mem::take(&mut *queue)
    };
    if queued.is_empty() {
        return;
    }
    let mut later = Vec::new();
    for (queued_level, generator) in queued {
        if queued_level != level {
            later.push((queued_level, generator));
            continue;
        }
        if block_at(host, level, generator).as_deref() != Some(GENERATOR) {
            continue;
        }
        for side in Direction::ALL {
            let [dx, dy, dz] = side.normal();
            let neighbour = [generator[0] + dx, generator[1] + dy, generator[2] + dz];
            if block_at(host, level, neighbour).as_deref() != Some(PULVERIZER) {
                continue;
            }
            let max = u32::try_from(GENERATOR_MAX_EXTRACT.max(0)).unwrap_or(0);
            let _ = host.transaction(|host, tx| -> Result<u32, ()> {
                let Some(from) = level.capability(&mut *host, &ENERGY, generator, Some(side))
                else {
                    return Ok(0);
                };
                let Some(to) =
                    level.capability(&mut *host, &ENERGY, neighbour, Some(side.opposite()))
                else {
                    return Ok(0);
                };
                Ok(move_energy(&mut *host, &from, &to, max, tx).unwrap_or(0))
            });
        }
    }
    if !later.is_empty() {
        PUSH.lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .extend(later);
    }
}

// ---------------------------------------------------------------------------
// The player surface
// ---------------------------------------------------------------------------

/// A right click on a machine: insert fuel into the generator or a matching
/// input into the pulverizer or the furnace; on the furnace, a machine core in
/// the hand goes into the core position above it, and an empty hand assembles
/// the multiblock and reads the status. There is no GUI in this slice, so the
/// click is always claimed (a `Success` cancel, as the retired
/// `canceled = true`) and never opens a menu.
pub(crate) fn right_click(host: HostSide<'_>, event: &mut RightClickBlockEvent<'_>) -> Flow {
    if event.cancelled {
        return Flow::Continue;
    }
    let click = event.click;
    let pos = click.pos;
    let HostSide::Server(server) = host else {
        return Flow::Continue;
    };
    let Some(level) = server.level(server.dimension()) else {
        return Flow::Continue;
    };
    let Some(name) = block_at(&*server, level, pos) else {
        return Flow::Continue;
    };
    if name != GENERATOR && name != PULVERIZER && name != FURNACE {
        return Flow::Continue;
    }
    // A spectator's right click reaches this hook at the pinned host; it must not insert
    // into the machine, so the game-mode check runs before anything is claimed or written.
    if let Some(player) = server.player(click.player)
        && player.is_spectator(&*server) == Some(true)
    {
        return Flow::Continue;
    }
    *event.cancellation_result = InteractionResult::Success;
    let Some(held) = click.stack.cloned() else {
        if name == FURNACE
            && let Some(message) = try_assemble(server, level, pos)
        {
            say(server, click.player, &message);
        }
        if let Some(line) = info_line(server, pos) {
            say(server, click.player, &line);
        }
        return Flow::Cancel;
    };
    if name == FURNACE {
        let held_id = held.item().id(&*server);
        // `MultiblockMachineController.tryPlaceNextCore`: a core in the hand
        // fills the empty core position instead of the input slot.
        if core_quality(&held_id).is_some() {
            let message = place_core(server, level, pos, click.player, click.hand, &held);
            say(server, click.player, &message);
            return Flow::Cancel;
        }
        if find_smelting(&*server, &held).is_none() {
            let message = format!("{held_id} has no smelting recipe");
            say(server, click.player, &message);
            return Flow::Cancel;
        }
        insert_held(
            server,
            level,
            pos,
            click.player,
            click.hand,
            &held,
            INPUT_SLOT,
        );
        return Flow::Cancel;
    }
    if name == GENERATOR {
        if !is_fuel(burn_time(&*server, &held)) {
            let message = format!("{} is not a fuel", held.item().id(&*server));
            say(server, click.player, &message);
            return Flow::Cancel;
        }
    } else if find_recipe(&*server, &held).is_none() {
        let message = format!("{} has no pulverizing recipe", held.item().id(&*server));
        say(server, click.player, &message);
        return Flow::Cancel;
    }
    let slot = if name == GENERATOR {
        FUEL_SLOT
    } else {
        INPUT_SLOT
    };
    insert_held(server, level, pos, click.player, click.hand, &held, slot);
    Flow::Cancel
}

/// `MultiblockMachineController.tryPlaceNextCore`: put the held machine core in
/// the controller's missing core position, one block above, and take it out of
/// the hand (a creative hand gives for free). Answers the line to print.
fn place_core(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
    player: u128,
    hand: mod_api::Hand,
    held: &ItemStack,
) -> String {
    let target = offset_pos(pos, CORE_OFFSET);
    let held_id = held.item().id(&*host);
    match block_at(&*host, level, target) {
        Some(occupied) if occupied != "minecraft:air" => {
            return format!("the core position holds {occupied}");
        }
        _ => {}
    }
    let Some(state) = host.block_state_id(&held_id) else {
        return format!("{held_id} is not a placeable block");
    };
    let place = ItemPlace::PlayerHand { player, hand };
    let resource = ItemResource::of(held);
    let taken = host
        .transaction(|host, tx| -> Result<u32, ()> { Ok(place.extract(host, tx, &resource, 1)) })
        .unwrap_or(0);
    if taken == 0 {
        return format!("nothing was taken to place {held_id}");
    }
    let flags = BlockUpdateFlags::NEIGHBORS | BlockUpdateFlags::CLIENTS;
    if level.set_block(&mut *host, target, state, flags) != Ok(true) {
        return format!("could not place {held_id} at {target:?}");
    }
    let id = block_at(&*host, level, target).unwrap_or_default();
    format!("placed {id} at {target:?}; right-click the controller again to assemble")
}

/// `MultiblockMachineController.initMultiblock` on a click: the block one above
/// the controller must be one of the seven machine cores; then the machine is
/// assembled and the core recorded. Answers the line to print.
fn try_assemble(host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3]) -> Option<String> {
    let entity = host.block_entity(level, pos)?;
    let nbt = entity.data(&*host)?;
    let saved = furnace_state(&nbt);
    let core_pos = offset_pos(pos, CORE_OFFSET);
    let core_id = block_at(&*host, level, core_pos)?;
    let Some(quality) = core_quality(&core_id) else {
        return Some(format!(
            "missing core: place a machine core at {core_pos:?} (it holds {core_id})"
        ));
    };
    if saved.assembled && saved.core == Some(CORE_OFFSET) {
        return Some(format!("already assembled with {core_id}"));
    }
    let mut updated = saved;
    updated.assembled = true;
    updated.core = Some(CORE_OFFSET);
    updated.core_quality = f32::from(i16::try_from(quality).unwrap_or(0));
    let mut patch = Compound::new();
    store_furnace(&mut patch, &updated);
    // `merge_data` is `/data merge block`: it loads the patch and marks the
    // chunk changed, which is the `setChanged` an assembled machine needs.
    entity.merge_data(host, patch).ok()?;
    Some(format!("assembled with {core_id} (quality {quality})"))
}

/// Put `held` into the machine's input slot and take what went in out of the
/// player's hand, in one transaction.
fn insert_held(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
    player: u128,
    hand: mod_api::Hand,
    held: &ItemStack,
    slot: usize,
) {
    let Some(entity) = host.block_entity(level, pos) else {
        return;
    };
    let limit = u32::try_from(max_stack(&*host, &held.item().id(&*host)).max(0)).unwrap_or(0);
    let current = entity.item_slot(&*host, slot as u32).ok().flatten();
    let inserted = match &current {
        None => held.count().min(limit),
        Some(stack) if stack.item() == held.item() => limit.saturating_sub(stack.count()),
        Some(_) => 0,
    }
    .min(held.count());
    if inserted == 0 {
        let message = format!("{} is full", held.item().id(&*host));
        say(host, player, &message);
        return;
    }
    let place = ItemPlace::PlayerHand { player, hand };
    let resource = ItemResource::of(held);
    let _ = host.transaction(|host, tx| -> Result<u32, ()> {
        let taken = place.extract(host, tx, &resource, inserted);
        if taken == 0 {
            return Ok(0);
        }
        let mut stack = held.clone();
        stack
            .set_count(
                current
                    .as_ref()
                    .map_or(taken, |stack| stack.count() + taken),
            )
            .ok();
        if entity
            .set_item_slot(host, tx, slot as u32, Some(stack))
            .is_ok()
        {
            let _ = entity.set_changed(host);
        }
        Ok(taken)
    });
}

/// A line to the acting player's action bar, or to the command feedback when the
/// click names no player.
fn say(host: &mut dyn ServerHost, player: u128, line: &str) {
    match host.player(player) {
        Some(player) => {
            let _ = player.action_bar(host, Text::literal(line));
        }
        None => {
            let _ = host.commands().feedback(Text::literal(line));
        }
    }
}

/// `/oritech info <pos>`: the machine's line, or `None` when no machine stands
/// there.
pub(crate) fn info_line(host: &mut dyn ServerHost, pos: [i32; 3]) -> Option<String> {
    let level = host.level(host.dimension())?;
    let name = block_at(&*host, level, pos)?;
    let entity = host.block_entity(level, pos)?;
    let nbt = entity.data(&*host)?;
    let time = level.game_time(&*host).unwrap_or(0);
    let slot = |index: usize| {
        entity
            .item_slot(&*host, u32::try_from(index).ok()?)
            .ok()
            .flatten()
            .map(|stack| (stack.item().id(&*host), stack.count()))
    };
    match name.as_str() {
        GENERATOR => {
            let state = generator_state(&nbt);
            let fuel = slot(FUEL_SLOT);
            Some(generator_line(
                pos,
                &state,
                fuel.as_ref().map(|(id, count)| (id.as_str(), *count)),
                time,
            ))
        }
        PULVERIZER => {
            let state = pulverizer_state(&nbt);
            let input = slot(INPUT_SLOT);
            let output_one = slot(OUTPUT_ONE);
            let output_two = slot(OUTPUT_TWO);
            let slots = [
                input.as_ref().map(|(id, count)| (id.as_str(), *count)),
                output_one.as_ref().map(|(id, count)| (id.as_str(), *count)),
                output_two.as_ref().map(|(id, count)| (id.as_str(), *count)),
            ];
            Some(pulverizer_line(pos, &state, slots, time))
        }
        FURNACE => {
            let saved = furnace_state(&nbt);
            // The core comes from the world, not from the saved offset, so a
            // line cannot name a core that is no longer there.
            let core = saved
                .core
                .and_then(|offset| block_at(&*host, level, offset_pos(pos, offset)))
                .and_then(|id| core_quality(&id).map(|quality| (id, quality)));
            let input = slot(INPUT_SLOT);
            let output = slot(OUTPUT_ONE);
            Some(furnace_line(
                pos,
                &FurnaceView {
                    saved: &saved,
                    core: core.as_ref().map(|(id, quality)| (id.as_str(), *quality)),
                    input: input.as_ref().map(|(id, count)| (id.as_str(), *count)),
                    output: output.as_ref().map(|(id, count)| (id.as_str(), *count)),
                },
                time,
            ))
        }
        other => Some(format!("{pos:?} is a {other}, not an Oritech machine.")),
    }
}

/// `/oritech energy <pos> <amount>`: set a machine's buffer, the harness
/// control the in-game check uses. Answers the line to print, or `None` when no
/// machine stands there (a foreign block entity is left completely alone:
/// writing `energy` into a chest's saved data would be a foreign key in someone
/// else's compound).
pub(crate) fn set_energy(host: &mut dyn ServerHost, pos: [i32; 3], amount: i32) -> Option<String> {
    let level = host.level(host.dimension())?;
    let name = block_at(&*host, level, pos)?;
    let capacity = match name.as_str() {
        GENERATOR => GENERATOR_CAPACITY,
        PULVERIZER => PULVERIZER_CAPACITY,
        FURNACE => FURNACE_CAPACITY,
        _ => return None,
    };
    let entity = host.block_entity(level, pos)?;
    let energy = amount.clamp(0, capacity);
    let mut patch = Compound::new();
    write_int(&mut patch, KEY_ENERGY, energy);
    // `merge_data` is `/data merge block`: it loads the patch and marks the
    // chunk changed, which is the `setChanged` the retired bundle had to fake.
    entity
        .merge_data(host, patch)
        .ok()
        .map(|_| format!("{name} at {pos:?}: {energy}/{capacity} FE."))
}

/// A world opened or closed: the parsed recipe caches and the push queue belong
/// to one world.
pub(crate) fn reset_world_state() {
    *PULVERIZER_RECIPES
        .lock()
        .unwrap_or_else(|poison| poison.into_inner()) = None;
    *SMELTING_RECIPES
        .lock()
        .unwrap_or_else(|poison| poison.into_inner()) = None;
    RECIPE_TICKS.store(0, Ordering::Relaxed);
    PUSH.lock()
        .unwrap_or_else(|poison| poison.into_inner())
        .clear();
}

// ---------------------------------------------------------------------------
// The readout lines (byte-identical to the retired bundle's)
// ---------------------------------------------------------------------------

/// The generator's line: `/oritech info` and an empty-hand click both print it.
pub(crate) fn generator_line(
    pos: [i32; 3],
    state: &Generator,
    fuel: Option<(&str, u32)>,
    t: i64,
) -> String {
    format!(
        "Basic Generator at {pos:?}: {}/{} FE, burning {}/{} ticks, fuel {}, t={t}",
        state.energy,
        GENERATOR_CAPACITY,
        state.progress.max(0),
        state.max_burn,
        slot_text(fuel),
    )
}

/// The pulverizer's line: input, then the two outputs.
pub(crate) fn pulverizer_line(
    pos: [i32; 3],
    state: &Pulverizer,
    slots: [Option<(&str, u32)>; 3],
    t: i64,
) -> String {
    let recipe = if state.recipe.is_empty() {
        "none"
    } else {
        state.recipe.as_str()
    };
    format!(
        "Pulverizer at {pos:?}: {}/{} FE, progress {} recipe {recipe}, input {}, out0 {}, \
         out1 {}, t={t}",
        state.energy,
        PULVERIZER_CAPACITY,
        state.progress,
        slot_text(slots[0]),
        slot_text(slots[1]),
        slot_text(slots[2]),
    )
}

fn slot_text(stack: Option<(&str, u32)>) -> String {
    match stack {
        Some((item, count)) => format!("{item} x{count}"),
        None => "empty".to_owned(),
    }
}

/// The furnace's line: the multiblock state first (a machine that is not
/// assembled does nothing), then the one smelt it is working on.
pub(crate) fn furnace_line(pos: [i32; 3], view: &FurnaceView<'_>, t: i64) -> String {
    let saved = view.saved;
    let (core, quality) = view
        .core
        .map_or(("empty", 0), |(id, quality)| (id, quality));
    let recipe = if saved.machine.recipe.is_empty() {
        "none"
    } else {
        saved.machine.recipe.as_str()
    };
    format!(
        "Powered Furnace at {pos:?}: {}/{} FE, assembled {} core {core} quality {quality}, \
         progress {} recipe {recipe}, input {}, out {}, t={t}",
        saved.machine.energy,
        FURNACE_CAPACITY,
        if saved.assembled { "yes" } else { "no" },
        saved.machine.progress,
        slot_text(view.input),
        slot_text(view.output),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_generator_declares_one_insert_only_slot() {
        let slots = container(GENERATOR).expect("the generator has a container");
        assert_eq!(slots.slots, GENERATOR_SLOTS, "one fuel slot");
        assert_eq!(slots.inputs, vec![FUEL_SLOT], "fuel may be inserted");
        assert_eq!(slots.outputs, Vec::<usize>::new(), "nothing may be taken");
        assert_eq!(
            slots.faces,
            std::array::from_fn(|_| vec![FUEL_SLOT]),
            "automation reaches the slot from every face"
        );
        assert_eq!(slots.menu, None, "no GUI in this slice");
        assert!(
            container(PULVERIZER).is_some(),
            "the pulverizer has one too"
        );
        assert!(
            container("minecraft:stone").is_none(),
            "a foreign block has none"
        );
    }

    #[test]
    fn the_pulverizer_declares_an_input_and_two_outputs() {
        let slots = container(PULVERIZER).expect("the pulverizer has a container");
        assert_eq!(slots.slots, PULVERIZER_SLOTS, "input and two outputs");
        assert_eq!(slots.inputs, vec![INPUT_SLOT], "the input may be filled");
        assert_eq!(
            slots.outputs,
            vec![OUTPUT_ONE, OUTPUT_TWO],
            "both outputs may be taken"
        );
        assert_eq!(
            slots.faces,
            std::array::from_fn(|_| (0..PULVERIZER_SLOTS).collect::<Vec<_>>()),
            "every face reaches every slot"
        );
    }

    #[test]
    fn the_buffers_are_the_jars_numbers() {
        let generator = battery(GENERATOR).expect("the generator has a buffer");
        assert_eq!(generator.capacity, GENERATOR_CAPACITY);
        assert_eq!(
            generator.max_insert, 0,
            "the generator's buffer is output only"
        );
        assert_eq!(generator.max_extract, GENERATOR_MAX_EXTRACT);
        assert_eq!(generator.insert_faces, [false; 6]);
        assert_eq!(generator.extract_faces, [true; 6]);

        let pulverizer = battery(PULVERIZER).expect("the pulverizer has a buffer");
        assert_eq!(pulverizer.capacity, PULVERIZER_CAPACITY);
        assert_eq!(pulverizer.max_insert, PULVERIZER_MAX_INSERT);
        assert_eq!(
            pulverizer.max_extract, 0,
            "the pulverizer's buffer is input only"
        );
        assert_eq!(pulverizer.insert_faces, [true; 6]);
        assert_eq!(pulverizer.extract_faces, [false; 6]);
        assert!(battery("minecraft:stone").is_none());
    }

    #[test]
    fn the_tick_s_state_reads_back_from_the_compound_it_wrote() {
        let mut nbt = Compound::new();
        assert_eq!(
            generator_state(&nbt),
            Generator::new(),
            "a fresh one is empty"
        );
        let generator = Generator {
            energy: 1_440,
            progress: 1_555,
            max_burn: 1_600,
        };
        store_generator(&mut nbt, &generator);
        assert_eq!(generator_state(&nbt), generator);
        assert_eq!(
            nbt.get(&KEY_ENERGY.into()),
            Some(&Tag::Int(1_440)),
            "the buffer is the int `energy` the energy capability reads"
        );

        let mut nbt = Compound::new();
        let pulverizer = Pulverizer {
            energy: 6_800,
            progress: 11,
            recipe: "oritech:pulverizer/bone".to_owned(),
        };
        store_pulverizer(&mut nbt, &pulverizer);
        assert_eq!(pulverizer_state(&nbt), pulverizer);
        assert_eq!(
            nbt.get(&KEY_RECIPE.into()),
            Some(&Tag::String("oritech:pulverizer/bone".into())),
            "the recipe id is saved as a string"
        );
    }

    #[test]
    fn a_saved_buffer_above_its_capacity_is_clamped() {
        let mut nbt = Compound::new();
        nbt.insert(KEY_ENERGY.into(), Tag::Int(GENERATOR_CAPACITY + 5));
        assert_eq!(generator_state(&nbt).energy, GENERATOR_CAPACITY);
        nbt.insert(KEY_ENERGY.into(), Tag::Int(-5));
        assert_eq!(generator_state(&nbt).energy, 0);
    }

    #[test]
    fn the_info_lines_are_the_ones_the_e2e_asserts() {
        let generator = Generator {
            energy: 1_440,
            progress: 1_555,
            max_burn: 1_600,
        };
        assert_eq!(
            generator_line([0, 100, 0], &generator, Some(("minecraft:coal", 4)), 406),
            "Basic Generator at [0, 100, 0]: 1440/50000 FE, burning 1555/1600 ticks, \
             fuel minecraft:coal x4, t=406"
        );

        let pulverizer = Pulverizer {
            energy: 6_800,
            progress: 0,
            recipe: String::new(),
        };
        assert_eq!(
            pulverizer_line(
                [4, 100, 0],
                &pulverizer,
                [None, Some(("minecraft:bone_meal", 6)), None],
                563
            ),
            "Pulverizer at [4, 100, 0]: 6800/25000 FE, progress 0 recipe none, input empty, \
             out0 minecraft:bone_meal x6, out1 empty, t=563"
        );
    }

    #[test]
    fn a_working_pulverizer_prints_its_recipe_and_input() {
        let pulverizer = Pulverizer {
            energy: 9_648,
            progress: 11,
            recipe: "oritech:pulverizer/bone".to_owned(),
        };
        assert_eq!(
            pulverizer_line(
                [4, 100, 0],
                &pulverizer,
                [Some(("minecraft:bone", 1)), None, None],
                424
            ),
            "Pulverizer at [4, 100, 0]: 9648/25000 FE, progress 11 recipe \
             oritech:pulverizer/bone, input minecraft:bone x1, out0 empty, out1 empty, t=424"
        );
    }

    // -- slice 2: the Powered Furnace and the machine cores -------------------

    #[test]
    fn the_furnace_declares_an_input_and_one_output() {
        let slots = container(FURNACE).expect("the furnace has a container");
        assert_eq!(slots.slots, FURNACE_SLOTS, "input and output");
        assert_eq!(slots.inputs, vec![INPUT_SLOT], "the input may be filled");
        assert_eq!(slots.outputs, vec![OUTPUT_ONE], "one output may be taken");
        assert_eq!(
            slots.faces,
            std::array::from_fn(|_| (0..FURNACE_SLOTS).collect::<Vec<_>>()),
            "every face reaches both slots"
        );
        assert_eq!(slots.menu, None, "no GUI in this slice");
    }

    #[test]
    fn the_furnace_buffer_is_the_jars_numbers() {
        let furnace = battery(FURNACE).expect("the furnace has a buffer");
        assert_eq!(furnace.capacity, FURNACE_CAPACITY);
        assert_eq!(furnace.capacity, 10_000, "FurnaceConfig.energyCapacity");
        assert_eq!(furnace.max_insert, FURNACE_MAX_INSERT);
        assert_eq!(furnace.max_insert, 256, "32 * 8");
        assert_eq!(furnace.max_extract, 0, "the furnace gives nothing back");
        assert_eq!(furnace.insert_faces, [true; 6]);
        assert_eq!(furnace.extract_faces, [false; 6]);
    }

    #[test]
    fn the_furnace_s_state_reads_back_from_the_compound_it_wrote() {
        let mut nbt = Compound::new();
        assert_eq!(
            furnace_state(&nbt),
            FurnaceSaved::default(),
            "a fresh furnace is empty and unassembled"
        );
        let saved = FurnaceSaved {
            machine: Furnace {
                energy: 6_800,
                progress: 11,
                recipe: "minecraft:iron_ingot_from_smelting_iron_ore".to_owned(),
            },
            assembled: true,
            core: Some(CORE_OFFSET),
            core_quality: 2.0,
        };
        store_furnace(&mut nbt, &saved);
        assert_eq!(furnace_state(&nbt), saved);
        assert_eq!(
            nbt.get(&KEY_ENERGY.into()),
            Some(&Tag::Int(6_800)),
            "the buffer is the int `energy` the energy capability reads"
        );
        assert_eq!(
            nbt.get(&KEY_ASSEMBLED.into()),
            Some(&Tag::Byte(1)),
            "the assembled flag is a byte, as a vanilla boolean state is"
        );
        assert_eq!(
            nbt.get(&KEY_CORES.into()),
            Some(&Tag::List(vec![Tag::Compound(Compound::from_iter([
                ("x".into(), Tag::Int(CORE_OFFSET[0])),
                ("y".into(), Tag::Int(CORE_OFFSET[1])),
                ("z".into(), Tag::Int(CORE_OFFSET[2])),
            ]))])),
            "upstream's `connectedCores`: a list of offsets"
        );
        assert_eq!(
            nbt.get(&KEY_QUALITY.into()),
            Some(&Tag::Float(2.0f32.to_bits())),
            "upstream's `coreQuality`"
        );
        // Overwriting with an unassembled furnace clears the core again.
        store_furnace(&mut nbt, &FurnaceSaved::default());
        assert_eq!(furnace_state(&nbt), FurnaceSaved::default());
        assert_eq!(nbt.get(&KEY_CORES.into()), Some(&Tag::List(Vec::new())));
    }

    #[test]
    fn a_furnace_buffer_above_its_capacity_is_clamped() {
        let mut nbt = Compound::new();
        nbt.insert(KEY_ENERGY.into(), Tag::Int(FURNACE_CAPACITY + 5));
        assert_eq!(furnace_state(&nbt).machine.energy, FURNACE_CAPACITY);
        nbt.insert(KEY_ENERGY.into(), Tag::Int(-5));
        assert_eq!(furnace_state(&nbt).machine.energy, 0);
    }

    #[test]
    fn the_furnace_line_is_the_one_the_e2e_asserts() {
        let assembled = FurnaceSaved {
            machine: Furnace {
                energy: 3_200,
                progress: 11,
                recipe: "minecraft:iron_ingot_from_smelting_iron_ore".to_owned(),
            },
            assembled: true,
            core: Some(CORE_OFFSET),
            core_quality: 2.0,
        };
        assert_eq!(
            furnace_line(
                [8, 100, 0],
                &FurnaceView {
                    saved: &assembled,
                    core: Some(("oritech:machine_core_2", 2)),
                    input: Some(("minecraft:iron_ore", 1)),
                    output: None,
                },
                424
            ),
            "Powered Furnace at [8, 100, 0]: 3200/10000 FE, assembled yes core \
             oritech:machine_core_2 quality 2, progress 11 recipe \
             minecraft:iron_ingot_from_smelting_iron_ore, input minecraft:iron_ore x1, out empty, \
             t=424"
        );

        let idle = FurnaceSaved::default();
        assert_eq!(
            furnace_line(
                [8, 100, 0],
                &FurnaceView {
                    saved: &idle,
                    core: None,
                    input: None,
                    output: Some(("minecraft:iron_ingot", 1)),
                },
                7
            ),
            "Powered Furnace at [8, 100, 0]: 0/10000 FE, assembled no core empty quality 0, \
             progress 0 recipe none, input empty, out minecraft:iron_ingot x1, t=7"
        );
    }
}
