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
    BlockUpdateFlags, Compound, Direction, ENERGY, FLUIDS, Flow, FluidResource, FluidStack, Host,
    HostSide, HostTransaction, Identifier, Ingredient, InteractionResult, ItemPlace, ItemResource,
    ItemStack, LevelRef, ModContainer, ModEnergyBuffer, ModFluidTanks, RightClickBlockEvent,
    ServerHost, Tag, Text, fluid_util, move_energy,
};

use crate::runtime::{
    ADDON_BLOCKS, ADDON_MODE, Addons, Burst, COOLER_CAPACITY, COOLER_CORE_OFFSET,
    COOLER_ENERGY_PER_TICK, COOLER_MAX_INSERT, COOLER_TANK, CORE_OFFSET, Cooler, CoolerRecipe,
    EnergyTarget, FURNACE_CAPACITY, FURNACE_ENERGY_PER_TICK, FURNACE_MAX_INSERT, FURNACE_SPEED,
    FluidInput, Furnace, GENERATOR_CAPACITY, GENERATOR_MAX_EXTRACT, Generator, LAVA_CAPACITY,
    LAVA_CORE_OFFSET, LAVA_ENERGY_PER_TICK, LAVA_MAX_EXTRACT, LAVA_TANK, LavaGenRecipe,
    LavaGenerator, MACHINE_ENERGY_PER_TICK, PULVERIZER_CAPACITY, PULVERIZER_MAX_INSERT, Pulverizer,
    PulverizerRecipe, REDSTONE_ADDON, RecipeResult, SmeltingRecipe, Step, Work, addon_names,
    addon_settings, addon_slots, after_consuming, after_draining, can_output, clamp_energy,
    cooler_output_fits, core_quality, efficiency_multiplier, energy_usage, generator_burn_ticks,
    generator_output, is_fuel, lava_output_targets, output_fits, processing_ticks,
    redstone_disables, speed_multiplier, tank_matches,
};

/// The furnace block and item (`BlockContent.POWERED_FURNACE_BLOCK`).
pub(crate) const FURNACE: &str = "oritech:powered_furnace_block";
/// The Industrial Chiller (`BlockContent.COOLER_BLOCK`) — slice 3.
pub(crate) const COOLER: &str = "oritech:cooler_block";
/// The Lava Generator (`BlockContent.LAVA_GENERATOR_BLOCK`) — slice 4.
pub(crate) const LAVA: &str = "oritech:lava_generator_block";
/// The vanilla recipe table the furnace reads (upstream's
/// `RecipeType.SMELTING`); nothing is registered, the game already has it.
pub(crate) const SMELTING_RECIPE: &str = "minecraft:smelting";

/// The generator block and item (`BlockContent.BASIC_GENERATOR_BLOCK`).
pub(crate) const GENERATOR: &str = "oritech:basic_generator_block";
/// The pulverizer block and item (`BlockContent.PULVERIZER_BLOCK`).
pub(crate) const PULVERIZER: &str = "oritech:pulverizer_block";
/// The recipe type the jar's `data/oritech/recipe/pulverizer/*.json` names.
pub(crate) const PULVERIZER_RECIPE: &str = "oritech:pulverizer";
/// The recipe type the jar's `data/oritech/recipe/cooler/*.json` names.
pub(crate) const COOLER_RECIPE: &str = "oritech:cooler";
/// The recipe type the jar's `data/oritech/recipe/lavagen/*.json` names.
pub(crate) const LAVA_RECIPE: &str = "oritech:lava_generator";

/// The block entity type names, in this mod's namespace; the block ids are the
/// types' own `.blocks(…)` lists.
pub(crate) const GENERATOR_TYPE: &str = "basic_generator";
pub(crate) const PULVERIZER_TYPE: &str = "pulverizer";
pub(crate) const FURNACE_TYPE: &str = "powered_furnace";
pub(crate) const COOLER_TYPE: &str = "cooler";
pub(crate) const LAVA_TYPE: &str = "lava_generator";

/// Upstream slot order: the generator's fuel, the pulverizer's input and its
/// two outputs, the furnace's input and its one output, the cooler's one output
/// (`InventorySlotAssignment`).
const FUEL_SLOT: usize = 0;
const INPUT_SLOT: usize = 0;
const OUTPUT_ONE: usize = 1;
const OUTPUT_TWO: usize = 2;
/// The cooler's `InventorySlotAssignment(0, 0, 0, 1)` counts its slots from 0:
/// one output and no input, so its result slot is the first one.
const COOLER_OUTPUT: usize = 0;
const GENERATOR_SLOTS: usize = 1;
const PULVERIZER_SLOTS: usize = 3;
const FURNACE_SLOTS: usize = 2;
const COOLER_SLOTS: usize = 1;

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
/// `UpgradableMachineBlockEntity.remainingBurstTicks`, a sync field upstream and
/// a saved int here so a reload does not lose a charged burst.
const KEY_BURST: &str = "oritech.addon_burst";
/// `MachineBlockEntity.disabledViaRedstone`'s own key (`saveAdditional`'s
/// `nbt.putBoolean("oritech.redstone", …)`); a vanilla boolean is a `Byte`.
const KEY_REDSTONE: &str = "oritech.redstone";

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

/// A saved boolean: vanilla's `putBoolean` writes a `Byte` of 0 or 1.
fn read_bool(nbt: &Compound, key: &str) -> bool {
    matches!(nbt.get(&key.into()), Some(Tag::Byte(value)) if *value != 0)
}

fn write_bool(nbt: &mut Compound, key: &str, value: bool) {
    nbt.insert(key.into(), Tag::Byte(if value { 1 } else { 0 }));
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
        // `InventorySlotAssignment(0, 0, 0, 1)`: no input, one output — the
        // chiller's input is the fluid tank below, not a slot.
        COOLER => (COOLER_SLOTS, vec![], vec![COOLER_OUTPUT]),
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
/// the pulverizer's `(25_000, 256, 0, 32)`, the furnace's
/// `FurnaceConfig(10_000, 256, —, 32)`, the cooler's
/// `BasicMachineConfig(50_000, 256, 0, 32)` and the lava generator's
/// `(100_000, 0, 512, 64)`.
pub(crate) fn battery(block: &str) -> Option<ModEnergyBuffer> {
    let (capacity, max_insert, max_extract) = match block {
        GENERATOR => (GENERATOR_CAPACITY, 0, GENERATOR_MAX_EXTRACT),
        PULVERIZER => (PULVERIZER_CAPACITY, PULVERIZER_MAX_INSERT, 0),
        FURNACE => (FURNACE_CAPACITY, FURNACE_MAX_INSERT, 0),
        COOLER => (COOLER_CAPACITY, COOLER_MAX_INSERT, 0),
        LAVA => (LAVA_CAPACITY, 0, LAVA_MAX_EXTRACT),
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

/// The capacity of the one tank a block declares, in millibuckets.
pub(crate) fn tank_capacity(block: &str) -> Option<i32> {
    match block {
        COOLER => Some(COOLER_TANK),
        LAVA => Some(LAVA_TANK),
        _ => None,
    }
}

/// The fluid tanks a block declares through `Registrar::block_fluids`: the
/// chiller's and the lava generator's one four-bucket tank, reachable from every
/// face, fillable and not drainable (upstream's `SimpleFluidStorage` the machine
/// only takes from). The amount lives in the entity's saved `fluids` list, in
/// the shape the game reads (`Fluid` + `Amount`, millibuckets).
pub(crate) fn fluids(block: &str) -> Option<ModFluidTanks> {
    let capacity = tank_capacity(block)?;
    let all = vec![0usize];
    // `ModFluidTanks` is `#[non_exhaustive]`: it is built from its default and
    // filled in, the way `rangedpumps` declares its tank.
    let mut tanks = ModFluidTanks::default();
    tanks.tanks = vec![capacity];
    tanks.faces = [
        all.clone(),
        all.clone(),
        all.clone(),
        all.clone(),
        all.clone(),
        all,
    ];
    tanks.inputs = vec![0];
    tanks.outputs = Vec::new();
    Some(tanks)
}

// ---------------------------------------------------------------------------
// The machine addon system (slice 5)
// ---------------------------------------------------------------------------

/// The most addon slots any delivered machine has (`LavaGeneratorEntity`'s two).
const MAX_ADDON_SLOTS: usize = 2;

/// The addon block ids at a machine's own `getAddonSlots`, in slot order.
/// Upstream keeps the connected list in the block entity and fills it on a
/// click or an addon's placement; this port reads the world each tick, so an
/// addon that is broken stops applying at the next tick without a click. The
/// offsets are world offsets, not rotated by a facing this port does not model
/// (PLAN-slice5.md, deviation 1).
fn connected_addons(
    host: &dyn Host,
    level: LevelRef,
    block: &str,
    pos: [i32; 3],
) -> [Option<&'static str>; MAX_ADDON_SLOTS] {
    let mut found = [None; MAX_ADDON_SLOTS];
    for (slot, offset) in addon_slots(block).iter().take(MAX_ADDON_SLOTS).enumerate() {
        let Some(id) = block_at(host, level, offset_pos(pos, *offset)) else {
            continue;
        };
        found[slot] = ADDON_BLOCKS.iter().find(|addon| **addon == id).copied();
    }
    found
}

/// The block ids this machine gathered, packed for [`Addons::gather`].
fn addon_ids(found: &[Option<&'static str>]) -> [&'static str; MAX_ADDON_SLOTS] {
    let mut ids = [""; MAX_ADDON_SLOTS];
    for (slot, id) in found.iter().flatten().enumerate() {
        ids[slot] = id;
    }
    ids
}

/// `MachineAddonController.gatherAddonStats` over the blocks in the slots.
fn gathered(found: &[Option<&'static str>]) -> Addons {
    let ids = addon_ids(found);
    let connected = found.iter().flatten().count();
    Addons::gather(&ids[..connected], ADDON_MODE)
}

/// What a readout prints about a machine's addons: their names, the burst
/// counter against the addons' own maximum, and — only while a Control Unit
/// Addon is connected — the saved redstone gate.
pub(crate) struct AddonView {
    pub(crate) names: String,
    pub(crate) burst: i32,
    pub(crate) burst_max: i32,
    pub(crate) redstone: Option<bool>,
}

impl AddonView {
    /// A machine with no addon at all: what every line printed before this slice.
    #[cfg(test)]
    pub(crate) fn none() -> Self {
        Self {
            names: addon_names(&[]),
            burst: 0,
            burst_max: 0,
            redstone: None,
        }
    }

    /// The view for one machine's connected blocks, saved counter and redstone
    /// flag. The gate is printed only when a Control Unit Addon is connected: a
    /// machine without one has no redstone state to report.
    pub(crate) fn of(
        found: &[Option<&'static str>],
        burst: Burst,
        max: i32,
        disabled: bool,
    ) -> Self {
        let ids = addon_ids(found);
        let connected = found.iter().flatten().count();
        Self {
            names: addon_names(&ids[..connected]),
            burst: burst.remaining,
            burst_max: max,
            redstone: found.contains(&Some(REDSTONE_ADDON)).then_some(disabled),
        }
    }

    /// The field every machine line carries: `addons speed+capacitor`, plus
    /// `burst <n>/<max>` only when a burst addon sets a maximum, plus
    /// `redstone <enabled|disabled>` only while a control unit is connected.
    fn text(&self) -> String {
        let mut text = if self.burst_max > 0 {
            format!(
                "addons {} burst {}/{}",
                self.names, self.burst, self.burst_max
            )
        } else {
            format!("addons {}", self.names)
        };
        if let Some(disabled) = self.redstone {
            text.push_str(if disabled {
                " redstone disabled"
            } else {
                " redstone enabled"
            });
        }
        text
    }
}

/// The burst counter saved in a machine's compound.
pub(crate) fn burst_state(nbt: &Compound) -> Burst {
    Burst {
        remaining: read_int(nbt, KEY_BURST),
    }
}

/// Write the burst counter back.
fn store_burst(nbt: &mut Compound, burst: Burst) {
    write_int(nbt, KEY_BURST, burst.remaining);
}

/// `MachineBlockEntity.disabledViaRedstone`, as it was last saved.
pub(crate) fn redstone_state(nbt: &Compound) -> bool {
    read_bool(nbt, KEY_REDSTONE)
}

/// Write the redstone flag back, under upstream's own key.
fn store_redstone(nbt: &mut Compound, disabled: bool) {
    write_bool(nbt, KEY_REDSTONE, disabled);
}

/// The redstone gate of one tick. `RedstoneAddonBlock.neighborChanged` reads
/// `world.hasNeighborSignal(pos)` at the addon's own position and pushes the
/// answer to its controller; this port reads the same signal from the machine's
/// addon slot every tick (the scan above), so a signal that goes away releases
/// the machine at the next tick instead of waiting for another neighbour change.
/// The flag is saved (upstream's `oritech.redstone`, and the readout prints it),
/// so a machine that is not ticking still says what it was.
///
/// Answers whether the machine is disabled now, and whether the saved flag moved
/// (a moved flag is a payload write even when the machine does nothing).
fn redstone_gate(
    host: &dyn Host,
    level: LevelRef,
    block: &str,
    pos: [i32; 3],
    nbt: &mut Compound,
) -> (bool, bool) {
    let mut connected = false;
    let mut powered = false;
    for offset in addon_slots(block).iter().take(MAX_ADDON_SLOTS) {
        let at = offset_pos(pos, *offset);
        if block_at(host, level, at).as_deref() != Some(REDSTONE_ADDON) {
            continue;
        }
        connected = true;
        powered |= level.has_neighbor_signal(host, at);
    }
    let disabled = redstone_disables(connected, powered);
    let moved = redstone_state(nbt) != disabled;
    if moved {
        store_redstone(nbt, disabled);
    }
    (disabled, moved)
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

/// The pulverizer saved in a block entity's compound; a fresh one is empty. The
/// capacity is the machine's own plus its addons' (`energyBonusCapacity`).
pub(crate) fn pulverizer_state(nbt: &Compound, capacity: i32) -> Pulverizer {
    Pulverizer {
        energy: clamp_energy(read_int(nbt, KEY_ENERGY), capacity),
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

/// The multiblock facts upstream keeps on the block (`MultiblockMachine`'s
/// `machine_assembled` state) and on the cores: their offsets
/// (`connectedCores`) and their average tier (`coreQuality`).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct Assembly {
    /// `MultiblockMachine.ASSEMBLED` — a blockstate upstream, saved here.
    pub(crate) assembled: bool,
    /// The connected core's offset from the controller (`connectedCores`).
    pub(crate) core: Option<[i32; 3]>,
    /// `coreQuality`: the average of the connected cores' tiers.
    pub(crate) quality: f32,
}

/// The Powered Furnace saved in a block entity's compound: the machine's three
/// values plus the multiblock facts (upstream's `connectedCores`, saved as the
/// one core's offset, and `coreQuality`).
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct FurnaceSaved {
    /// The buffer, progress and recipe the pure state machine keeps.
    pub(crate) machine: Furnace,
    /// The assembly: assembled flag, core offset, core quality.
    pub(crate) assembly: Assembly,
}

/// The multiblock facts in a compound: the assembled byte, the one connected
/// core's offset and its quality.
fn multiblock_state(nbt: &Compound) -> Assembly {
    let cores = match nbt.get(&KEY_CORES.into()) {
        Some(Tag::List(entries)) => entries.as_slice(),
        _ => &[],
    };
    Assembly {
        assembled: matches!(nbt.get(&KEY_ASSEMBLED.into()), Some(Tag::Byte(1))),
        core: cores.iter().find_map(core_offset),
        quality: read_float(nbt, KEY_QUALITY),
    }
}

/// Write the multiblock facts back: the assembled byte, `connectedCores` as a
/// list of one offset, and `coreQuality`.
fn store_multiblock(nbt: &mut Compound, assembly: Assembly) {
    let assembled = i8::from(assembly.assembled);
    nbt.insert(KEY_ASSEMBLED.into(), Tag::Byte(assembled));
    let cores = assembly.core.map_or_else(Vec::new, |[x, y, z]| {
        vec![Tag::compound([
            ("x", Tag::Int(x)),
            ("y", Tag::Int(y)),
            ("z", Tag::Int(z)),
        ])]
    });
    nbt.insert(KEY_CORES.into(), Tag::List(cores));
    write_float(nbt, KEY_QUALITY, assembly.quality);
}

/// The furnace saved in a block entity's compound; a fresh one is unassembled.
pub(crate) fn furnace_state(nbt: &Compound, capacity: i32) -> FurnaceSaved {
    FurnaceSaved {
        machine: Furnace {
            energy: clamp_energy(read_int(nbt, KEY_ENERGY), capacity),
            progress: read_int(nbt, KEY_PROGRESS).max(0) as u32,
            recipe: read_string(nbt, KEY_RECIPE),
        },
        assembly: multiblock_state(nbt),
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
    store_multiblock(nbt, saved.assembly);
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

/// The Industrial Chiller saved in a block entity's compound: the machine's
/// three values, the multiblock facts, and the one tank.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct CoolerSaved {
    /// The buffer, progress and recipe the pure state machine keeps.
    pub(crate) machine: Cooler,
    /// The assembly: assembled flag, core offset, core quality.
    pub(crate) assembly: Assembly,
    /// The tank: its fluid and its amount in millibuckets; `None` when empty.
    pub(crate) tank: Option<(String, i32)>,
}

/// The chiller saved in a block entity's compound; a fresh one is empty.
pub(crate) fn cooler_state(nbt: &Compound, capacity: i32) -> CoolerSaved {
    CoolerSaved {
        machine: Cooler {
            energy: clamp_energy(read_int(nbt, KEY_ENERGY), capacity),
            progress: read_int(nbt, KEY_PROGRESS).max(0) as u32,
            recipe: read_string(nbt, KEY_RECIPE),
        },
        assembly: multiblock_state(nbt),
        tank: tank_state(nbt, COOLER_TANK),
    }
}

/// Write the chiller's tick result back.
pub(crate) fn store_cooler(nbt: &mut Compound, saved: &CoolerSaved) {
    write_int(nbt, KEY_ENERGY, saved.machine.energy);
    write_int(nbt, KEY_PROGRESS, saved.machine.progress as i32);
    write_string(nbt, KEY_RECIPE, &saved.machine.recipe);
    store_multiblock(nbt, saved.assembly);
    store_tank(
        nbt,
        saved
            .tank
            .as_ref()
            .map(|(fluid, amount)| (fluid.as_str(), *amount)),
        COOLER_TANK,
    );
}

/// The Lava Generator saved in a block entity's compound: the machine's four
/// values, the multiblock facts, and the one tank.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct LavaSaved {
    /// The buffer, burn and recipe the pure state machine keeps.
    pub(crate) machine: LavaGenerator,
    /// The assembly: assembled flag, core offset, core quality.
    pub(crate) assembly: Assembly,
    /// The tank: its fluid and its amount in millibuckets; `None` when empty.
    pub(crate) tank: Option<(String, i32)>,
}

/// The lava generator saved in a block entity's compound; a fresh one is empty.
pub(crate) fn lava_state(nbt: &Compound, capacity: i32) -> LavaSaved {
    LavaSaved {
        machine: LavaGenerator {
            energy: clamp_energy(read_int(nbt, KEY_ENERGY), capacity),
            progress: read_int(nbt, KEY_PROGRESS).max(0),
            max_burn: read_int(nbt, KEY_STORED_BURN).max(0),
            recipe: read_string(nbt, KEY_RECIPE),
        },
        assembly: multiblock_state(nbt),
        tank: tank_state(nbt, LAVA_TANK),
    }
}

/// Write the lava generator's tick result back: the same keys the basic
/// generator and the multiblocks use.
pub(crate) fn store_lava(nbt: &mut Compound, saved: &LavaSaved) {
    write_int(nbt, KEY_ENERGY, saved.machine.energy);
    write_int(nbt, KEY_PROGRESS, saved.machine.progress);
    write_int(nbt, KEY_STORED_BURN, saved.machine.max_burn);
    write_string(nbt, KEY_RECIPE, &saved.machine.recipe);
    store_multiblock(nbt, saved.assembly);
    store_tank(
        nbt,
        saved
            .tank
            .as_ref()
            .map(|(fluid, amount)| (fluid.as_str(), *amount)),
        LAVA_TANK,
    );
}

/// The entity's first (only) tank, as the game keeps it under `fluids`: a list
/// of `{Fluid, Amount}` compounds, an empty tank `minecraft:empty` with 0. This
/// is the shape `native_fluids.rs` reads and writes, so a foreign insert lands
/// where this reads it.
pub(crate) fn tank_state(nbt: &Compound, capacity: i32) -> Option<(String, i32)> {
    let Some(Tag::List(entries)) = nbt.get(&"fluids".into()) else {
        return None;
    };
    let Some(Tag::Compound(entry)) = entries.first() else {
        return None;
    };
    let fluid = match entry.get(&"Fluid".into()) {
        Some(Tag::String(fluid)) if fluid.to_string_lossy() != "minecraft:empty" => {
            fluid.to_string_lossy()
        }
        _ => return None,
    };
    let amount = match entry.get(&"Amount".into()) {
        Some(Tag::Int(amount)) => *amount,
        _ => 0,
    };
    let amount = amount.clamp(0, capacity);
    (amount > 0).then_some((fluid, amount))
}

/// Write the tank back in the game's shape, keeping the one entry so the tank
/// number stays in place.
fn store_tank(nbt: &mut Compound, tank: Option<(&str, i32)>, capacity: i32) {
    let (fluid, amount) = match tank {
        Some((fluid, amount)) if amount > 0 => (fluid, amount.clamp(0, capacity)),
        _ => ("minecraft:empty", 0),
    };
    let entry = Tag::compound([
        ("Fluid", Tag::String(fluid.into())),
        ("Amount", Tag::Int(amount)),
    ]);
    nbt.insert("fluids".into(), Tag::List(vec![entry]));
}

/// What the readout prints about a chiller: its saved state plus what the world
/// says about its core and its output slot.
pub(crate) struct CoolerView<'a> {
    pub(crate) saved: &'a CoolerSaved,
    pub(crate) core: Option<(&'a str, i32)>,
    pub(crate) output: Option<(&'a str, u32)>,
}

/// What the readout prints about a lava generator: its saved state plus what the
/// world says about its core (resolved from the block, so the line cannot name a
/// core that is gone).
pub(crate) struct LavaView<'a> {
    pub(crate) saved: &'a LavaSaved,
    pub(crate) core: Option<(&'a str, i32)>,
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
    level: LevelRef,
    pos: [i32; 3],
    nbt: &mut Compound,
    items: &mut Vec<Option<ItemStack>>,
) -> bool {
    let resized = items.len() != PULVERIZER_SLOTS;
    if resized {
        items.resize(PULVERIZER_SLOTS, None);
    }
    let (addons, speed, cost, mut burst) =
        addon_tick(&*host, level, PULVERIZER, pos, nbt, PULVERIZER_PROFILE);
    let (disabled, flag_moved) = redstone_gate(&*host, level, PULVERIZER, pos, nbt);
    let capacity = addons.capacity(PULVERIZER_CAPACITY);
    let before = pulverizer_state(nbt, capacity);
    let before_burst = burst;
    let mut state = before.clone();
    let mut step = Step::Idle;
    // `MachineBlockEntity.serverTick`'s first line: `disabledViaRedstone` returns
    // before the recipe lookup, so a gated machine resolves no recipe, spends
    // nothing and keeps whatever progress it had.
    if !disabled {
        let input = items[INPUT_SLOT].clone();
        let recipe = input.as_ref().and_then(|stack| find_recipe(&*host, stack));
        let work = recipe.as_ref().and_then(|recipe| {
            let outputs = pulverizer_outputs(&*host, items);
            can_output(recipe, output_refs(&outputs), |item| {
                max_stack(&*host, item)
            })
            .then_some(Work {
                id: &recipe.id,
                time: processing_ticks(recipe.time, speed),
            })
        });
        step = state.tick_with(work, cost);
        if step == Step::Finished
            && let Some(recipe) = recipe.as_ref()
        {
            finish_operation(&*host, items, recipe);
            // `supportExtraChambersAuto`: the processing addon repeats the craft
            // while another matching input and its output room remain.
            for _ in 0..addons.chambers {
                let Some(stack) = items[INPUT_SLOT].clone() else {
                    break;
                };
                let Some(next) = find_recipe(&*host, &stack) else {
                    break;
                };
                let outputs = pulverizer_outputs(&*host, items);
                if next.id != recipe.id
                    || !can_output(&next, output_refs(&outputs), |item| max_stack(&*host, item))
                {
                    break;
                }
                finish_operation(&*host, items, &next);
            }
        }
    }
    if matches!(step, Step::Working | Step::Finished) {
        burst.spend(addons.burst_ticks);
    }
    burst.recharge(addons.burst_ticks);
    store_pulverizer(nbt, &state);
    store_burst(nbt, burst);
    resized || flag_moved || step != Step::Idle || state != before || burst != before_burst
}

/// The pulverizer's two output slots, as `can_output` reads them.
fn pulverizer_outputs(host: &dyn Host, items: &[Option<ItemStack>]) -> [Option<(String, i32)>; 2] {
    let slot = |index: usize| {
        items
            .get(index)
            .and_then(Option::as_ref)
            .map(|stack| (stack.item().id(host), count_of(stack)))
    };
    [slot(OUTPUT_ONE), slot(OUTPUT_TWO)]
}

/// The borrowed form of an output list.
fn output_refs(outputs: &[Option<(String, i32)>; 2]) -> [Option<(&str, i32)>; 2] {
    [
        outputs[0].as_ref().map(|(id, count)| (id.as_str(), *count)),
        outputs[1].as_ref().map(|(id, count)| (id.as_str(), *count)),
    ]
}

/// A machine's own numbers, which its addons modulate: the FE one working tick
/// spends, the speed its base tick uses (only the furnace's is not 1), and the
/// factor its `calculateEnergyUsage` divides by (only the furnace's is not 1).
#[derive(Debug, Clone, Copy)]
struct MachineProfile {
    base_energy: i32,
    base_speed: f32,
    divisor: f64,
}

/// `OritechConfig.processingMachines.pulverizerData`.
const PULVERIZER_PROFILE: MachineProfile = MachineProfile {
    base_energy: MACHINE_ENERGY_PER_TICK,
    base_speed: 1.0,
    divisor: 1.0,
};
/// `FurnaceConfig`: 32 FE a tick, `speedMultiplier` 0.5, and the `/ 2` its own
/// `calculateEnergyUsage` adds.
const FURNACE_PROFILE: MachineProfile = MachineProfile {
    base_energy: FURNACE_ENERGY_PER_TICK,
    base_speed: FURNACE_SPEED as f32,
    divisor: 2.0,
};
/// `coolerData`.
const COOLER_PROFILE: MachineProfile = MachineProfile {
    base_energy: COOLER_ENERGY_PER_TICK,
    base_speed: 1.0,
    divisor: 1.0,
};

/// The addon side of one tick: the machine's gathered addons, its effective
/// speed (base, addons and the current burst bonus), its per-tick cost, and its
/// saved burst counter brought inside the addons' range.
fn addon_tick(
    host: &dyn Host,
    level: LevelRef,
    block: &str,
    pos: [i32; 3],
    nbt: &Compound,
    profile: MachineProfile,
) -> (Addons, f32, i32, Burst) {
    let found = connected_addons(host, level, block, pos);
    let addons = gathered(&found);
    let burst = burst_state(nbt).clamped(addons.burst_ticks);
    let bonus = burst.bonus();
    let speed = speed_multiplier(&addons, profile.base_speed, bonus);
    let cost = energy_usage(
        profile.base_energy,
        profile.divisor,
        speed,
        efficiency_multiplier(&addons, bonus),
    );
    (addons, speed, cost, burst)
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
    // The furnace's own 0.5 speed is the base the addons multiply, and its
    // `calculateEnergyUsage` halves the result of the same formula.
    let (addons, speed, cost, mut burst) =
        addon_tick(&*host, level, FURNACE, pos, nbt, FURNACE_PROFILE);
    let (disabled, flag_moved) = redstone_gate(&*host, level, FURNACE, pos, nbt);
    let capacity = addons.capacity(FURNACE_CAPACITY);
    let before = furnace_state(nbt, capacity);
    let before_burst = burst;
    let mut saved = before.clone();
    // The core is the assembly: a controller whose core block is gone (broken,
    // replaced, or never there) is not assembled, and its progress is kept —
    // upstream's `onCoreBroken` clears the assembly, not the operation.
    recheck_core(&*host, level, pos, &mut saved.assembly);
    let mut step = Step::Idle;
    // `MachineBlockEntity.serverTick`: a redstone-disabled machine returns before
    // its recipe, so the core's assembly is the other gate on the same tick.
    if saved.assembly.assembled && !disabled {
        let input = items[INPUT_SLOT].clone();
        let recipe = input
            .as_ref()
            .and_then(|stack| find_smelting(&*host, stack));
        let work = recipe.as_ref().and_then(|recipe| {
            let output = items
                .get(OUTPUT_ONE)
                .and_then(Option::as_ref)
                .map(|stack| (stack.item().id(&*host), count_of(stack)));
            let fits = output_fits(
                output.as_ref().map(|(id, count)| (id.as_str(), *count)),
                &recipe.result,
                max_stack(&*host, &recipe.result.item),
            );
            fits.then_some(Work {
                id: &recipe.id,
                time: processing_ticks(recipe.cooking_time, speed),
            })
        });
        step = saved.machine.tick_with(work, cost);
        if step == Step::Finished
            && let Some(recipe) = recipe.as_ref()
        {
            finish_furnace(&*host, items, &recipe.result);
            // `PoweredFurnaceBlockEntity.serverTick`'s chamber loop.
            for _ in 0..addons.chambers {
                let Some(stack) = items[INPUT_SLOT].clone() else {
                    break;
                };
                let Some(next) = find_smelting(&*host, &stack) else {
                    break;
                };
                let output = items
                    .get(OUTPUT_ONE)
                    .and_then(Option::as_ref)
                    .map(|stack| (stack.item().id(&*host), count_of(stack)));
                if next.id != recipe.id
                    || !output_fits(
                        output.as_ref().map(|(id, count)| (id.as_str(), *count)),
                        &next.result,
                        max_stack(&*host, &next.result.item),
                    )
                {
                    break;
                }
                finish_furnace(&*host, items, &next.result);
            }
        }
    }
    if matches!(step, Step::Working | Step::Finished) {
        burst.spend(addons.burst_ticks);
    }
    burst.recharge(addons.burst_ticks);
    store_furnace(nbt, &saved);
    store_burst(nbt, burst);
    resized || flag_moved || step != Step::Idle || saved != before || burst != before_burst
}

/// `ItemStack.getCount` as an `i32`.
fn count_of(stack: &ItemStack) -> i32 {
    i32::try_from(stack.count()).unwrap_or(i32::MAX)
}

/// `MultiblockMachineController.onCoreBroken`: a controller whose core block is
/// gone (broken, replaced, or never there) is no longer assembled, and its
/// operation's progress is kept.
fn recheck_core(host: &dyn Host, level: LevelRef, pos: [i32; 3], assembly: &mut Assembly) {
    if !assembly.assembled {
        return;
    }
    let core_alive = assembly.core.is_some_and(|offset| {
        block_at(host, level, offset_pos(pos, offset)).is_some_and(|id| core_quality(&id).is_some())
    });
    if !core_alive {
        *assembly = Assembly::default();
    }
}

/// One server tick of the Industrial Chiller: the same assembly gate as the
/// furnace, then one `oritech:cooler` craft from the fluid in the tank. A
/// finished craft takes the recipe's millibuckets out of the tank and grants its
/// one result.
pub(crate) fn tick_cooler(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
    nbt: &mut Compound,
    items: &mut Vec<Option<ItemStack>>,
) -> bool {
    let resized = items.len() != COOLER_SLOTS;
    if resized {
        items.resize(COOLER_SLOTS, None);
    }
    let (addons, speed, cost, mut burst) =
        addon_tick(&*host, level, COOLER, pos, nbt, COOLER_PROFILE);
    let (disabled, flag_moved) = redstone_gate(&*host, level, COOLER, pos, nbt);
    let capacity = addons.capacity(COOLER_CAPACITY);
    let before = cooler_state(nbt, capacity);
    let before_burst = burst;
    let mut saved = before.clone();
    recheck_core(&*host, level, pos, &mut saved.assembly);
    let mut step = Step::Idle;
    if saved.assembly.assembled && !disabled {
        // `getRecipe()` picks a recipe whose fluid the tank holds
        // (`recipeInputMatchesTank`); `canOutputRecipe` is the 64 rule.
        let recipe = saved
            .tank
            .as_ref()
            .and_then(|(fluid, amount)| find_cooler(&*host, fluid, *amount));
        let work = recipe.as_ref().and_then(|recipe| {
            let output = items
                .get(COOLER_OUTPUT)
                .and_then(Option::as_ref)
                .map(|stack| (stack.item().id(&*host), count_of(stack)));
            cooler_output_fits(
                output.as_ref().map(|(id, count)| (id.as_str(), *count)),
                &recipe.result,
            )
            .then_some(Work {
                id: &recipe.id,
                time: processing_ticks(recipe.time, speed),
            })
        });
        step = saved.machine.tick_with(work, cost);
        if step == Step::Finished
            && let Some(recipe) = recipe.as_ref()
        {
            finish_cooler(&*host, &mut saved, items, recipe);
            // `CoolerBlockEntity.craftItem`'s chamber loop: another bucket of the
            // same fluid and the 64 rule, then one more craft.
            for _ in 0..addons.chambers {
                let Some((fluid, amount)) = saved.tank.clone() else {
                    break;
                };
                if !tank_matches(Some((fluid.as_str(), amount)), &recipe.fluid) {
                    break;
                }
                let output = items
                    .get(COOLER_OUTPUT)
                    .and_then(Option::as_ref)
                    .map(|stack| (stack.item().id(&*host), count_of(stack)));
                if !cooler_output_fits(
                    output.as_ref().map(|(id, count)| (id.as_str(), *count)),
                    &recipe.result,
                ) {
                    break;
                }
                finish_cooler(&*host, &mut saved, items, recipe);
            }
        }
    }
    if matches!(step, Step::Working | Step::Finished) {
        burst.spend(addons.burst_ticks);
    }
    burst.recharge(addons.burst_ticks);
    store_cooler(nbt, &saved);
    store_burst(nbt, burst);
    resized || flag_moved || step != Step::Idle || saved != before || burst != before_burst
}

/// `CoolerBlockEntity.processCraftInstance`: take the recipe's millibuckets out
/// of the tank and grant its one result into the output slot.
fn finish_cooler(
    host: &dyn Host,
    saved: &mut CoolerSaved,
    items: &mut [Option<ItemStack>],
    recipe: &CoolerRecipe,
) {
    // The recipe was chosen because the tank holds it, so this is a re-check of
    // the same rule, not a second one.
    let Some((fluid, amount)) = saved.tank.clone() else {
        return;
    };
    if !tank_matches(Some((fluid.as_str(), amount)), &recipe.fluid) {
        return;
    }
    grow_output(host, items, COOLER_OUTPUT, &recipe.result);
    saved.tank = after_draining(amount, recipe.fluid.amount).map(|left| (fluid, left));
}

/// One server tick of the Lava Generator: the same assembly gate as the other
/// multiblocks, then one `oritech:lava_generator` burn from the fluid in the
/// tank. A loaded recipe takes its millibuckets out of the tank immediately
/// (`consumeFluidRecipeInput`) and its `time` becomes the burn left; each
/// burning tick makes 64 FE until the buffer has no room for one.
pub(crate) fn tick_lava_generator(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
    nbt: &mut Compound,
) -> bool {
    let found = connected_addons(&*host, level, LAVA, pos);
    let addons = gathered(&found);
    let (disabled, flag_moved) = redstone_gate(&*host, level, LAVA, pos, nbt);
    let mut burst = burst_state(nbt).clamped(addons.burst_ticks);
    let before_burst = burst;
    let bonus = burst.bonus();
    let speed = speed_multiplier(&addons, 1.0, bonus);
    let efficiency = efficiency_multiplier(&addons, bonus);
    let output = generator_output(LAVA_ENERGY_PER_TICK, speed, efficiency);
    let capacity = addons.capacity(LAVA_CAPACITY);
    let before = lava_state(nbt, capacity);
    let mut saved = before.clone();
    recheck_core(&*host, level, pos, &mut saved.assembly);
    let mut produced = 0;
    if saved.assembly.assembled && !disabled {
        // `UpgradableGeneratorBlockEntity.serverTick`: `tryConsumeInput` runs
        // only while no burn is left and the buffer has room for one tick; the
        // recipe comes from the tank and its fluid leaves it at load time.
        if saved.machine.wants_recipe_for(output) {
            let recipe = saved
                .tank
                .as_ref()
                .and_then(|(fluid, amount)| find_lavagen(&*host, fluid, *amount));
            if let Some(recipe) = recipe {
                let ticks = generator_burn_ticks(recipe.time, speed, efficiency);
                if saved.machine.load_recipe_for(ticks, &recipe.id, output) {
                    drain_tank(&mut saved.tank, &recipe.fluid);
                }
            } else {
                // `currentRecipe = OritechRecipe.DUMMY`: with no matching
                // recipe there is nothing to burn, and the readout says so.
                saved.machine.recipe.clear();
            }
        }
        produced = saved.machine.tick_with(output);
        // `outputEnergy()` runs at the end of `serverTick`, burning or not, but
        // only while the machine is assembled: an inactive machine returns at
        // `isActive(state)` before it, so a controller whose core is gone stops
        // pushing its stored FE.
        if saved.machine.energy > 0 {
            queue_push(level, pos);
        }
    }
    // A generator never calls `useEnergy`, so upstream never drains its burst
    // counter: a charged burst addon keeps a generator at 8x speed. Preserved.
    burst.recharge(addons.burst_ticks);
    store_lava(nbt, &saved);
    store_burst(nbt, burst);
    produced > 0 || flag_moved || saved != before || burst != before_burst
}

/// `FluidMultiblockGeneratorBlockEntity.consumeFluidRecipeInput`: the recipe's
/// millibuckets leave the tank when the burn starts, `minecraft:empty` when the
/// last of it goes. The recipe was chosen because the tank holds it, so the
/// match is a re-check of the same rule, not a second one.
fn drain_tank(tank: &mut Option<(String, i32)>, input: &FluidInput) {
    let Some((fluid, amount)) = tank.clone() else {
        return;
    };
    if !tank_matches(Some((fluid.as_str(), amount)), input) {
        return;
    }
    *tank = after_draining(amount, input.amount).map(|left| (fluid, left));
}

/// `LavaGeneratorEntity.outputEnergy`'s plan for a generator block: the most one
/// call may take out of it (`maxEnergyExtraction`) and the positions it pushes
/// to, each with the face the target's handler is queried from. The basic
/// generator walks its six neighbours; the lava generator has its own ten;
/// anything else is not a generator.
fn push_plan(block: &str) -> Option<(i32, Vec<EnergyTarget>)> {
    match block {
        GENERATOR => Some((
            GENERATOR_MAX_EXTRACT,
            Direction::ALL
                .iter()
                .map(|side| {
                    let [x, y, z] = side.normal();
                    ([x, y, z], [-x, -y, -z])
                })
                .collect(),
        )),
        LAVA => Some((LAVA_MAX_EXTRACT, lava_output_targets().to_vec())),
        _ => None,
    }
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
/// The cooler's recipes have no item ingredient, so they match a tank rather
/// than an `Ingredient` and are cached as parsed recipes only. So are the lava
/// generator's.
static COOLER_RECIPES: Mutex<Option<Vec<CoolerRecipe>>> = Mutex::new(None);
static LAVA_RECIPES: Mutex<Option<Vec<LavaGenRecipe>>> = Mutex::new(None);
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

/// The `oritech:cooler` recipe a tank's contents start: the first whose fluid
/// input the tank holds (`CoolerBlockEntity.getRecipe` →
/// `recipeInputMatchesTank`). No ingredient is resolved, because a cooler recipe
/// has no item input.
fn find_cooler(host: &dyn Host, fluid: &str, amount: i32) -> Option<CoolerRecipe> {
    let due = RECIPE_TICKS
        .fetch_add(1, Ordering::Relaxed)
        .is_multiple_of(RECIPE_CACHE_TICKS);
    let mut cache = COOLER_RECIPES
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    if cache.is_none() || due {
        let parsed = host
            .recipes_of_type(COOLER_RECIPE)
            .iter()
            .filter_map(|recipe| CoolerRecipe::parse(&recipe.id, &recipe.json.to_string()).ok())
            .collect();
        *cache = Some(parsed);
    }
    let recipes = cache.as_ref()?;
    recipes
        .iter()
        .find(|recipe| tank_matches(Some((fluid, amount)), &recipe.fluid))
        .cloned()
}

/// The `oritech:lava_generator` recipe a tank's contents start: the first whose
/// fluid input the tank holds (`FluidMultiblockGeneratorBlockEntity.getRecipe`).
/// No ingredient is resolved, because the recipe has no item input.
fn find_lavagen(host: &dyn Host, fluid: &str, amount: i32) -> Option<LavaGenRecipe> {
    let due = RECIPE_TICKS
        .fetch_add(1, Ordering::Relaxed)
        .is_multiple_of(RECIPE_CACHE_TICKS);
    let mut cache = LAVA_RECIPES
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    if cache.is_none() || due {
        let parsed = host
            .recipes_of_type(LAVA_RECIPE)
            .iter()
            .filter_map(|recipe| LavaGenRecipe::parse(&recipe.id, &recipe.json.to_string()).ok())
            .collect();
        *cache = Some(parsed);
    }
    let recipes = cache.as_ref()?;
    recipes
        .iter()
        .find(|recipe| tank_matches(Some((fluid, amount)), &recipe.fluid))
        .cloned()
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

/// The `Direction` a face normal names.
fn direction(normal: [i32; 3]) -> Option<Direction> {
    Direction::ALL
        .into_iter()
        .find(|side| side.normal() == normal)
}

/// After a level's tick: move the queued generators' energy into their targets
/// through the engine's energy handlers (`move_energy`, the
/// `EnergyHandlerUtil.move` counterpart of the retired `native_move_energy`).
///
/// Upstream's `outputEnergy` builds a cache on each `getOutputTargets` position
/// and moves into whatever handler answers, so the capability is the filter
/// here: a block with no `ENERGY` handler is skipped whatever its id, and a
/// generator gives at most its own `maxEnergyExtraction` a call (the basic
/// generator 256, the lava generator 512). The target's own `maxEnergyInsertion`
/// limits what lands.
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
        let Some(block) = block_at(host, level, generator) else {
            continue;
        };
        let Some((extract, targets)) = push_plan(&block) else {
            continue;
        };
        let max = u32::try_from(extract.max(0)).unwrap_or(0);
        for (offset, face) in targets {
            let neighbour = [
                generator[0] + offset[0],
                generator[1] + offset[1],
                generator[2] + offset[2],
            ];
            let Some(target_face) = direction(face) else {
                continue;
            };
            let source_face = target_face.opposite();
            let _ = host.transaction(|host, tx| -> Result<u32, ()> {
                let Some(from) =
                    level.capability(&mut *host, &ENERGY, generator, Some(source_face))
                else {
                    return Ok(0);
                };
                let Some(to) = level.capability(&mut *host, &ENERGY, neighbour, Some(target_face))
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
    let is_multiblock = name == FURNACE || name == COOLER || name == LAVA;
    if !is_multiblock && name != GENERATOR && name != PULVERIZER {
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
        if let Some(offset) = core_offset_for(&name)
            && let Some(message) = try_assemble(server, level, pos, offset)
        {
            say(server, click.player, &message);
        }
        if let Some(line) = info_line(server, pos) {
            say(server, click.player, &line);
        }
        return Flow::Cancel;
    };
    let held_id = held.item().id(&*server);
    // `MachineAddonBlock.setPlacedBy`: upstream notices the machine next to a
    // placed addon and asks it to scan. Here the machine's own click puts a held
    // addon in its first open slot, which is the same world state as placing the
    // block at that position.
    if addon_settings(&held_id).is_some() {
        let message = place_addon(server, level, pos, click.player, click.hand, &held, &name);
        say(server, click.player, &message);
        return Flow::Cancel;
    }
    if is_multiblock {
        // `MultiblockMachineController.tryPlaceNextCore`: a core in the hand
        // fills the empty core position instead of the machine's own slots.
        if core_quality(&held_id).is_some()
            && let Some(offset) = core_offset_for(&name)
        {
            let message = place_core(server, level, pos, click.player, click.hand, &held, offset);
            say(server, click.player, &message);
            return Flow::Cancel;
        }
    }
    if name == LAVA {
        // The lava generator's input is its tank, and a held fluid container is
        // the player's way in: the engine's own `FluidUtil` interaction empties
        // it into the block's declared tank (and would fill it from a container
        // if the tank were drainable — it is not). The two bucket slots upstream
        // adds are not declared here (PLAN-slice4.md, deviation 1).
        let held_id = held.item().id(&*server);
        let moved = match server.player(click.player) {
            Some(player) => {
                let side = direction(click.hit.face);
                fluid_util::interact_with_fluid_handler(
                    &mut *server,
                    player,
                    click.hand,
                    level,
                    pos,
                    side,
                )
            }
            None => false,
        };
        if moved {
            if let Some(line) = info_line(server, pos) {
                say(server, click.player, &line);
            }
        } else {
            let message = format!(
                "{held_id} has nowhere to go: the lava generator's tank takes a fluid container"
            );
            say(server, click.player, &message);
        }
        return Flow::Cancel;
    }
    if name == COOLER {
        // No input slot: the chiller's input is its fluid tank, which a pipe or
        // `/oritech fluid` fills. A held item has nowhere to go.
        let held_id = held.item().id(&*server);
        let message = format!("{held_id} has nowhere to go: the chiller takes fluid, not items");
        say(server, click.player, &message);
        return Flow::Cancel;
    }
    if name == FURNACE {
        let held_id = held.item().id(&*server);
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

/// The core offset a multiblock controller assembles at: the furnace's one
/// block above, the chiller's one block north. `None` for anything else, which
/// is the click path's "not a machine" test.
fn core_offset_for(block: &str) -> Option<[i32; 3]> {
    match block {
        FURNACE => Some(CORE_OFFSET),
        COOLER => Some(COOLER_CORE_OFFSET),
        LAVA => Some(LAVA_CORE_OFFSET),
        _ => None,
    }
}

/// `MultiblockMachineController.tryPlaceNextCore`: put the held machine core in
/// the controller's missing core position (`offset`), and take it out of the
/// hand (a creative hand gives for free). Answers the line to print.
fn place_core(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
    player: u128,
    hand: mod_api::Hand,
    held: &ItemStack,
    offset: [i32; 3],
) -> String {
    let target = offset_pos(pos, offset);
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

/// `MachineAddonBlock.setPlacedBy` from the machine's side: put `held` in the
/// machine's first open addon slot and take it out of the hand (a creative hand
/// gives for free). Answers the line to print.
fn place_addon(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
    player: u128,
    hand: mod_api::Hand,
    held: &ItemStack,
    block: &str,
) -> String {
    let held_id = held.item().id(&*host);
    let target = addon_slots(block)
        .iter()
        .map(|offset| offset_pos(pos, *offset))
        .find(|target| block_at(&*host, level, *target).is_none_or(|id| id == "minecraft:air"));
    let Some(target) = target else {
        return format!("{held_id} has no open addon slot on this machine");
    };
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
    format!("placed {id} at {target:?} (the machine scans its slot on its next tick)")
}

/// `MultiblockMachineController.initMultiblock` on a click: the block at
/// `offset` from the controller must be one of the seven machine cores; then the
/// machine is assembled and the core recorded. Answers the line to print.
fn try_assemble(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
    offset: [i32; 3],
) -> Option<String> {
    let entity = host.block_entity(level, pos)?;
    let nbt = entity.data(&*host)?;
    let saved = multiblock_state(&nbt);
    let core_pos = offset_pos(pos, offset);
    let core_id = block_at(&*host, level, core_pos)?;
    let Some(quality) = core_quality(&core_id) else {
        return Some(format!(
            "missing core: place a machine core at {core_pos:?} (it holds {core_id})"
        ));
    };
    if saved.assembled && saved.core == Some(offset) {
        return Some(format!("already assembled with {core_id}"));
    }
    let updated = Assembly {
        assembled: true,
        core: Some(offset),
        quality: f32::from(i16::try_from(quality).unwrap_or(0)),
    };
    let mut patch = Compound::new();
    store_multiblock(&mut patch, updated);
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
            let found = connected_addons(&*host, level, PULVERIZER, pos);
            let addons = gathered(&found);
            let burst = burst_state(&nbt).clamped(addons.burst_ticks);
            let capacity = addons.capacity(PULVERIZER_CAPACITY);
            let state = pulverizer_state(&nbt, capacity);
            let input = slot(INPUT_SLOT);
            let output_one = slot(OUTPUT_ONE);
            let output_two = slot(OUTPUT_TWO);
            let slots = [
                input.as_ref().map(|(id, count)| (id.as_str(), *count)),
                output_one.as_ref().map(|(id, count)| (id.as_str(), *count)),
                output_two.as_ref().map(|(id, count)| (id.as_str(), *count)),
            ];
            Some(pulverizer_line(
                pos,
                &state,
                slots,
                capacity,
                &AddonView::of(&found, burst, addons.burst_ticks, redstone_state(&nbt)),
                time,
            ))
        }
        FURNACE => {
            let found = connected_addons(&*host, level, FURNACE, pos);
            let addons = gathered(&found);
            let burst = burst_state(&nbt).clamped(addons.burst_ticks);
            let capacity = addons.capacity(FURNACE_CAPACITY);
            let saved = furnace_state(&nbt, capacity);
            // The core comes from the world, not from the saved offset, so a
            // line cannot name a core that is no longer there.
            let core = saved
                .assembly
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
                capacity,
                &AddonView::of(&found, burst, addons.burst_ticks, redstone_state(&nbt)),
                time,
            ))
        }
        COOLER => {
            let found = connected_addons(&*host, level, COOLER, pos);
            let addons = gathered(&found);
            let burst = burst_state(&nbt).clamped(addons.burst_ticks);
            let capacity = addons.capacity(COOLER_CAPACITY);
            let saved = cooler_state(&nbt, capacity);
            let core = saved
                .assembly
                .core
                .and_then(|offset| block_at(&*host, level, offset_pos(pos, offset)))
                .and_then(|id| core_quality(&id).map(|quality| (id, quality)));
            let output = slot(COOLER_OUTPUT);
            Some(cooler_line(
                pos,
                &CoolerView {
                    saved: &saved,
                    core: core.as_ref().map(|(id, quality)| (id.as_str(), *quality)),
                    output: output.as_ref().map(|(id, count)| (id.as_str(), *count)),
                },
                capacity,
                &AddonView::of(&found, burst, addons.burst_ticks, redstone_state(&nbt)),
                time,
            ))
        }
        LAVA => {
            let found = connected_addons(&*host, level, LAVA, pos);
            let addons = gathered(&found);
            let burst = burst_state(&nbt).clamped(addons.burst_ticks);
            let capacity = addons.capacity(LAVA_CAPACITY);
            let saved = lava_state(&nbt, capacity);
            let core = saved
                .assembly
                .core
                .and_then(|offset| block_at(&*host, level, offset_pos(pos, offset)))
                .and_then(|id| core_quality(&id).map(|quality| (id, quality)));
            Some(lava_line(
                pos,
                &LavaView {
                    saved: &saved,
                    core: core.as_ref().map(|(id, quality)| (id.as_str(), *quality)),
                },
                capacity,
                &AddonView::of(&found, burst, addons.burst_ticks, redstone_state(&nbt)),
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
    let found = connected_addons(&*host, level, &name, pos);
    let addons = gathered(&found);
    let capacity = match name.as_str() {
        GENERATOR => GENERATOR_CAPACITY,
        PULVERIZER => addons.capacity(PULVERIZER_CAPACITY),
        FURNACE => addons.capacity(FURNACE_CAPACITY),
        COOLER => addons.capacity(COOLER_CAPACITY),
        LAVA => addons.capacity(LAVA_CAPACITY),
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

/// `/oritech fluid <pos> <fluid> <amount>`: fill a machine's tank through the
/// engine's own `native_fluid_insert`, so the *declaration* is what is being
/// proved — the game routes the fluid by the tanks and faces the block declared,
/// and refuses a block that declares none. Answers the line to print, or `None`
/// when no machine with a tank stands there.
pub(crate) fn set_fluid(
    host: &mut dyn ServerHost,
    pos: [i32; 3],
    fluid: &str,
    amount: i32,
) -> Option<String> {
    let level = host.level(host.dimension())?;
    let name = block_at(&*host, level, pos)?;
    let capacity = tank_capacity(&name)?;
    let asked = amount.max(0);
    // Through the engine's own `horizon:fluids` handler for this block: the game
    // builds it from the tanks the block *declared*, so a block that declared
    // none accepts nothing, and a fluid the pack does not know is not even
    // nameable. That is what makes the reply evidence of the declaration.
    let accepted = host.fluid_key(fluid).map_or(0, |key| {
        let resource = FluidResource::of(&FluidStack::new(key, 1));
        let Some(handler) = level.capability(&mut *host, &FLUIDS, pos, Some(Direction::Up)) else {
            return 0;
        };
        host.transaction(|host, tx| -> Result<u32, ()> {
            Ok(handler.insert_any(host, tx, &resource, u32::try_from(asked).unwrap_or(0)))
        })
        .unwrap_or(0)
    });
    let entity = host.block_entity(level, pos)?;
    let nbt = entity.data(&*host)?;
    let (tank_fluid, tank_amount) =
        tank_state(&nbt, capacity).unwrap_or_else(|| ("empty".to_owned(), 0));
    Some(fluid_line(
        &name,
        pos,
        &tank_fluid,
        tank_amount,
        asked,
        i32::try_from(accepted).unwrap_or(i32::MAX),
        capacity,
    ))
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
    *COOLER_RECIPES
        .lock()
        .unwrap_or_else(|poison| poison.into_inner()) = None;
    *LAVA_RECIPES
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
    capacity: i32,
    addons: &AddonView,
    t: i64,
) -> String {
    let recipe = if state.recipe.is_empty() {
        "none"
    } else {
        state.recipe.as_str()
    };
    format!(
        "Pulverizer at {pos:?}: {}/{} FE, progress {} recipe {recipe}, input {}, out0 {}, \
         out1 {}, {}, t={t}",
        state.energy,
        capacity,
        state.progress,
        slot_text(slots[0]),
        slot_text(slots[1]),
        slot_text(slots[2]),
        addons.text(),
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
pub(crate) fn furnace_line(
    pos: [i32; 3],
    view: &FurnaceView<'_>,
    capacity: i32,
    addons: &AddonView,
    t: i64,
) -> String {
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
         progress {} recipe {recipe}, input {}, out {}, {}, t={t}",
        saved.machine.energy,
        capacity,
        if saved.assembly.assembled {
            "yes"
        } else {
            "no"
        },
        saved.machine.progress,
        slot_text(view.input),
        slot_text(view.output),
        addons.text(),
    )
}

/// A tank as a line prints it: the fluid and its amount against the block's
/// declared capacity, or `empty`.
fn tank_text(tank: Option<(&str, i32)>, capacity: i32) -> String {
    match tank {
        Some((fluid, amount)) => format!("{fluid} {amount}/{capacity} mB"),
        None => format!("empty 0/{capacity} mB"),
    }
}

/// The chiller's line: the assembly first, then the tank and the one craft.
pub(crate) fn cooler_line(
    pos: [i32; 3],
    view: &CoolerView<'_>,
    capacity: i32,
    addons: &AddonView,
    t: i64,
) -> String {
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
        "Industrial Chiller at {pos:?}: {}/{} FE, assembled {} core {core} quality {quality}, \
         progress {} recipe {recipe}, tank {}, out {}, {}, t={t}",
        saved.machine.energy,
        capacity,
        if saved.assembly.assembled {
            "yes"
        } else {
            "no"
        },
        saved.machine.progress,
        tank_text(
            saved
                .tank
                .as_ref()
                .map(|(fluid, amount)| (fluid.as_str(), *amount)),
            COOLER_TANK,
        ),
        slot_text(view.output),
        addons.text(),
    )
}

/// The lava generator's line: the assembly first (a machine that is not
/// assembled burns nothing), then the burn and the tank.
pub(crate) fn lava_line(
    pos: [i32; 3],
    view: &LavaView<'_>,
    capacity: i32,
    addons: &AddonView,
    t: i64,
) -> String {
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
        "Lava Generator at {pos:?}: {}/{} FE, assembled {} core {core} quality {quality}, \
         burning {}/{} ticks recipe {recipe}, tank {}, {}, t={t}",
        saved.machine.energy,
        capacity,
        if saved.assembly.assembled {
            "yes"
        } else {
            "no"
        },
        saved.machine.progress.max(0),
        saved.machine.max_burn,
        tank_text(
            saved
                .tank
                .as_ref()
                .map(|(fluid, amount)| (fluid.as_str(), *amount)),
            LAVA_TANK,
        ),
        addons.text(),
    )
}

/// The line `/oritech fluid` answers: what the tank holds after the game's own
/// insert, and how much of the request it took.
pub(crate) fn fluid_line(
    name: &str,
    pos: [i32; 3],
    fluid: &str,
    amount: i32,
    asked: i32,
    accepted: i32,
    capacity: i32,
) -> String {
    let tank = match fluid {
        "empty" => None,
        fluid => Some((fluid, amount)),
    };
    format!(
        "{name} at {pos:?}: tank {} (asked {asked}, accepted {accepted})",
        tank_text(tank, capacity)
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
        assert_eq!(pulverizer_state(&nbt, PULVERIZER_CAPACITY), pulverizer);
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
                PULVERIZER_CAPACITY,
                &AddonView::none(),
                563
            ),
            "Pulverizer at [4, 100, 0]: 6800/25000 FE, progress 0 recipe none, input empty, \
             out0 minecraft:bone_meal x6, out1 empty, addons none, t=563"
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
                PULVERIZER_CAPACITY,
                &AddonView::none(),
                424
            ),
            "Pulverizer at [4, 100, 0]: 9648/25000 FE, progress 11 recipe \
             oritech:pulverizer/bone, input minecraft:bone x1, out0 empty, out1 empty, \
             addons none, t=424"
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
            furnace_state(&nbt, FURNACE_CAPACITY),
            FurnaceSaved::default(),
            "a fresh furnace is empty and unassembled"
        );
        let saved = FurnaceSaved {
            machine: Furnace {
                energy: 6_800,
                progress: 11,
                recipe: "minecraft:iron_ingot_from_smelting_iron_ore".to_owned(),
            },
            assembly: Assembly {
                assembled: true,
                core: Some(CORE_OFFSET),
                quality: 2.0,
            },
        };
        store_furnace(&mut nbt, &saved);
        assert_eq!(furnace_state(&nbt, FURNACE_CAPACITY), saved);
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
        assert_eq!(
            furnace_state(&nbt, FURNACE_CAPACITY),
            FurnaceSaved::default()
        );
        assert_eq!(nbt.get(&KEY_CORES.into()), Some(&Tag::List(Vec::new())));
    }

    #[test]
    fn a_furnace_buffer_above_its_capacity_is_clamped() {
        let mut nbt = Compound::new();
        nbt.insert(KEY_ENERGY.into(), Tag::Int(FURNACE_CAPACITY + 5));
        assert_eq!(
            furnace_state(&nbt, FURNACE_CAPACITY).machine.energy,
            FURNACE_CAPACITY
        );
        nbt.insert(KEY_ENERGY.into(), Tag::Int(-5));
        assert_eq!(furnace_state(&nbt, FURNACE_CAPACITY).machine.energy, 0);
    }

    #[test]
    fn the_furnace_line_is_the_one_the_e2e_asserts() {
        let assembled = FurnaceSaved {
            machine: Furnace {
                energy: 3_200,
                progress: 11,
                recipe: "minecraft:iron_ingot_from_smelting_iron_ore".to_owned(),
            },
            assembly: Assembly {
                assembled: true,
                core: Some(CORE_OFFSET),
                quality: 2.0,
            },
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
                FURNACE_CAPACITY,
                &AddonView::none(),
                424
            ),
            "Powered Furnace at [8, 100, 0]: 3200/10000 FE, assembled yes core \
             oritech:machine_core_2 quality 2, progress 11 recipe \
             minecraft:iron_ingot_from_smelting_iron_ore, input minecraft:iron_ore x1, out empty, \
             addons none, t=424"
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
                FURNACE_CAPACITY,
                &AddonView::none(),
                7
            ),
            "Powered Furnace at [8, 100, 0]: 0/10000 FE, assembled no core empty quality 0, \
             progress 0 recipe none, input empty, out minecraft:iron_ingot x1, addons none, t=7"
        );
    }

    // -- slice 3: the Industrial Chiller --------------------------------------

    #[test]
    fn the_cooler_declares_one_output_slot_and_no_item_input() {
        let slots = container(COOLER).expect("the chiller has a container");
        assert_eq!(slots.slots, COOLER_SLOTS, "one slot, and it is the output");
        assert_eq!(
            slots.inputs,
            Vec::<usize>::new(),
            "InventorySlotAssignment(0, 0, 0, 1): the chiller's input is fluid"
        );
        assert_eq!(
            slots.outputs,
            vec![COOLER_OUTPUT],
            "the one result may be taken"
        );
        assert_eq!(
            slots.faces,
            std::array::from_fn(|_| vec![COOLER_OUTPUT]),
            "automation reaches the output from every face"
        );
        assert_eq!(slots.menu, None, "no GUI in this slice");
    }

    #[test]
    fn the_cooler_buffer_is_the_jars_numbers() {
        let cooler = battery(COOLER).expect("the chiller has a buffer");
        assert_eq!(cooler.capacity, 50_000, "coolerData.energyCapacity");
        assert_eq!(cooler.max_insert, 256, "32 * 8");
        assert_eq!(cooler.max_extract, 0, "the chiller gives nothing back");
        assert_eq!(cooler.insert_faces, [true; 6]);
        assert_eq!(cooler.extract_faces, [false; 6]);
    }

    #[test]
    fn the_cooler_declares_one_four_bucket_tank_a_neighbour_can_fill() {
        let tanks = fluids(COOLER).expect("the chiller has a tank");
        assert_eq!(tanks.tanks, vec![COOLER_TANK], "one four-bucket tank");
        assert_eq!(tanks.inputs, vec![0], "a neighbour may fill it");
        assert_eq!(
            tanks.outputs,
            Vec::<usize>::new(),
            "nothing may drain it: the machine only takes fluid"
        );
        assert_eq!(
            tanks.faces,
            std::array::from_fn(|_| vec![0]),
            "every face reaches the tank"
        );
        assert!(
            fluids("minecraft:stone").is_none(),
            "a foreign block has none"
        );
        assert!(
            fluids(FURNACE).is_none(),
            "the furnace's input is an item, not a fluid"
        );
    }

    #[test]
    fn each_multiblock_assembles_at_its_own_core_offset() {
        assert_eq!(
            core_offset_for(FURNACE),
            Some(CORE_OFFSET),
            "the furnace's core is one block above"
        );
        assert_eq!(
            core_offset_for(COOLER),
            Some(COOLER_CORE_OFFSET),
            "the chiller's core is one block north"
        );
        assert_eq!(
            core_offset_for(GENERATOR),
            None,
            "a plain machine has no core"
        );
        assert_eq!(core_offset_for(PULVERIZER), None);
        assert_eq!(core_offset_for("minecraft:stone"), None);
    }

    #[test]
    fn the_cooler_s_state_reads_back_from_the_compound_it_wrote() {
        let mut nbt = Compound::new();
        assert_eq!(
            cooler_state(&nbt, COOLER_CAPACITY),
            CoolerSaved::default(),
            "a fresh chiller is empty, unassembled and has an empty tank"
        );
        let saved = CoolerSaved {
            machine: Cooler {
                energy: 43_600,
                progress: 11,
                recipe: "oritech:cooler/ice".to_owned(),
            },
            assembly: Assembly {
                assembled: true,
                core: Some(COOLER_CORE_OFFSET),
                quality: 2.0,
            },
            tank: Some(("minecraft:water".to_owned(), 1_000)),
        };
        store_cooler(&mut nbt, &saved);
        assert_eq!(cooler_state(&nbt, COOLER_CAPACITY), saved);
        assert_eq!(
            nbt.get(&KEY_ENERGY.into()),
            Some(&Tag::Int(43_600)),
            "the buffer is the int `energy` the energy capability reads"
        );
        assert_eq!(
            nbt.get(&KEY_ASSEMBLED.into()),
            Some(&Tag::Byte(1)),
            "the assembled flag is a byte, as a vanilla boolean state is"
        );
        assert_eq!(
            nbt.get(&"fluids".into()),
            Some(&Tag::List(vec![Tag::compound([
                ("Fluid", Tag::String("minecraft:water".into())),
                ("Amount", Tag::Int(1_000)),
            ])])),
            "the tank is the game's own `fluids` shape, so `fluid_insert` writes here too"
        );
        // Drained: the entry stays and reads as `minecraft:empty`.
        store_cooler(
            &mut nbt,
            &CoolerSaved {
                tank: None,
                ..saved
            },
        );
        assert_eq!(
            tank_state(&nbt, COOLER_TANK),
            None,
            "an empty tank is no tank"
        );
        assert_eq!(
            nbt.get(&"fluids".into()),
            Some(&Tag::List(vec![Tag::compound([
                ("Fluid", Tag::String("minecraft:empty".into())),
                ("Amount", Tag::Int(0)),
            ])])),
            "the entry keeps its place, as the game's writer does"
        );
    }

    #[test]
    fn a_cooler_tank_above_its_capacity_is_clamped() {
        let mut nbt = Compound::new();
        store_tank(
            &mut nbt,
            Some(("minecraft:water", COOLER_TANK + 5)),
            COOLER_TANK,
        );
        assert_eq!(
            tank_state(&nbt, COOLER_TANK),
            Some(("minecraft:water".to_owned(), COOLER_TANK))
        );
        store_tank(&mut nbt, Some(("minecraft:water", -5)), COOLER_TANK);
        assert_eq!(
            tank_state(&nbt, COOLER_TANK),
            None,
            "a negative amount is an empty tank"
        );
    }

    #[test]
    fn the_cooler_line_is_the_one_the_e2e_asserts() {
        let working = CoolerSaved {
            machine: Cooler {
                energy: 43_600,
                progress: 11,
                recipe: "oritech:cooler/ice".to_owned(),
            },
            assembly: Assembly {
                assembled: true,
                core: Some(COOLER_CORE_OFFSET),
                quality: 2.0,
            },
            tank: Some(("minecraft:water".to_owned(), 1_000)),
        };
        assert_eq!(
            cooler_line(
                [12, 100, 0],
                &CoolerView {
                    saved: &working,
                    core: Some(("oritech:machine_core_2", 2)),
                    output: None,
                },
                COOLER_CAPACITY,
                &AddonView::none(),
                424
            ),
            "Industrial Chiller at [12, 100, 0]: 43600/50000 FE, assembled yes core \
             oritech:machine_core_2 quality 2, progress 11 recipe oritech:cooler/ice, tank \
             minecraft:water 1000/4000 mB, out empty, addons none, t=424"
        );

        let idle = CoolerSaved::default();
        assert_eq!(
            cooler_line(
                [12, 100, 0],
                &CoolerView {
                    saved: &idle,
                    core: None,
                    output: Some(("minecraft:ice", 3)),
                },
                COOLER_CAPACITY,
                &AddonView::none(),
                7
            ),
            "Industrial Chiller at [12, 100, 0]: 0/50000 FE, assembled no core empty quality 0, \
             progress 0 recipe none, tank empty 0/4000 mB, out minecraft:ice x3, addons none, t=7"
        );
    }

    #[test]
    fn the_fluid_command_line_is_the_one_the_e2e_asserts() {
        assert_eq!(
            fluid_line(
                COOLER,
                [12, 100, 0],
                "minecraft:water",
                1_000,
                1_000,
                1_000,
                COOLER_TANK
            ),
            "oritech:cooler_block at [12, 100, 0]: tank minecraft:water 1000/4000 mB \
             (asked 1000, accepted 1000)"
        );
        assert_eq!(
            fluid_line(COOLER, [12, 100, 0], "empty", 0, 1_000, 0, COOLER_TANK),
            "oritech:cooler_block at [12, 100, 0]: tank empty 0/4000 mB (asked 1000, accepted 0)"
        );
    }

    #[test]
    fn the_lava_generator_declares_an_output_only_buffer_and_one_tank() {
        // OritechConfig.generators.lavaGeneratorData:
        // `BasicMachineConfig(b, "lavaGeneratorData", 100_000, 0, 64 * 8, 64)`.
        let buffer = battery(LAVA).expect("the lava generator has a buffer");
        assert_eq!(buffer.capacity, 100_000, "energyCapacity");
        assert_eq!(buffer.max_insert, 0, "it gives energy, it does not take it");
        assert_eq!(buffer.max_extract, 512, "maxEnergyExtraction = 64 * 8");
        assert_eq!(
            buffer.insert_faces, [false; 6],
            "no face takes a fill: nothing can charge a generator"
        );
        assert_eq!(buffer.extract_faces, [true; 6], "a cable may drain it");
        // `FluidMultiblockGeneratorBlockEntity.fluidStorage`.
        let tanks = fluids(LAVA).expect("the lava generator has a tank");
        assert_eq!(tanks.tanks, vec![LAVA_TANK], "one four-bucket tank");
        assert_eq!(tanks.inputs, vec![0], "a bucket or a pipe may fill it");
        assert_eq!(
            tanks.outputs,
            Vec::<usize>::new(),
            "the machine only takes fluid: nothing may drain it"
        );
        assert_eq!(
            tanks.faces,
            std::array::from_fn(|_| vec![0]),
            "every face reaches the tank"
        );
        assert!(
            container(LAVA).is_none(),
            "no item slots are declared: the bucket path is the engine's fluid interaction \
             (PLAN-slice4.md, deviation 1)"
        );
    }

    #[test]
    fn the_tank_capacity_is_per_block() {
        assert_eq!(tank_capacity(COOLER), Some(COOLER_TANK));
        assert_eq!(tank_capacity(LAVA), Some(LAVA_TANK));
        assert_eq!(
            tank_capacity(FURNACE),
            None,
            "the furnace's input is an item, not a fluid"
        );
        assert_eq!(tank_capacity("minecraft:stone"), None);
        assert!(fluids(LAVA).is_some() && fluids(COOLER).is_some());
    }

    #[test]
    fn the_lava_generator_s_state_reads_back_from_the_compound_it_wrote() {
        let mut nbt = Compound::new();
        assert_eq!(
            lava_state(&nbt, LAVA_CAPACITY),
            LavaSaved::default(),
            "a fresh lava generator is empty, unassembled and has an empty tank"
        );
        let saved = LavaSaved {
            machine: LavaGenerator {
                energy: 7_680,
                progress: 0,
                max_burn: 120,
                recipe: "oritech:lavagen/lava".to_owned(),
            },
            assembly: Assembly {
                assembled: true,
                core: Some(LAVA_CORE_OFFSET),
                quality: 2.0,
            },
            tank: None,
        };
        store_lava(&mut nbt, &saved);
        assert_eq!(lava_state(&nbt, LAVA_CAPACITY), saved);
        assert_eq!(
            nbt.get(&KEY_ENERGY.into()),
            Some(&Tag::Int(7_680)),
            "the buffer is the int `energy` the energy capability reads"
        );
        assert_eq!(
            nbt.get(&KEY_STORED_BURN.into()),
            Some(&Tag::Int(120)),
            "storedBurn is the same key the basic generator writes"
        );
        assert_eq!(
            nbt.get(&KEY_RECIPE.into()),
            Some(&Tag::String("oritech:lavagen/lava".into())),
            "the recipe is saved, so a reload continues the burn"
        );
        assert_eq!(
            nbt.get(&KEY_ASSEMBLED.into()),
            Some(&Tag::Byte(1)),
            "the assembled flag is the multiblock's own byte"
        );
        assert_eq!(
            nbt.get(&"fluids".into()),
            Some(&Tag::List(vec![Tag::compound([
                ("Fluid", Tag::String("minecraft:empty".into())),
                ("Amount", Tag::Int(0)),
            ])])),
            "an emptied tank keeps its entry, in the game's own `fluids` shape"
        );
        let filled = LavaSaved {
            tank: Some(("minecraft:lava".to_owned(), 100)),
            ..saved
        };
        store_lava(&mut nbt, &filled);
        assert_eq!(lava_state(&nbt, LAVA_CAPACITY).tank, filled.tank);
    }

    #[test]
    fn a_lava_tank_above_its_capacity_is_clamped() {
        let mut nbt = Compound::new();
        store_tank(&mut nbt, Some(("minecraft:lava", LAVA_TANK + 5)), LAVA_TANK);
        assert_eq!(
            tank_state(&nbt, LAVA_TANK),
            Some(("minecraft:lava".to_owned(), LAVA_TANK))
        );
        store_tank(&mut nbt, Some(("minecraft:lava", -5)), LAVA_TANK);
        assert_eq!(tank_state(&nbt, LAVA_TANK), None);
    }

    #[test]
    fn the_lava_generator_line_is_the_one_the_e2e_asserts() {
        let burning = LavaSaved {
            machine: LavaGenerator {
                energy: 3_840,
                progress: 60,
                max_burn: 120,
                recipe: "oritech:lavagen/lava".to_owned(),
            },
            assembly: Assembly {
                assembled: true,
                core: Some(LAVA_CORE_OFFSET),
                quality: 2.0,
            },
            tank: Some(("minecraft:lava".to_owned(), 100)),
        };
        assert_eq!(
            lava_line(
                [20, 100, 4],
                &LavaView {
                    saved: &burning,
                    core: Some(("oritech:machine_core_2", 2)),
                },
                LAVA_CAPACITY,
                &AddonView::none(),
                424
            ),
            "Lava Generator at [20, 100, 4]: 3840/100000 FE, assembled yes core \
             oritech:machine_core_2 quality 2, burning 60/120 ticks recipe \
             oritech:lavagen/lava, tank minecraft:lava 100/4000 mB, addons none, t=424"
        );

        let idle = LavaSaved::default();
        assert_eq!(
            lava_line(
                [20, 100, 4],
                &LavaView {
                    saved: &idle,
                    core: None,
                },
                LAVA_CAPACITY,
                &AddonView::none(),
                7
            ),
            "Lava Generator at [20, 100, 4]: 0/100000 FE, assembled no core empty quality 0, \
             burning 0/0 ticks recipe none, tank empty 0/4000 mB, addons none, t=7"
        );
    }

    #[test]
    fn the_fuel_command_line_is_the_lava_generators_own_capacity() {
        assert_eq!(
            fluid_line(
                LAVA,
                [20, 100, 4],
                "minecraft:lava",
                100,
                100,
                100,
                LAVA_TANK
            ),
            "oritech:lava_generator_block at [20, 100, 4]: tank minecraft:lava 100/4000 mB \
             (asked 100, accepted 100)"
        );
    }

    #[test]
    fn the_energy_push_plan_is_per_generator() {
        let (extract, targets) = push_plan(GENERATOR).expect("the basic generator pushes");
        assert_eq!(extract, 256, "basicGeneratorData.maxEnergyExtraction");
        assert_eq!(targets.len(), 6, "its six neighbours");
        for side in Direction::ALL {
            let [x, y, z] = side.normal();
            assert!(
                targets.contains(&([x, y, z], [-x, -y, -z])),
                "the basic generator pushes {side:?}"
            );
        }
        let (extract, targets) = push_plan(LAVA).expect("the lava generator pushes");
        assert_eq!(
            extract, 512,
            "lavaGeneratorData.maxEnergyExtraction = 64 * 8"
        );
        assert_eq!(
            targets,
            lava_output_targets(),
            "its ten targets are the pure rule's list"
        );
        assert!(
            push_plan(PULVERIZER).is_none(),
            "a machine that spends FE is not a source"
        );
        assert!(push_plan("minecraft:stone").is_none());
    }

    // -- slice 5: the addon system --------------------------------------------

    #[test]
    fn the_burst_counter_reads_back_from_the_compound_it_wrote() {
        let mut nbt = Compound::new();
        assert_eq!(burst_state(&nbt), Burst::default(), "a fresh machine");
        store_burst(&mut nbt, Burst { remaining: 227 });
        assert_eq!(burst_state(&nbt).remaining, 227);
        assert_eq!(
            nbt.get(&KEY_BURST.into()),
            Some(&Tag::Int(227)),
            "the counter is a saved int"
        );
        assert_eq!(
            burst_state(&nbt).clamped(0).remaining,
            0,
            "removing the burst addon clamps the stored counter"
        );
    }

    #[test]
    fn a_capacitor_keeps_a_buffer_above_the_machines_own_capacity() {
        let mut nbt = Compound::new();
        nbt.insert(KEY_ENERGY.into(), Tag::Int(1_000_000));
        assert_eq!(
            pulverizer_state(&nbt, PULVERIZER_CAPACITY).energy,
            PULVERIZER_CAPACITY,
            "without the addon the saved value is clamped as before"
        );
        let addons = Addons::gather(&["oritech:machine_capacitor_addon"], ADDON_MODE);
        assert_eq!(
            pulverizer_state(&nbt, addons.capacity(PULVERIZER_CAPACITY)).energy,
            1_000_000,
            "the capacitor's room is what the reader clamps against"
        );
    }

    #[test]
    fn the_readout_names_a_machines_addons_and_its_burst_counter() {
        let state = Pulverizer {
            energy: 100,
            progress: 0,
            recipe: String::new(),
        };
        let capacitor = AddonView {
            names: "speed+capacitor".to_owned(),
            burst: 0,
            burst_max: 0,
            redstone: None,
        };
        assert_eq!(
            pulverizer_line(
                [4, 100, 0],
                &state,
                [None, None, None],
                2_025_000,
                &capacitor,
                9
            ),
            "Pulverizer at [4, 100, 0]: 100/2025000 FE, progress 0 recipe none, input empty, \
             out0 empty, out1 empty, addons speed+capacitor, t=9"
        );
        let bursting = AddonView {
            names: "burst".to_owned(),
            burst: 227,
            burst_max: 240,
            redstone: None,
        };
        assert_eq!(
            pulverizer_line(
                [4, 100, 0],
                &state,
                [None, None, None],
                PULVERIZER_CAPACITY,
                &bursting,
                9
            ),
            "Pulverizer at [4, 100, 0]: 100/25000 FE, progress 0 recipe none, input empty, \
             out0 empty, out1 empty, addons burst burst 227/240, t=9"
        );
    }

    // -- slice 6: the Control Unit Addon --------------------------------------

    #[test]
    fn the_redstone_flag_reads_back_from_the_compound_it_wrote() {
        // `MachineBlockEntity.saveAdditional`'s `nbt.putBoolean("oritech.redstone", …)`,
        // which a vanilla compound writes as a byte.
        let mut nbt = Compound::new();
        assert!(!redstone_state(&nbt), "a fresh machine is not disabled");
        store_redstone(&mut nbt, true);
        assert!(redstone_state(&nbt));
        assert_eq!(
            nbt.get(&KEY_REDSTONE.into()),
            Some(&Tag::Byte(1)),
            "upstream's boolean is a byte"
        );
        store_redstone(&mut nbt, false);
        assert!(!redstone_state(&nbt));
        assert_eq!(nbt.get(&KEY_REDSTONE.into()), Some(&Tag::Byte(0)));
    }

    #[test]
    fn the_readout_names_the_redstone_addon_and_its_gate() {
        let state = Pulverizer {
            energy: 10_000,
            progress: 0,
            recipe: String::new(),
        };
        let slots = [Some(("minecraft:bone", 1)), None, None];
        let gated = AddonView {
            names: "redstone".to_owned(),
            burst: 0,
            burst_max: 0,
            redstone: Some(true),
        };
        assert_eq!(
            pulverizer_line([20, 100, 0], &state, slots, PULVERIZER_CAPACITY, &gated, 7),
            "Pulverizer at [20, 100, 0]: 10000/25000 FE, progress 0 recipe none, \
             input minecraft:bone x1, out0 empty, out1 empty, addons redstone redstone disabled, t=7"
        );
        let released = AddonView {
            names: "redstone".to_owned(),
            burst: 0,
            burst_max: 0,
            redstone: Some(false),
        };
        assert_eq!(
            pulverizer_line(
                [20, 100, 0],
                &state,
                [None, None, None],
                PULVERIZER_CAPACITY,
                &released,
                7
            ),
            "Pulverizer at [20, 100, 0]: 10000/25000 FE, progress 0 recipe none, \
             input empty, out0 empty, out1 empty, addons redstone redstone enabled, t=7"
        );
        // No control unit, no redstone field: the line every earlier phase reads.
        let plain = AddonView {
            names: "speed".to_owned(),
            burst: 0,
            burst_max: 0,
            redstone: None,
        };
        assert_eq!(
            pulverizer_line(
                [20, 100, 0],
                &state,
                [None, None, None],
                PULVERIZER_CAPACITY,
                &plain,
                7
            ),
            "Pulverizer at [20, 100, 0]: 10000/25000 FE, progress 0 recipe none, \
             input empty, out0 empty, out1 empty, addons speed, t=7"
        );
    }
}
