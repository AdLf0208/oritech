// Pure Oritech slice-1 arithmetic (standalone crate lib, injected as
// `crate::game::oritech`). No game access lives here: the Basic Generator's
// burn and FE production, the Pulverizer's recipe/energy/progress machine, the
// recipe JSON read from the pin's custom-recipe table, and the output-room rule.
//
// Upstream (`rearth/oritech` v1.2.13, read for behaviour only):
// - `UpgradableGeneratorBlockEntity.serverTick`: while `progress > 0` and the
//   buffer can fit one production tick, `progress--` and add
//   `energyPerTick * (1 / speedMultiplier)` FE. `BasicGeneratorEntity` loads one
//   input item's `FuelRegistry` burn time into `progress`.
// - `MachineBlockEntity.serverTick`: with a matching recipe and room for its
//   results, if the buffer holds `energyPerTick * efficiency / speed`, spend it,
//   `progress++`, and on `progress >= recipe.getTime() * speed` output the
//   results, shrink one input and reset.
// - `OritechConfig`: generator 50,000 FE / 0 insert / 256 extract / 32 per tick;
//   pulverizer 25,000 FE / 256 insert / 0 extract / 32 per tick.
// See `FINDINGS.md` for the file names and the exact lines of each value.

use serde_json::Value;

/// `OritechConfig.generators.basicGeneratorData.energyCapacity`.
pub const GENERATOR_CAPACITY: i32 = 50_000;
/// ...`.energyPerTick`: FE one burning tick produces.
pub const GENERATOR_ENERGY_PER_TICK: i32 = 32;
/// ...`.maxEnergyExtraction` (`32 * 8`): the most one extraction moves.
pub const GENERATOR_MAX_EXTRACT: i32 = 256;
/// `OritechConfig.processingMachines.pulverizerData.energyCapacity`.
pub const PULVERIZER_CAPACITY: i32 = 25_000;
/// ...`.maxEnergyInsertion` (`32 * 8`): the most one insert moves.
pub const PULVERIZER_MAX_INSERT: i32 = 256;
/// ...`.energyPerTick`: FE one working tick spends.
pub const MACHINE_ENERGY_PER_TICK: i32 = 32;
/// Energy clamped into a buffer.
#[must_use]
pub fn clamp_energy(energy: i32, capacity: i32) -> i32 {
    energy.clamp(0, capacity.max(0))
}

/// The Basic Generator's saved state: the buffer amount, the burning ticks left
/// (upstream's `progress`) and the ticks the last fuel gave (its
/// `currentMaxBurnTime`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Generator {
    /// Stored FE.
    pub energy: i32,
    /// Burning ticks left (`progress`).
    pub progress: i32,
    /// Ticks the last loaded fuel gave (`currentMaxBurnTime`).
    pub max_burn: i32,
}

impl Generator {
    /// An empty, unlit generator.
    #[must_use]
    #[allow(
        dead_code,
        reason = "pure API: the glue builds the state from NBT, the tests use the empty one"
    )]
    pub fn new() -> Self {
        Self::default()
    }

    /// The FE the buffer can still take.
    #[must_use]
    pub fn room(&self) -> i32 {
        GENERATOR_CAPACITY.saturating_sub(self.energy).max(0)
    }

    /// `UpgradableGeneratorBlockEntity.canFitEnergy`: room for one whole
    /// production tick.
    #[must_use]
    pub fn can_fit(&self) -> bool {
        self.room() >= GENERATOR_ENERGY_PER_TICK
    }

    /// Upstream's gate for `tryConsumeInput`: no burn left and room to make FE.
    #[must_use]
    pub fn wants_fuel(&self) -> bool {
        self.progress <= 0 && self.can_fit()
    }

    /// `BasicGeneratorEntity.tryConsumeInput`: take one fuel item's burn time.
    /// False when no fuel is wanted or the item has no burn time.
    pub fn load_fuel(&mut self, burn_time: u32) -> bool {
        if !self.wants_fuel() || !is_fuel(burn_time) {
            return false;
        }
        self.progress = i32::try_from(burn_time).unwrap_or(i32::MAX);
        self.max_burn = self.progress;
        true
    }

    /// One burning tick: the FE added (`0` when not burning or the buffer is
    /// full).
    pub fn tick(&mut self) -> i32 {
        if self.progress <= 0 || !self.can_fit() {
            return 0;
        }
        self.progress -= 1;
        self.energy += GENERATOR_ENERGY_PER_TICK;
        GENERATOR_ENERGY_PER_TICK
    }
}

/// What one tick of the Pulverizer did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// No recipe (or a blocked output): progress resets, no FE is spent.
    Idle,
    /// Out of FE: progress is kept, as upstream keeps the timer.
    Starved,
    /// One tick closer to the operation.
    Working,
    /// The operation finished this tick; progress is back to 0.
    Finished,
}

/// The work a tick may advance: the recipe's id and its `time` in ticks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Work<'a> {
    /// The recipe id, as the custom-recipe table names it.
    pub id: &'a str,
    /// `time` from the recipe file, in ticks.
    pub time: u32,
}

/// The Pulverizer's state: the buffer, the progress toward the current recipe
/// and the recipe it belongs to (upstream's `progress` and `currentRecipe`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Pulverizer {
    /// Stored FE.
    pub energy: i32,
    /// Ticks done toward the current recipe.
    pub progress: u32,
    /// The recipe being worked on; empty when none.
    pub recipe: String,
}

impl Pulverizer {
    /// An empty pulverizer.
    #[must_use]
    #[allow(
        dead_code,
        reason = "pure API: the glue builds the state from NBT, the tests use the empty one"
    )]
    pub fn new() -> Self {
        Self::default()
    }

    /// One server tick. `work` is `None` when the input has no matching recipe
    /// or its results do not fit the output slots.
    pub fn tick(&mut self, work: Option<Work<'_>>) -> Step {
        self.tick_with(work, MACHINE_ENERGY_PER_TICK)
    }

    /// One server tick at an addon's per-tick cost.
    pub fn tick_with(&mut self, work: Option<Work<'_>>, cost: i32) -> Step {
        advance(
            &mut self.energy,
            &mut self.progress,
            &mut self.recipe,
            work,
            cost,
        )
    }
}

/// One entry of a pulverizer recipe's `results` list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecipeResult {
    /// The item id the file names.
    pub item: String,
    /// How many it produces (at least 1).
    pub count: i32,
}

/// An `oritech:pulverizer` recipe, read from the pin's custom-recipe table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PulverizerRecipe {
    /// The recipe id.
    pub id: String,
    /// Its ingredient, as the pin's `native_recipe_ingredient` reads it.
    pub ingredient: Value,
    /// Its results, in file order.
    pub results: Vec<RecipeResult>,
    /// `time`, in ticks.
    pub time: u32,
}

/// Why a pulverizer recipe's JSON was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecipeError {
    /// The JSON did not parse or has no `type`.
    Unreadable,
    /// No `ingredients`, or not exactly one (the pulverizer has one input slot).
    WrongInputCount,
    /// No `results`, or more than the two output slots.
    WrongResultCount,
    /// A result without an item id or with a count outside 1..=99.
    BadResult,
    /// No positive `time`.
    BadTime,
}

impl std::fmt::Display for RecipeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Unreadable => "unreadable JSON",
            Self::WrongInputCount => "not exactly one ingredient",
            Self::WrongResultCount => "no result or more than two",
            Self::BadResult => "a result without an item id or with a bad count",
            Self::BadTime => "no positive time",
        })
    }
}

impl std::error::Error for RecipeError {}

impl PulverizerRecipe {
    /// Parse one recipe file. The JSON is the pin's custom-recipe JSON, kept as
    /// the data pack wrote it.
    pub fn parse(id: &str, json: &str) -> Result<Self, RecipeError> {
        let value: Value = serde_json::from_str(json).map_err(|_| RecipeError::Unreadable)?;
        if value.get("type").and_then(Value::as_str).is_none() {
            return Err(RecipeError::Unreadable);
        }
        let ingredients = value
            .get("ingredients")
            .and_then(Value::as_array)
            .ok_or(RecipeError::WrongInputCount)?;
        if ingredients.len() != 1 {
            return Err(RecipeError::WrongInputCount);
        }
        let ingredient = ingredient_json(&ingredients[0]);
        let results = value
            .get("results")
            .and_then(Value::as_array)
            .ok_or(RecipeError::WrongResultCount)?;
        if results.is_empty() || results.len() > 2 {
            return Err(RecipeError::WrongResultCount);
        }
        let mut parsed = Vec::with_capacity(results.len());
        for entry in results {
            let item = entry
                .get("id")
                .and_then(Value::as_str)
                .filter(|item| !item.is_empty())
                .ok_or(RecipeError::BadResult)?;
            let count = entry.get("count").and_then(Value::as_i64).unwrap_or(1);
            if !(1..=99).contains(&count) {
                return Err(RecipeError::BadResult);
            }
            parsed.push(RecipeResult {
                item: item.to_owned(),
                count: count as i32,
            });
        }
        let time = value
            .get("time")
            .and_then(Value::as_u64)
            .filter(|time| *time > 0)
            .ok_or(RecipeError::BadTime)?;
        Ok(Self {
            id: id.to_owned(),
            ingredient,
            results: parsed,
            time: u32::try_from(time).unwrap_or(u32::MAX),
        })
    }
}

/// A 1.21.1 ingredient (`{"item": …}` / `{"tag": …}` / a list) as the pin's
/// `native_recipe_ingredient` names it (`"id"` / `"#tag"` / a list).
#[must_use]
pub fn ingredient_json(value: &Value) -> Value {
    match value {
        Value::Object(object) => {
            if let Some(item) = object.get("item").and_then(Value::as_str) {
                Value::String(item.to_owned())
            } else if let Some(tag) = object.get("tag").and_then(Value::as_str) {
                Value::String(format!("#{tag}"))
            } else {
                value.clone()
            }
        }
        Value::Array(entries) => Value::Array(entries.iter().map(ingredient_json).collect()),
        _ => value.clone(),
    }
}

/// Whether `result` fits an output slot holding `slot` (item id and count). An
/// empty slot fits the result only when the whole result fits one stack: the
/// recipe parser accepts 1..=99, and `grow_output` cannot write past the item's
/// limit without silently losing the difference.
#[must_use]
pub fn output_fits(slot: Option<(&str, i32)>, result: &RecipeResult, max_stack: i32) -> bool {
    match slot {
        None => result.count >= 0 && result.count <= max_stack,
        Some((item, count)) => item == result.item && count + result.count <= max_stack,
    }
}

/// `MachineBlockEntity.canOutputRecipe`: every result must fit its own output
/// slot (slot 1 then slot 2).
#[must_use]
pub fn can_output(
    recipe: &PulverizerRecipe,
    outputs: [Option<(&str, i32)>; 2],
    max_stack: impl Fn(&str) -> i32,
) -> bool {
    recipe
        .results
        .iter()
        .zip(outputs)
        .all(|(result, slot)| output_fits(slot, result, max_stack(&result.item)))
}

/// The burn steps a fuel item gives: its burn time in ticks (`FuelRegistry`),
/// zero for a non-fuel.
#[must_use]
pub fn is_fuel(burn_time: u32) -> bool {
    burn_time > 0
}

/// The count an input slot keeps after one item is consumed: `None` when the
/// last item went, because an empty stack is `None` and the SDK refuses a stack
/// of zero. Writing the input back unchanged would leave the item in the slot
/// while the operation's output was already granted, so the machine would make
/// output from the same item on every operation.
#[must_use]
pub fn after_consuming(count: u32) -> Option<u32> {
    count.checked_sub(1).filter(|left| *left > 0)
}

/// `OritechConfig.processingMachines.furnaceData.energyCapacity` (slice 2).
pub const FURNACE_CAPACITY: i32 = 10_000;
/// ...`.maxEnergyInsertion` (`32 * 8`).
pub const FURNACE_MAX_INSERT: i32 = 256;
/// ...`.energyPerTick`: the config's own figure (the usage below is derived from it).
pub const FURNACE_ENERGY_PER_TICK: i32 = 32;
/// ...`.speedMultiplier`: relative to a vanilla furnace, so `0.5` is twice as fast.
pub const FURNACE_SPEED: f64 = 0.5;
/// `PoweredFurnaceBlockEntity.getCorePositions`: the one core, one block above.
pub const CORE_OFFSET: [i32; 3] = [0, 1, 0];
/// `BlockContent.MACHINE_CORE_1..MACHINE_CORE_7`, in tier order: the blocks a
/// multiblock machine assembles from, and their `MachineCoreBlock` quality.
pub const CORES: [&str; 7] = [
    "oritech:machine_core_1",
    "oritech:machine_core_2",
    "oritech:machine_core_3",
    "oritech:machine_core_4",
    "oritech:machine_core_5",
    "oritech:machine_core_6",
    "oritech:machine_core_7",
];

/// A machine core's quality: `MachineCoreBlock(…, n)`'s `n`, `None` for any
/// other block.
#[must_use]
pub fn core_quality(block: &str) -> Option<i32> {
    CORES
        .iter()
        .position(|core| *core == block)
        .and_then(|index| i32::try_from(index).ok())
        .map(|index| index + 1)
}

/// The ticks a smelt of `cooking_time` takes here:
/// `PoweredFurnaceBlockEntity.furnaceCraftingFinished`'s
/// `progress >= cookingTime * speedMultiplier`, never below one tick.
#[must_use]
pub fn furnace_ticks(cooking_time: u32) -> u32 {
    let ticks = (f64::from(cooking_time) * FURNACE_SPEED) as u32;
    ticks.max(1)
}

/// `PoweredFurnaceBlockEntity.calculateEnergyUsage`:
/// `energyPerTick * efficiency * (1 / speed) / 2`, with both multipliers at
/// their no-addon defaults (efficiency 1, furniture speed 0.5).
#[must_use]
pub fn furnace_energy_usage() -> i32 {
    // 32 * 1 * (1 / 0.5) / 2 = 32, but written as the jar writes it so a config
    // change moves one place, not two.
    let usage = f64::from(FURNACE_ENERGY_PER_TICK) * (1.0 / FURNACE_SPEED) / 2.0;
    usage as i32
}

/// The step both processing machines take (`MachineBlockEntity.serverTick` /
/// `PoweredFurnaceBlockEntity.serverTick`): no work resets the progress, a
/// starved machine keeps it, a working tick spends `cost` FE, and the
/// operation finishes at `work.time`. One body, so the two machines cannot
/// drift apart.
fn advance(
    energy: &mut i32,
    progress: &mut u32,
    recipe: &mut String,
    work: Option<Work<'_>>,
    cost: i32,
) -> Step {
    let Some(work) = work else {
        *progress = 0;
        recipe.clear();
        return Step::Idle;
    };
    if *recipe != work.id {
        *progress = 0;
        recipe.clear();
        recipe.push_str(work.id);
    }
    if *energy < cost {
        return Step::Starved;
    }
    *energy -= cost;
    *progress += 1;
    if *progress >= work.time.max(1) {
        *progress = 0;
        Step::Finished
    } else {
        Step::Working
    }
}

/// The Powered Furnace's state: the buffer, the progress toward the current
/// smelt and the recipe it belongs to — the same three values the Pulverizer
/// keeps, because the pin's machines advance one way.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Furnace {
    /// Stored FE.
    pub energy: i32,
    /// Ticks done toward the current smelt.
    pub progress: u32,
    /// The recipe being worked on; empty when none.
    pub recipe: String,
}

impl Furnace {
    /// An empty furnace.
    #[must_use]
    #[allow(
        dead_code,
        reason = "pure API: the glue builds the state from NBT, the tests use the empty one"
    )]
    pub fn new() -> Self {
        Self::default()
    }

    /// One server tick. `work` is `None` when the input has no smelting recipe,
    /// its output does not fit, or the machine is not assembled.
    pub fn tick(&mut self, work: Option<Work<'_>>) -> Step {
        self.tick_with(work, furnace_energy_usage())
    }

    /// One server tick at an addon's per-tick cost.
    pub fn tick_with(&mut self, work: Option<Work<'_>>, cost: i32) -> Step {
        advance(
            &mut self.energy,
            &mut self.progress,
            &mut self.recipe,
            work,
            cost,
        )
    }
}

/// Why a smelting recipe's JSON was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmeltError {
    /// The JSON did not parse or has no `type`.
    Unreadable,
    /// No `ingredient`.
    BadIngredient,
    /// No `result` with an item id, or a count outside 1..=99.
    BadResult,
    /// A `cookingtime` that is not a positive number.
    BadTime,
}

impl std::fmt::Display for SmeltError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Unreadable => "unreadable JSON",
            Self::BadIngredient => "no ingredient",
            Self::BadResult => "no result with an item id, or a bad count",
            Self::BadTime => "a cooking time that is not positive",
        })
    }
}

impl std::error::Error for SmeltError {}

/// One `minecraft:smelting` recipe, read from the game's own recipe table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmeltingRecipe {
    /// The recipe id, as the table names it.
    pub id: String,
    /// Its ingredient, as the pin's `recipe_ingredient` reads it.
    pub ingredient: Value,
    /// Its one result.
    pub result: RecipeResult,
    /// `cookingtime`, in ticks (vanilla's 200 when the file leaves it out).
    pub cooking_time: u32,
}

impl SmeltingRecipe {
    /// Parse one `minecraft:smelting` recipe file: `{type, cookingtime,
    /// ingredient, result}`, the 26.3 shape (a bare id, a list, `{item}` or
    /// `{tag}` for the ingredient; `{id, count}` for the result).
    pub fn parse(id: &str, json: &str) -> Result<Self, SmeltError> {
        let value: Value = serde_json::from_str(json).map_err(|_| SmeltError::Unreadable)?;
        if value.get("type").and_then(Value::as_str).is_none() {
            return Err(SmeltError::Unreadable);
        }
        let raw = value.get("ingredient").ok_or(SmeltError::BadIngredient)?;
        let ingredient = ingredient_json(raw);
        if matches!(&ingredient, Value::Array(entries) if entries.is_empty()) {
            return Err(SmeltError::BadIngredient);
        }
        let result = value.get("result").ok_or(SmeltError::BadResult)?;
        let item = ["id", "item"]
            .iter()
            .find_map(|key| result.get(*key).and_then(Value::as_str))
            .filter(|item| !item.is_empty())
            .ok_or(SmeltError::BadResult)?;
        let count = result.get("count").and_then(Value::as_i64).unwrap_or(1);
        if !(1..=99).contains(&count) {
            return Err(SmeltError::BadResult);
        }
        // `SmeltingRecipe.getCookingTime`'s default is 200 ticks; a file that
        // names a time must name a usable one.
        let cooking_time = match value.get("cookingtime") {
            None => 200,
            Some(time) => time
                .as_u64()
                .filter(|time| *time > 0)
                .ok_or(SmeltError::BadTime)?,
        };
        Ok(Self {
            id: id.to_owned(),
            ingredient,
            result: RecipeResult {
                item: item.to_owned(),
                count: i32::try_from(count).unwrap_or(1),
            },
            cooking_time: u32::try_from(cooking_time).unwrap_or(u32::MAX),
        })
    }
}

// ---------------------------------------------------------------------------
// Slice 3: the Industrial Chiller (`oritech:cooler_block`)
// ---------------------------------------------------------------------------

/// `OritechConfig.processingMachines.coolerData.energyCapacity`.
pub const COOLER_CAPACITY: i32 = 50_000;
/// ...`.maxEnergyInsertion` (`32 * 8`): the most one insert moves.
pub const COOLER_MAX_INSERT: i32 = 256;
/// ...`.energyPerTick`: the config's own figure (the usage below is derived
/// from it).
pub const COOLER_ENERGY_PER_TICK: i32 = 32;
/// `CoolerBlockEntity.fluidStorage` = `new SimpleFluidStorage(4 * bucketAmount())`:
/// the machine's one tank, in millibuckets.
pub const COOLER_TANK: i32 = 4_000;
/// `CoolerBlockEntity.getCorePositions`: the one core, one block to the north
/// (`Vec3i(0, 0, -1)`) — a different offset from the furnace's core, which is
/// why the assembly reads it per machine.
pub const COOLER_CORE_OFFSET: [i32; 3] = [0, 0, -1];
/// `CoolerBlockEntity.processCraftInstance`'s fit rule: the output slot's count
/// plus the result's must not pass 64. Upstream compares against a literal 64,
/// not the item's stack size.
pub const COOLER_OUTPUT_LIMIT: i32 = 64;

/// `CoolerBlockEntity.calculateEnergyUsage` with its no-addon multipliers
/// (`energyPerTick * efficiency(1) * (1 / speed(1))`). A cold biome halves both
/// multipliers, which leaves the usage where it is and halves the duration; the
/// pin has no biome read for a mod, so both are the warm figures here
/// (`notes.md`, *Deviations (slice 3)*).
#[must_use]
pub fn cooler_energy_usage() -> i32 {
    COOLER_ENERGY_PER_TICK
}

/// The ticks one craft of `time` takes here: `checkCraftingFinished`'s
/// `progress >= time * speed`, never below one tick.
#[must_use]
pub fn cooler_ticks(time: u32) -> u32 {
    time.max(1)
}

/// A recipe's fluid input: `FluidIngredient`'s `fluid` (an id, or `#tag` for a
/// tag) and its `amount`, a bucket when the file leaves the amount out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FluidInput {
    /// The fluid id, or `#tag` when the file names a tag.
    pub fluid: String,
    /// Millibuckets one craft takes.
    pub amount: i32,
}

impl FluidInput {
    /// Whether the file named a tag rather than one fluid.
    #[must_use]
    pub fn is_tag(&self) -> bool {
        self.fluid.starts_with('#')
    }
}

/// `CentrifugeBlockEntity.recipeInputMatchesTank`, the test the cooler reuses: a
/// recipe that needs no fluid always matches; otherwise the tank must hold that
/// fluid and at least the amount.
///
/// A tag input cannot be resolved here: the pin's mod API has no fluid-tag
/// lookup, so such a recipe never matches and is recorded as a gap rather than
/// silently treated as "any fluid".
#[must_use]
pub fn tank_matches(tank: Option<(&str, i32)>, input: &FluidInput) -> bool {
    if input.amount <= 0 {
        return true;
    }
    if input.is_tag() || input.fluid.is_empty() {
        return false;
    }
    let Some((fluid, amount)) = tank else {
        return false;
    };
    fluid == input.fluid && amount >= input.amount
}

/// `CoolerBlockEntity.processCraftInstance`: the whole result must fit the one
/// output slot — the same item, and `count + result.count <= 64`.
#[must_use]
pub fn cooler_output_fits(output: Option<(&str, i32)>, result: &RecipeResult) -> bool {
    let budget = COOLER_OUTPUT_LIMIT.max(0);
    if result.count <= 0 || result.count > budget {
        return false;
    }
    match output {
        None => true,
        Some((item, count)) => {
            item == result.item && count.max(0).saturating_add(result.count) <= budget
        }
    }
}

/// The tank's amount after one craft takes `needed`: `None` when the craft took
/// the last of it, because an empty tank is written as `minecraft:empty`.
#[must_use]
pub fn after_draining(amount: i32, needed: i32) -> Option<i32> {
    let left = amount.saturating_sub(needed);
    (left > 0).then_some(left)
}

/// Why a cooler recipe's JSON was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoolerError {
    /// The JSON did not parse or has no `type`.
    Unreadable,
    /// No `fluidInput`, or a fluid input with no fluid id.
    BadFluid,
    /// No result with an item id, a count outside 1..=99, or more than one
    /// result (the cooler has one output slot).
    BadResult,
    /// No positive `time`.
    BadTime,
}

impl std::fmt::Display for CoolerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Unreadable => "unreadable JSON",
            Self::BadFluid => "no fluid input",
            Self::BadResult => "no result with an item id, a bad count, or more than one",
            Self::BadTime => "no positive time",
        })
    }
}

impl std::error::Error for CoolerError {}

/// One `oritech:cooler` recipe: a fluid in, one item out, in `time` ticks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoolerRecipe {
    /// The recipe id.
    pub id: String,
    /// The fluid it takes.
    pub fluid: FluidInput,
    /// Its one result.
    pub result: RecipeResult,
    /// `time`, in ticks.
    pub time: u32,
}

impl CoolerRecipe {
    /// Parse one `oritech:cooler` file: `{type, fluidInput: {fluid, amount?},
    /// results: [{id, count}], time}`.
    pub fn parse(id: &str, json: &str) -> Result<Self, CoolerError> {
        let value: Value = serde_json::from_str(json).map_err(|_| CoolerError::Unreadable)?;
        if value.get("type").and_then(Value::as_str).is_none() {
            return Err(CoolerError::Unreadable);
        }
        // `FluidIngredient.CODEC`: `fluid` is an id or `#tag`, and `amount`
        // defaults to one bucket when the file leaves it out.
        let fluid = parse_fluid_input(value.get("fluidInput").unwrap_or(&Value::Null))
            .ok_or(CoolerError::BadFluid)?;
        // The cooler has one output slot: one result, or the recipe cannot run.
        let results = value
            .get("results")
            .and_then(Value::as_array)
            .ok_or(CoolerError::BadResult)?;
        if results.len() != 1 {
            return Err(CoolerError::BadResult);
        }
        let entry = &results[0];
        let item = entry
            .get("id")
            .and_then(Value::as_str)
            .filter(|item| !item.is_empty())
            .ok_or(CoolerError::BadResult)?;
        let count = entry.get("count").and_then(Value::as_i64).unwrap_or(1);
        if !(1..=99).contains(&count) {
            return Err(CoolerError::BadResult);
        }
        let time = value
            .get("time")
            .and_then(Value::as_u64)
            .filter(|time| *time > 0)
            .ok_or(CoolerError::BadTime)?;
        Ok(Self {
            id: id.to_owned(),
            fluid,
            result: RecipeResult {
                item: item.to_owned(),
                count: i32::try_from(count).unwrap_or(1),
            },
            time: u32::try_from(time).unwrap_or(u32::MAX),
        })
    }
}

/// `FluidIngredient.CODEC`'s `{fluid, amount?}`: a fluid id or `#tag`, and the
/// amount, a bucket when the file leaves it out. Shared by the cooler's and the
/// lava generator's files, which name the same object.
fn parse_fluid_input(input: &Value) -> Option<FluidInput> {
    let input = input.as_object()?;
    let fluid = input
        .get("fluid")
        .and_then(Value::as_str)
        .filter(|fluid| !fluid.is_empty())?;
    let amount = match input.get("amount") {
        None => 1_000,
        Some(amount) => amount.as_i64().filter(|amount| *amount > 0)?,
    };
    Some(FluidInput {
        fluid: fluid.to_owned(),
        amount: i32::try_from(amount).unwrap_or(i32::MAX),
    })
}

/// The Industrial Chiller's state: the buffer, the progress toward the current
/// craft and the recipe it belongs to — the same three values the Pulverizer and
/// the Furnace keep, because the pin's machines advance one way.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Cooler {
    /// Stored FE.
    pub energy: i32,
    /// Ticks done toward the current craft.
    pub progress: u32,
    /// The recipe being worked on; empty when none.
    pub recipe: String,
}

impl Cooler {
    /// An empty, unassembled chiller's state.
    #[must_use]
    #[allow(
        dead_code,
        reason = "pure API: the glue builds the state from NBT, the tests use the empty one"
    )]
    pub fn new() -> Self {
        Self::default()
    }

    /// One server tick. `work` is `None` when the tank does not hold a recipe's
    /// fluid, its output does not fit, or the machine is not assembled.
    pub fn tick(&mut self, work: Option<Work<'_>>) -> Step {
        self.tick_with(work, cooler_energy_usage())
    }

    /// One server tick at an addon's per-tick cost.
    pub fn tick_with(&mut self, work: Option<Work<'_>>, cost: i32) -> Step {
        advance(
            &mut self.energy,
            &mut self.progress,
            &mut self.recipe,
            work,
            cost,
        )
    }
}

// ---------------------------------------------------------------------------
// Slice 4: the Lava Generator (`oritech:lava_generator_block`)
// ---------------------------------------------------------------------------

/// `OritechConfig.generators.lavaGeneratorData.energyCapacity`.
pub const LAVA_CAPACITY: i32 = 100_000;
/// ...`.maxEnergyExtraction` (`64 * 8`): the most one extraction moves.
pub const LAVA_MAX_EXTRACT: i32 = 512;
/// ...`.energyPerTick`: FE one burning tick produces.
pub const LAVA_ENERGY_PER_TICK: i32 = 64;
/// `FluidMultiblockGeneratorBlockEntity.fluidStorage` =
/// `new SimpleFluidStorage(4 * bucketAmount())`: the machine's one tank.
pub const LAVA_TANK: i32 = 4_000;
/// `LavaGeneratorEntity.getCorePositions`: the one core, one block above the
/// controller — the furnace's offset, not the chiller's.
pub const LAVA_CORE_OFFSET: [i32; 3] = [0, 1, 0];

/// One energy target: the offset from the generator and the face of the target
/// whose handler is queried.
pub type EnergyTarget = ([i32; 3], [i32; 3]);

/// `LavaGeneratorEntity.getOutputTargets`, as (offset, the face the target's
/// handler is queried from). It is not the six neighbours: the four horizontal
/// positions are repeated one block up, the top target is two blocks up, and a
/// target directly above is absent (upstream pushes there through the pair at
/// `(0, 2, 0)` instead).
pub const LAVA_OUTPUT_TARGETS: [EnergyTarget; 10] = [
    // pos.above(2), DOWN
    ([0, 2, 0], [0, -1, 0]),
    // pos.below(), DOWN
    ([0, -1, 0], [0, -1, 0]),
    // pos.east(), WEST; pos.east().above(), WEST
    ([1, 0, 0], [-1, 0, 0]),
    ([1, 1, 0], [-1, 0, 0]),
    // pos.south(), NORTH; pos.south().above(), NORTH
    ([0, 0, 1], [0, 0, -1]),
    ([0, 1, 1], [0, 0, -1]),
    // pos.west(), EAST; pos.west().above(), EAST
    ([-1, 0, 0], [1, 0, 0]),
    ([-1, 1, 0], [1, 0, 0]),
    // pos.north(), SOUTH; pos.north().above(), SOUTH
    ([0, 0, -1], [0, 0, 1]),
    ([0, 1, -1], [0, 0, 1]),
];

/// The targets `LavaGeneratorEntity.outputEnergy` walks, for the glue.
#[must_use]
pub fn lava_output_targets() -> &'static [EnergyTarget] {
    &LAVA_OUTPUT_TARGETS
}

/// `UpgradableGeneratorBlockEntity.calculateEnergyUsage` for a generator:
/// `energyPerTick * (1 / speedMultiplier)`. With no addon the speed is 1, so a
/// burning tick makes the config's own 64 FE.
#[must_use]
pub fn lava_energy_usage() -> i32 {
    LAVA_ENERGY_PER_TICK
}

/// The ticks one burn of `time` takes here: `progress = recipeTime`, never below
/// one tick.
#[must_use]
pub fn lavagen_ticks(time: u32) -> u32 {
    time.max(1)
}

/// The Lava Generator's saved state: the buffer, the burning ticks left
/// (upstream's `progress`), the ticks the loaded recipe gave
/// (`currentMaxBurnTime`, saved as `storedBurn`) and the recipe being burned.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LavaGenerator {
    /// Stored FE.
    pub energy: i32,
    /// Burning ticks left (`progress`).
    pub progress: i32,
    /// Ticks the loaded recipe gave (`currentMaxBurnTime`).
    pub max_burn: i32,
    /// The recipe being burned; empty when none.
    pub recipe: String,
}

impl LavaGenerator {
    /// An empty, unlit lava generator.
    #[must_use]
    #[allow(
        dead_code,
        reason = "pure API: the glue builds the state from NBT, the tests use the empty one"
    )]
    pub fn new() -> Self {
        Self::default()
    }

    /// The FE the buffer can still take.
    #[must_use]
    pub fn room(&self) -> i32 {
        LAVA_CAPACITY.saturating_sub(self.energy).max(0)
    }

    /// `UpgradableGeneratorBlockEntity.canFitEnergy`: room for one whole
    /// production tick.
    #[must_use]
    pub fn can_fit(&self) -> bool {
        self.can_fit_amount(LAVA_ENERGY_PER_TICK)
    }

    /// `canFitEnergy` with an addon's own per-tick production.
    #[must_use]
    pub fn can_fit_amount(&self, amount: i32) -> bool {
        self.room() >= amount
    }

    /// Upstream's gate for `tryConsumeInput`: no burn left and room to make FE.
    /// Only then does the tank's recipe load (and its fluid leave the tank).
    #[must_use]
    pub fn wants_recipe(&self) -> bool {
        self.wants_recipe_for(LAVA_ENERGY_PER_TICK)
    }

    /// `wants_recipe` with an addon's own per-tick production.
    #[must_use]
    pub fn wants_recipe_for(&self, amount: i32) -> bool {
        self.progress <= 0 && self.can_fit_amount(amount)
    }

    /// `FluidMultiblockGeneratorBlockEntity.consumeFluidRecipeInput`: the recipe's
    /// time becomes the burn left. False when no burn is wanted.
    pub fn load_recipe(&mut self, ticks: u32, recipe: &str) -> bool {
        self.load_recipe_for(ticks, recipe, LAVA_ENERGY_PER_TICK)
    }

    /// `load_recipe` with an addon's own per-tick production.
    pub fn load_recipe_for(&mut self, ticks: u32, recipe: &str, amount: i32) -> bool {
        if !self.wants_recipe_for(amount) || ticks == 0 {
            return false;
        }
        self.progress = i32::try_from(ticks).unwrap_or(i32::MAX);
        self.max_burn = self.progress;
        self.recipe.clear();
        self.recipe.push_str(recipe);
        true
    }

    /// One burning tick: the FE added (`0` when not burning or the buffer is
    /// full).
    pub fn tick(&mut self) -> i32 {
        self.tick_with(LAVA_ENERGY_PER_TICK)
    }

    /// One burning tick at an addon's own per-tick production.
    pub fn tick_with(&mut self, amount: i32) -> i32 {
        if self.progress <= 0 || !self.can_fit_amount(amount) {
            return 0;
        }
        self.progress -= 1;
        self.energy += amount;
        amount
    }
}

/// Why a `oritech:lava_generator` file was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LavaGenError {
    /// The JSON did not parse or has no `type`.
    Unreadable,
    /// No `fluidInput`, or a fluid input with no fluid id.
    BadFluid,
    /// No positive `time`.
    BadTime,
}

impl std::fmt::Display for LavaGenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Unreadable => "unreadable JSON",
            Self::BadFluid => "no fluid input",
            Self::BadTime => "no positive time",
        })
    }
}

impl std::error::Error for LavaGenError {}

/// One `oritech:lava_generator` recipe: a fluid burnt for `time` ticks with no
/// item input and no item result (`FuelGeneratorRecipe`, the jar's
/// `lavagen/*.json`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LavaGenRecipe {
    /// The recipe id.
    pub id: String,
    /// The fluid it burns.
    pub fluid: FluidInput,
    /// `time`, in ticks.
    pub time: u32,
}

impl LavaGenRecipe {
    /// Parse one `oritech:lava_generator` file:
    /// `{type, fluidInput: {fluid, amount?}, ingredients: [], results: [], time}`.
    pub fn parse(id: &str, json: &str) -> Result<Self, LavaGenError> {
        let value: Value = serde_json::from_str(json).map_err(|_| LavaGenError::Unreadable)?;
        if value.get("type").and_then(Value::as_str).is_none() {
            return Err(LavaGenError::Unreadable);
        }
        let fluid = parse_fluid_input(value.get("fluidInput").unwrap_or(&Value::Null))
            .ok_or(LavaGenError::BadFluid)?;
        let time = value
            .get("time")
            .and_then(Value::as_u64)
            .filter(|time| *time > 0)
            .ok_or(LavaGenError::BadTime)?;
        Ok(Self {
            id: id.to_owned(),
            fluid,
            time: u32::try_from(time).unwrap_or(u32::MAX),
        })
    }
}

// ---------------------------------------------------------------------------
// The machine addon system (slice 5)
// ---------------------------------------------------------------------------

/// `BlockContent.MACHINE_SPEED_ADDON`: speed ×0.5, efficiency ×1.2 upstream,
/// turned into this port's effective multipliers by [`Addons::gather`].
pub const SPEED_ADDON: &str = "oritech:machine_speed_addon";
/// `BlockContent.MACHINE_EFFICIENCY_ADDON`: efficiency ×0.8.
pub const EFFICIENCY_ADDON: &str = "oritech:machine_efficiency_addon";
/// `BlockContent.MACHINE_PROCESSING_ADDON`: efficiency ×1.5 and one extra
/// chamber.
pub const PROCESSING_ADDON: &str = "oritech:machine_processing_addon";
/// `BlockContent.MACHINE_CAPACITOR_ADDON`: +2,000,000 FE and +2,000 insert.
pub const CAPACITOR_ADDON: &str = "oritech:machine_capacitor_addon";
/// `BlockContent.MACHINE_BURST_ADDON`: 240 burst ticks.
pub const BURST_ADDON: &str = "oritech:machine_burst_addon";
/// `BlockContent.MACHINE_ULTIMATE_ADDON`: speed ×0.25, efficiency ×1.1.
pub const ULTIMATE_ADDON: &str = "oritech:machine_ultimate_addon";

/// The six default addon blocks, in the order the readout names them.
pub const ADDON_BLOCKS: [&str; 6] = [
    SPEED_ADDON,
    EFFICIENCY_ADDON,
    PROCESSING_ADDON,
    CAPACITOR_ADDON,
    BURST_ADDON,
    ULTIMATE_ADDON,
];

/// `OritechConfig.addonConfig.burstAddonSpeedMultiplier`: a charged burst addon
/// divides the duration by this.
pub const BURST_SPEED_MULTIPLIER: f32 = 8.0;
/// ...`.burstAddonThrottleMultiplier`: an exhausted one multiplies it by this.
pub const BURST_THROTTLE_MULTIPLIER: f32 = 1.2;

/// `MachineAddonBlock.AddonSettings`: what one addon block contributes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AddonSettings {
    /// `speedMultiplier` — lower is faster.
    pub speed: f32,
    /// `efficiencyMultiplier` — lower is cheaper.
    pub efficiency: f32,
    /// `addedCapacity`.
    pub capacity_bonus: i32,
    /// `addedInsert`.
    pub insert_bonus: i32,
    /// `chamberCount`.
    pub chambers: u32,
    /// `burstTicks`.
    pub burst_ticks: i32,
}

/// The settings of one of the six default addon blocks, `None` for any other
/// block (the other addon kinds belong to machine families this port does not
/// carry).
#[must_use]
pub fn addon_settings(block: &str) -> Option<AddonSettings> {
    /// `AddonSettings.getDefaultSettings`' neutral entry.
    const PLAIN: AddonSettings = AddonSettings {
        speed: 1.0,
        efficiency: 1.0,
        capacity_bonus: 0,
        insert_bonus: 0,
        chambers: 0,
        burst_ticks: 0,
    };
    let settings = match block {
        // `withSpeedMultiplier(speedAddonSpeed)` / `withEfficiencyMultiplier(speedAddonEfficiency)`.
        SPEED_ADDON => AddonSettings {
            speed: 0.5,
            efficiency: 1.2,
            ..PLAIN
        },
        // `withEfficiencyMultiplier(efficiencyAddonEfficiency)`.
        EFFICIENCY_ADDON => AddonSettings {
            efficiency: 0.8,
            ..PLAIN
        },
        // `withEfficiencyMultiplier(chamberAddonEfficiency).withChambers(1)`.
        PROCESSING_ADDON => AddonSettings {
            efficiency: 1.5,
            chambers: 1,
            ..PLAIN
        },
        // `withAddedCapacity(2_000_000).withAddedInsert(2_000)`.
        CAPACITOR_ADDON => AddonSettings {
            capacity_bonus: 2_000_000,
            insert_bonus: 2_000,
            ..PLAIN
        },
        // `withBurstTicks(burstAddonTicks)`.
        BURST_ADDON => AddonSettings {
            burst_ticks: 240,
            ..PLAIN
        },
        // `withSpeedMultiplier(ultimateAddonSpeed).withEfficiencyMultiplier(ultimateAddonEfficiency)`.
        ULTIMATE_ADDON => AddonSettings {
            speed: 0.25,
            efficiency: 1.1,
            ..PLAIN
        },
        _ => return None,
    };
    Some(settings)
}

/// `OritechConfig.additiveAddons`' two branches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddonMode {
    /// The shipped default: each addon's *bonus* is added, then converted back
    /// to a multiplier.
    Additive,
    /// `additiveAddons = false`: the settings multiply each other directly.
    Multiplicative,
}

/// The jar's shipped default (`OritechConfig.additiveAddons`, `.define(…, true)`).
pub const ADDON_MODE: AddonMode = AddonMode::Additive;

/// `BaseAddonData`: the effective multipliers and bonuses one machine gathered
/// from its connected addons.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Addons {
    /// The effective speed multiplier (lower is faster).
    pub speed: f32,
    /// The effective efficiency multiplier (lower is cheaper).
    pub efficiency: f32,
    /// `energyBonusCapacity`.
    pub capacity_bonus: i32,
    /// `energyBonusTransfer`.
    pub insert_bonus: i32,
    /// `extraChambers`.
    pub chambers: u32,
    /// `maxBurstTicks`.
    pub burst_ticks: i32,
}

impl Default for Addons {
    fn default() -> Self {
        Self {
            speed: 1.0,
            efficiency: 1.0,
            capacity_bonus: 0,
            insert_bonus: 0,
            chambers: 0,
            burst_ticks: 0,
        }
    }
}

impl Addons {
    /// `MachineAddonController.gatherAddonStats`: the six default blocks'
    /// settings, combined the way the jar's config picks.
    #[must_use]
    pub fn gather(blocks: &[&str], mode: AddonMode) -> Self {
        let mut gathered = Self::default();
        let mut speed = 1.0_f32;
        let mut efficiency = 1.0_f32;
        for block in blocks {
            let Some(settings) = addon_settings(block) else {
                continue;
            };
            match mode {
                AddonMode::Additive => {
                    speed += 1.0 - settings.speed;
                    efficiency += 1.0 - settings.efficiency;
                }
                AddonMode::Multiplicative => {
                    speed *= settings.speed;
                    efficiency *= settings.efficiency;
                }
            }
            gathered.capacity_bonus = gathered
                .capacity_bonus
                .saturating_add(settings.capacity_bonus);
            gathered.insert_bonus = gathered.insert_bonus.saturating_add(settings.insert_bonus);
            gathered.chambers = gathered.chambers.saturating_add(settings.chambers);
            gathered.burst_ticks = gathered.burst_ticks.saturating_add(settings.burst_ticks);
        }
        if mode == AddonMode::Additive {
            // The conversion back to a multiplier. A non-positive accumulator
            // would divide by zero, which no shipped settings reach; the guard
            // keeps a bad config from producing NaN.
            let efficiency_change = efficiency - 1.0;
            speed = if speed > 0.0 { 1.0 / speed } else { f32::MAX };
            efficiency = if efficiency > 0.0 {
                1.0 / efficiency
            } else {
                f32::MAX
            };
            if efficiency_change < 0.0 {
                efficiency = 1.0 + efficiency_change.abs();
            }
        }
        Self {
            speed,
            efficiency,
            ..gathered
        }
    }

    /// `getDefaultCapacity() + energyBonusCapacity`.
    #[must_use]
    pub fn capacity(&self, base: i32) -> i32 {
        base.saturating_add(self.capacity_bonus)
    }

    /// `getDefaultInsertRate() + energyBonusTransfer`.
    #[must_use]
    pub fn insert(&self, base: i32) -> i32 {
        base.saturating_add(self.insert_bonus)
    }
}

/// The addon positions of one machine: `getAddonSlots` (world offsets, not
/// rotated by a facing this port does not model). The basic generator has none.
#[must_use]
pub fn addon_slots(block: &str) -> &'static [[i32; 3]] {
    match block {
        "oritech:pulverizer_block" => &[[1, 0, 0]],
        "oritech:powered_furnace_block" => &[[0, -1, 0]],
        "oritech:cooler_block" => &[[0, 0, -2]],
        "oritech:lava_generator_block" => &[[1, 0, 0], [1, 1, 0]],
        _ => &[],
    }
}

/// The readout's addon field: `none`, or the connected blocks' short names in
/// [`ADDON_BLOCKS`] order (`speed+capacitor`).
#[must_use]
pub fn addon_names(blocks: &[&str]) -> String {
    let short = [
        "speed",
        "efficiency",
        "processing",
        "capacitor",
        "burst",
        "ultimate",
    ];
    let names: Vec<&str> = ADDON_BLOCKS
        .iter()
        .zip(short)
        .filter(|(block, _)| blocks.contains(block))
        .map(|(_, name)| name)
        .collect();
    if names.is_empty() {
        "none".to_owned()
    } else {
        names.join("+")
    }
}

/// `MachineBlockEntity.calculateEnergyUsage`: `base * efficiency * (1/speed)`,
/// divided by the machine's own factor (the furnace's 2). The burst bonus
/// multiplies the speed *and* the efficiency, so it cancels here and only moves
/// the duration; [`speed_multiplier`] and [`efficiency_multiplier`] both apply
/// it, which is what makes the cancellation hold. Rounded up: the buffer is a
/// whole number of FE.
#[must_use]
pub fn energy_usage(base: i32, divisor: f64, speed: f32, efficiency: f32) -> i32 {
    if speed <= 0.0 {
        return i32::MAX;
    }
    // The jar subtracts the float from a long buffer, which for a whole-number
    // buffer takes `ceil(usage)` off it every tick: `x - 57.6` truncates to
    // `x - 58`. Written as the jar's formula so a config change moves one place.
    let usage = f64::from(base) * f64::from(efficiency) * (1.0 / f64::from(speed)) / divisor;
    if !usage.is_finite() || usage >= f64::from(i32::MAX) {
        return i32::MAX;
    }
    (usage.ceil().max(0.0)) as i32
}

/// `UpgradableMachineBlockEntity.getSpeedMultiplier`: the machine's own base
/// speed (the furnace's 0.5, 1 everywhere else) times the addons, times the
/// burst bonus.
#[must_use]
pub fn speed_multiplier(addons: &Addons, base_speed: f32, burst_bonus: f32) -> f32 {
    base_speed * addons.speed * burst_bonus
}

/// `UpgradableMachineBlockEntity.getEfficiencyMultiplier`: the addons times the
/// burst bonus (the burst bonus cancels against the speed in the usage, and is
/// what lets a burst shorten the duration without changing the FE a tick).
#[must_use]
pub fn efficiency_multiplier(addons: &Addons, burst_bonus: f32) -> f32 {
    addons.efficiency * burst_bonus
}

/// The ticks `checkCraftingFinished` asks for: `progress >= time * speed`, the
/// speed already carrying the base and the burst bonus. Never below one tick.
#[must_use]
pub fn processing_ticks(time: u32, speed: f32) -> u32 {
    if speed <= 0.0 {
        return 1;
    }
    let ticks = (f64::from(time) * f64::from(speed)).ceil();
    if !ticks.is_finite() || ticks >= f64::from(u32::MAX) {
        return u32::MAX;
    }
    (ticks.max(1.0)) as u32
}

/// `UpgradableGeneratorBlockEntity.tryConsumeInput`'s burn time:
/// `(int)(time * speed * (1/efficiency))`, never below one tick.
#[must_use]
pub fn generator_burn_ticks(time: u32, speed: f32, efficiency: f32) -> u32 {
    if speed <= 0.0 || efficiency <= 0.0 {
        return 1;
    }
    // `(int)` on a positive float truncates, which is the floor.
    let ticks = (f64::from(time) * f64::from(speed) / f64::from(efficiency)).floor();
    if !ticks.is_finite() || ticks >= f64::from(u32::MAX) {
        return u32::MAX;
    }
    (ticks.max(1.0)) as u32
}

/// `UpgradableGeneratorBlockEntity.calculateEnergyUsage` for a generator:
/// `base * efficiency / speed`, truncated as the jar's long buffer truncates the
/// float it adds.
#[must_use]
pub fn generator_output(base: i32, speed: f32, efficiency: f32) -> i32 {
    if speed <= 0.0 {
        return i32::MAX;
    }
    let output = (f64::from(base) * f64::from(efficiency) / f64::from(speed)).floor();
    if !output.is_finite() || output >= f64::from(i32::MAX) {
        return i32::MAX;
    }
    (output.max(0.0)) as i32
}

/// `UpgradableMachineBlockEntity`'s burst accumulator: +1 a server tick up to
/// the addons' `maxBurstTicks`, −2 on every working tick, floored at the
/// negative of that maximum.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Burst {
    /// `remainingBurstTicks`.
    pub remaining: i32,
}

impl Burst {
    /// The bonus `getBurstBonus` applies to the speed and efficiency.
    #[must_use]
    pub fn bonus(&self) -> f32 {
        if self.remaining > 0 {
            1.0 / BURST_SPEED_MULTIPLIER
        } else if self.remaining < 0 {
            BURST_THROTTLE_MULTIPLIER
        } else {
            1.0
        }
    }

    /// `addBurstTicks`: +1, capped at `max`.
    pub fn recharge(&mut self, max: i32) {
        self.remaining = self.remaining.saturating_add(1).min(max.max(0));
    }

    /// `consumeBurstTicks`: −2, floored at `-max`.
    pub fn spend(&mut self, max: i32) {
        self.remaining = self.remaining.saturating_sub(2).max(-max.max(0));
    }

    /// A stored counter brought inside a (possibly new) addon's range.
    #[must_use]
    pub fn clamped(self, max: i32) -> Self {
        let max = max.max(0);
        Self {
            remaining: self.remaining.clamp(-max, max),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn work(id: &'static str, time: u32) -> Option<Work<'static>> {
        Some(Work { id, time })
    }

    #[test]
    fn consuming_the_last_input_empties_the_slot_instead_of_keeping_it() {
        // The SDK refuses `set_count(0)`, so a one-item input that is written back would
        // stay in its slot while the operation's output was already granted: a powered
        // pulverizer would then create output from the same item for ever.
        assert_eq!(
            after_consuming(1),
            None,
            "the last item leaves the slot empty"
        );
        assert_eq!(after_consuming(2), Some(1));
        assert_eq!(after_consuming(64), Some(63));
    }

    #[test]
    fn a_burn_tick_makes_32_fe_and_spends_one_tick() {
        let mut generator = Generator {
            energy: 0,
            progress: 1600,
            max_burn: 1600,
        };
        assert_eq!(generator.tick(), GENERATOR_ENERGY_PER_TICK);
        assert_eq!(generator.energy, 32);
        assert_eq!(generator.progress, 1599);
        assert_eq!(generator.max_burn, 1600);
    }

    #[test]
    fn a_buffer_without_room_for_a_whole_tick_does_not_burn() {
        let mut generator = Generator {
            energy: GENERATOR_CAPACITY - 31,
            progress: 10,
            max_burn: 1600,
        };
        assert_eq!(generator.tick(), 0);
        assert_eq!(generator.energy, GENERATOR_CAPACITY - 31);
        assert_eq!(generator.progress, 10);
        assert!(!generator.can_fit());
        assert!(!generator.wants_fuel());
    }

    #[test]
    fn a_full_buffer_takes_no_fuel() {
        let mut generator = Generator {
            energy: GENERATOR_CAPACITY,
            progress: 0,
            max_burn: 0,
        };
        assert!(!generator.wants_fuel());
        assert!(!generator.load_fuel(1600));
        assert_eq!(generator.progress, 0);
    }

    #[test]
    fn one_coal_is_1600_ticks_and_stops_at_the_capacity() {
        // No drain: the burn pauses at 49,984 FE (1,562 ticks in), keeping the
        // rest of the fuel, as upstream's `canFitEnergy` does.
        let mut generator = Generator::new();
        assert!(generator.load_fuel(1600));
        assert_eq!(generator.progress, 1600);
        assert_eq!(generator.max_burn, 1600);
        let mut made = 0;
        for _ in 0..1600 {
            made += generator.tick();
        }
        assert_eq!(made, 49_984);
        assert_eq!(generator.energy, 49_984);
        assert_eq!(generator.progress, 1600 - 1_562);

        // With a drain after every tick the whole coal is spent: 1,600 x 32.
        let mut drained = Generator::new();
        assert!(drained.load_fuel(1600));
        let mut total = 0;
        for _ in 0..1600 {
            total += drained.tick();
            drained.energy = 0;
        }
        assert_eq!(total, 51_200);
        assert_eq!(drained.progress, 0);
    }

    #[test]
    fn a_fuel_without_burn_time_is_not_taken() {
        let mut generator = Generator::new();
        assert!(!generator.load_fuel(0));
        assert_eq!(generator.progress, 0);
        assert_eq!(generator.max_burn, 0);
        assert!(!is_fuel(0));
        assert!(is_fuel(1));
    }

    #[test]
    fn the_room_is_never_negative() {
        assert_eq!(Generator::new().room(), GENERATOR_CAPACITY);
        assert_eq!(
            Generator {
                energy: GENERATOR_CAPACITY + 1,
                ..Generator::new()
            }
            .room(),
            0
        );
        assert_eq!(clamp_energy(-5, GENERATOR_CAPACITY), 0);
        assert_eq!(
            clamp_energy(GENERATOR_CAPACITY + 5, GENERATOR_CAPACITY),
            GENERATOR_CAPACITY
        );
    }

    #[test]
    fn a_working_tick_spends_32_fe_and_counts_up() {
        let mut pulverizer = Pulverizer {
            energy: 10_000,
            ..Pulverizer::new()
        };
        assert_eq!(
            pulverizer.tick(work("oritech:pulverizer/bone", 100)),
            Step::Working
        );
        assert_eq!(pulverizer.energy, 10_000 - MACHINE_ENERGY_PER_TICK);
        assert_eq!(pulverizer.progress, 1);
        assert_eq!(pulverizer.recipe, "oritech:pulverizer/bone");
    }

    #[test]
    fn the_operation_finishes_at_the_recipes_time() {
        let mut pulverizer = Pulverizer {
            energy: 10_000,
            ..Pulverizer::new()
        };
        for tick in 1..100 {
            assert_eq!(
                pulverizer.tick(work("oritech:pulverizer/bone", 100)),
                Step::Working,
                "tick {tick}"
            );
        }
        assert_eq!(
            pulverizer.tick(work("oritech:pulverizer/bone", 100)),
            Step::Finished
        );
        // 100 ticks x 32 FE = 3,200 FE, exactly.
        assert_eq!(pulverizer.energy, 10_000 - 3_200);
        assert_eq!(pulverizer.progress, 0);
    }

    #[test]
    fn a_starved_machine_keeps_its_progress() {
        let mut pulverizer = Pulverizer {
            energy: MACHINE_ENERGY_PER_TICK - 1,
            progress: 40,
            recipe: "oritech:pulverizer/bone".to_owned(),
        };
        assert_eq!(
            pulverizer.tick(work("oritech:pulverizer/bone", 100)),
            Step::Starved
        );
        assert_eq!(pulverizer.energy, MACHINE_ENERGY_PER_TICK - 1);
        assert_eq!(pulverizer.progress, 40);
    }

    #[test]
    fn no_work_resets_the_progress_without_spending_fe() {
        let mut pulverizer = Pulverizer {
            energy: 5_000,
            progress: 40,
            recipe: "oritech:pulverizer/bone".to_owned(),
        };
        assert_eq!(pulverizer.tick(None), Step::Idle);
        assert_eq!(pulverizer.progress, 0);
        assert_eq!(pulverizer.recipe, "");
        assert_eq!(pulverizer.energy, 5_000);
    }

    #[test]
    fn another_recipe_restarts_the_operation() {
        let mut pulverizer = Pulverizer {
            energy: 5_000,
            progress: 40,
            recipe: "oritech:pulverizer/bone".to_owned(),
        };
        assert_eq!(
            pulverizer.tick(work("oritech:pulverizer/blaze", 100)),
            Step::Working
        );
        assert_eq!(pulverizer.progress, 1);
        assert_eq!(pulverizer.recipe, "oritech:pulverizer/blaze");
    }

    #[test]
    fn the_parser_reads_the_jar_shape() {
        let recipe = PulverizerRecipe::parse(
            "oritech:pulverizer/bone",
            r#"{
                "type": "oritech:pulverizer",
                "ingredients": [ { "item": "minecraft:bone" } ],
                "results": [ { "count": 6, "id": "minecraft:bone_meal" } ],
                "time": 100
            }"#,
        )
        .unwrap();
        assert_eq!(recipe.id, "oritech:pulverizer/bone");
        assert_eq!(recipe.time, 100);
        assert_eq!(
            recipe.ingredient,
            Value::String("minecraft:bone".to_owned())
        );
        assert_eq!(
            recipe.results,
            vec![RecipeResult {
                item: "minecraft:bone_meal".to_owned(),
                count: 6
            }]
        );
    }

    #[test]
    fn the_parser_reads_a_tag_ingredient() {
        let recipe = PulverizerRecipe::parse(
            "oritech:pulverizer/coalore",
            r#"{
                "type": "oritech:pulverizer",
                "ingredients": [ { "tag": "minecraft:coal_ores" } ],
                "results": [ { "count": 2, "id": "minecraft:coal" } ],
                "time": 100
            }"#,
        )
        .unwrap();
        assert_eq!(
            recipe.ingredient,
            Value::String("#minecraft:coal_ores".to_owned())
        );
    }

    #[test]
    fn the_parser_refuses_what_the_machine_cannot_run() {
        let base = r#""type":"oritech:pulverizer","time":100"#;
        assert_eq!(
            PulverizerRecipe::parse("r", r#"{"type":"oritech:pulverizer"}"#),
            Err(RecipeError::WrongInputCount)
        );
        assert_eq!(
            PulverizerRecipe::parse(
                "r",
                &format!(
                    r#"{{{base},"ingredients":[],"results":[{{"id":"minecraft:coal","count":1}}]}}"#
                )
            ),
            Err(RecipeError::WrongInputCount)
        );
        assert_eq!(
            PulverizerRecipe::parse(
                "r",
                &format!(r#"{{{base},"ingredients":[{{"item":"minecraft:bone"}}],"results":[]}}"#)
            ),
            Err(RecipeError::WrongResultCount)
        );
        assert_eq!(
            PulverizerRecipe::parse(
                "r",
                r#"{"type":"oritech:pulverizer","ingredients":[{"item":"minecraft:bone"}],"results":[{"id":"minecraft:bone_meal","count":1}]}"#
            ),
            Err(RecipeError::BadTime)
        );
        assert_eq!(
            PulverizerRecipe::parse(
                "r",
                &format!(
                    r#"{{{base},"ingredients":[{{"item":"minecraft:bone"}}],"results":[{{"count":0}}]}}"#
                )
            ),
            Err(RecipeError::BadResult)
        );
        assert_eq!(
            PulverizerRecipe::parse("r", "not json"),
            Err(RecipeError::Unreadable)
        );
    }

    #[test]
    fn an_output_must_match_its_slot_and_fit() {
        let result = RecipeResult {
            item: "minecraft:coal".to_owned(),
            count: 2,
        };
        assert!(output_fits(None, &result, 64));
        assert!(output_fits(Some(("minecraft:coal", 62)), &result, 64));
        assert!(!output_fits(Some(("minecraft:coal", 63)), &result, 64));
        assert!(!output_fits(Some(("minecraft:bone", 1)), &result, 64));
    }

    #[test]
    fn an_empty_output_slot_rejects_a_result_larger_than_one_stack() {
        let too_many = RecipeResult {
            item: "minecraft:bone_meal".to_owned(),
            count: 99,
        };
        assert!(
            !output_fits(None, &too_many, 64),
            "99 items do not fit an empty 64-stack slot"
        );
        let exactly = RecipeResult {
            item: "minecraft:bone_meal".to_owned(),
            count: 64,
        };
        assert!(output_fits(None, &exactly, 64));
    }

    #[test]
    fn can_output_checks_every_result_in_its_own_slot() {
        let recipe = PulverizerRecipe::parse(
            "oritech:pulverizer/bone",
            r#"{
                "type": "oritech:pulverizer",
                "ingredients": [ { "item": "minecraft:bone" } ],
                "results": [ { "count": 6, "id": "minecraft:bone_meal" } ],
                "time": 100
            }"#,
        )
        .unwrap();
        assert!(can_output(&recipe, [None, None], |_| 64));
        assert!(can_output(
            &recipe,
            [Some(("minecraft:bone_meal", 58)), None],
            |_| 64
        ));
        assert!(!can_output(
            &recipe,
            [Some(("minecraft:bone_meal", 59)), None],
            |_| 64
        ));
        assert!(!can_output(
            &recipe,
            [Some(("minecraft:coal", 1)), None],
            |_| 64
        ));
    }

    #[test]
    fn an_ingredient_mapping_covers_item_tag_and_list() {
        assert_eq!(
            ingredient_json(&serde_json::json!({"item": "minecraft:bone"})),
            Value::String("minecraft:bone".to_owned())
        );
        assert_eq!(
            ingredient_json(&serde_json::json!({"tag": "minecraft:coals"})),
            Value::String("#minecraft:coals".to_owned())
        );
        assert_eq!(
            ingredient_json(&serde_json::json!([
                "minecraft:bone",
                "minecraft:blaze_rod"
            ])),
            serde_json::json!(["minecraft:bone", "minecraft:blaze_rod"])
        );
        assert_eq!(
            ingredient_json(&serde_json::json!("minecraft:bone")),
            Value::String("minecraft:bone".to_owned())
        );
    }

    // -- slice 2: the Powered Furnace and the machine cores -------------------

    #[test]
    fn the_furnace_smelts_twice_as_fast_as_a_vanilla_furnace() {
        // `furnaceCraftingFinished`: `progress >= cookingTime * speedMultiplier`
        // with the config's 0.5, so vanilla's 200-tick smelt takes 100 here.
        assert_eq!(
            furnace_ticks(200),
            100,
            "an iron ore smelts in 100 ticks, half a vanilla furnace's 200"
        );
        assert_eq!(furnace_ticks(100), 50);
        assert_eq!(furnace_ticks(1), 1, "a one-tick recipe still needs a tick");
        assert_eq!(furnace_ticks(0), 1, "a smelt is never zero ticks");
    }

    #[test]
    fn the_furnace_spends_32_fe_a_tick() {
        // `calculateEnergyUsage`: `energyPerTick * efficiency(1) * (1 / 0.5) / 2`.
        let formula = (f64::from(FURNACE_ENERGY_PER_TICK) * (1.0 / FURNACE_SPEED) / 2.0) as i32;
        assert_eq!(furnace_energy_usage(), 32, "the jar's 32 FE a working tick");
        assert_eq!(
            furnace_energy_usage(),
            formula,
            "the usage is the config's formula, not a second independent number"
        );
    }

    #[test]
    fn the_furnace_finishes_in_100_ticks_for_3200_fe() {
        let mut furnace = Furnace {
            energy: FURNACE_CAPACITY,
            ..Furnace::new()
        };
        let dur = furnace_ticks(200);
        for tick in 1..dur {
            assert_eq!(
                furnace.tick(work("minecraft:iron_ingot_from_smelting_iron_ore", dur)),
                Step::Working,
                "tick {tick}"
            );
        }
        assert_eq!(
            furnace.tick(work("minecraft:iron_ingot_from_smelting_iron_ore", dur)),
            Step::Finished
        );
        assert_eq!(
            furnace.energy,
            FURNACE_CAPACITY - 32 * 100,
            "100 ticks x 32 FE = 3,200 FE"
        );
        assert_eq!(furnace.progress, 0);
    }

    #[test]
    fn the_furnace_keeps_progress_when_starved_and_resets_without_work() {
        let mut starved = Furnace {
            energy: furnace_energy_usage() - 1,
            progress: 40,
            recipe: "minecraft:iron_ingot_from_smelting_iron_ore".to_owned(),
        };
        assert_eq!(
            starved.tick(work("minecraft:iron_ingot_from_smelting_iron_ore", 100)),
            Step::Starved
        );
        assert_eq!(starved.progress, 40, "a starved smelt keeps its progress");
        assert_eq!(starved.energy, furnace_energy_usage() - 1);

        // Upstream's `else` branch: no recipe (or a blocked output) resets it.
        let mut idle = Furnace {
            energy: 5_000,
            progress: 40,
            recipe: "minecraft:iron_ingot_from_smelting_iron_ore".to_owned(),
        };
        assert_eq!(idle.tick(None), Step::Idle);
        assert_eq!(idle.progress, 0);
        assert_eq!(idle.recipe, "");
        assert_eq!(idle.energy, 5_000, "idling spends nothing");
    }

    #[test]
    fn the_smelting_parser_reads_the_pins_shape() {
        // The 26.3 file as the prepared resources hold it.
        let iron = SmeltingRecipe::parse(
            "minecraft:iron_ingot_from_smelting_iron_ore",
            r#"{
                "type": "minecraft:smelting",
                "cookingtime": 200,
                "experience": 0.7,
                "group": "iron_ingot",
                "ingredient": "minecraft:iron_ore",
                "result": { "id": "minecraft:iron_ingot" }
            }"#,
        )
        .unwrap();
        assert_eq!(iron.cooking_time, 200);
        assert_eq!(
            iron.ingredient,
            Value::String("minecraft:iron_ore".to_owned())
        );
        assert_eq!(
            iron.result,
            RecipeResult {
                item: "minecraft:iron_ingot".to_owned(),
                count: 1
            },
            "a result without a count is one item"
        );

        // A list ingredient and a counted result.
        let nuggets = SmeltingRecipe::parse(
            "minecraft:copper_nugget_from_smelting",
            r#"{
                "type": "minecraft:smelting",
                "cookingtime": 200,
                "ingredient": [ "minecraft:copper_pickaxe", "minecraft:copper_shovel" ],
                "result": { "id": "minecraft:copper_nugget", "count": 3 }
            }"#,
        )
        .unwrap();
        assert_eq!(
            nuggets.ingredient,
            serde_json::json!(["minecraft:copper_pickaxe", "minecraft:copper_shovel"])
        );
        assert_eq!(nuggets.result.count, 3);

        // `{item}` and `{tag}` read as the pin's `"id"` / `"#tag"`.
        let tagged = SmeltingRecipe::parse(
            "r",
            r#"{"type":"minecraft:smelting","cookingtime":200,
                "ingredient":{"tag":"minecraft:coals"},
                "result":{"id":"minecraft:torch","count":1}}"#,
        )
        .unwrap();
        assert_eq!(
            tagged.ingredient,
            Value::String("#minecraft:coals".to_owned())
        );
        let item = SmeltingRecipe::parse(
            "r",
            r#"{"type":"minecraft:smelting","ingredient":{"item":"minecraft:sand"},
                "result":{"id":"minecraft:glass"}}"#,
        )
        .unwrap();
        assert_eq!(item.ingredient, Value::String("minecraft:sand".to_owned()));
        assert_eq!(
            item.cooking_time, 200,
            "vanilla's SmeltingRecipe default is 200 ticks"
        );
    }

    #[test]
    fn the_smelting_parser_refuses_what_the_furnace_cannot_run() {
        assert_eq!(
            SmeltingRecipe::parse("r", "not json"),
            Err(SmeltError::Unreadable)
        );
        assert_eq!(
            SmeltingRecipe::parse(
                "r",
                r#"{"cookingtime":200,"result":{"id":"minecraft:glass"}}"#
            ),
            Err(SmeltError::Unreadable),
            "no type"
        );
        assert_eq!(
            SmeltingRecipe::parse(
                "r",
                r#"{"type":"minecraft:smelting","cookingtime":200,"result":{"id":"minecraft:glass"}}"#
            ),
            Err(SmeltError::BadIngredient)
        );
        assert_eq!(
            SmeltingRecipe::parse(
                "r",
                r#"{"type":"minecraft:smelting","cookingtime":200,"ingredient":"minecraft:sand"}"#
            ),
            Err(SmeltError::BadResult)
        );
        assert_eq!(
            SmeltingRecipe::parse(
                "r",
                r#"{"type":"minecraft:smelting","cookingtime":200,"ingredient":"minecraft:sand",
                    "result":{"id":"minecraft:glass","count":0}}"#
            ),
            Err(SmeltError::BadResult),
            "a count outside 1..=99"
        );
        assert_eq!(
            SmeltingRecipe::parse(
                "r",
                r#"{"type":"minecraft:smelting","cookingtime":0,"ingredient":"minecraft:sand",
                    "result":{"id":"minecraft:glass"}}"#
            ),
            Err(SmeltError::BadTime)
        );
    }

    #[test]
    fn only_the_seven_cores_are_cores_and_their_quality_is_their_tier() {
        assert_eq!(CORES.len(), 7, "the jar's MACHINE_CORE_1..7");
        for (index, core) in CORES.iter().enumerate() {
            assert_eq!(
                core_quality(core),
                Some(i32::try_from(index + 1).expect("seven fits")),
                "{core}"
            );
        }
        assert_eq!(CORE_OFFSET, [0, 1, 0], "the core sits one block above");
        assert_eq!(
            core_quality("oritech:powered_furnace_block"),
            None,
            "the controller is not a core"
        );
        assert_eq!(
            core_quality("oritech:machine_core_8"),
            None,
            "there is no eighth tier"
        );
        assert_eq!(core_quality("minecraft:iron_block"), None);
    }

    // -- slice 3: the Industrial Chiller --------------------------------------

    #[test]
    fn the_coolers_numbers_are_the_jars() {
        assert_eq!(COOLER_CAPACITY, 50_000, "coolerData.energyCapacity");
        assert_eq!(COOLER_MAX_INSERT, 256, "32 * 8");
        assert_eq!(COOLER_ENERGY_PER_TICK, 32, "coolerData.energyPerTick");
        assert_eq!(COOLER_TANK, 4_000, "SimpleFluidStorage(4 * bucketAmount())");
        assert_eq!(cooler_energy_usage(), 32, "32 * 1 * (1 / 1)");
        assert_eq!(cooler_ticks(200), 200, "a 200-tick file at speed 1");
        assert_eq!(cooler_ticks(0), 1, "never below one tick");
        assert_eq!(
            COOLER_CORE_OFFSET,
            [0, 0, -1],
            "getCorePositions: the core sits to the north"
        );
        assert_ne!(
            COOLER_CORE_OFFSET, CORE_OFFSET,
            "the two multiblocks do not share an offset"
        );
    }

    #[test]
    fn the_cooler_spends_32_fe_a_tick() {
        let mut cooler = Cooler {
            energy: 1_000,
            progress: 0,
            recipe: String::new(),
        };
        assert_eq!(cooler.tick(work("oritech:cooler/ice", 200)), Step::Working);
        assert_eq!(cooler.energy, 968, "one working tick is 32 FE");
        assert_eq!(cooler.progress, 1);
        assert_eq!(cooler.recipe, "oritech:cooler/ice");
    }

    #[test]
    fn the_cooler_finishes_in_200_ticks_for_6400_fe() {
        let mut cooler = Cooler {
            energy: COOLER_CAPACITY,
            progress: 0,
            recipe: String::new(),
        };
        let mut ticks = 0;
        loop {
            ticks += 1;
            if cooler.tick(work("oritech:cooler/ice", 200)) == Step::Finished {
                break;
            }
            assert!(ticks < 500, "the chiller never finished");
        }
        assert_eq!(ticks, 200, "the jar's `time: 200` at speed 1");
        assert_eq!(COOLER_CAPACITY - cooler.energy, 6_400, "200 ticks x 32 FE");
        assert_eq!(cooler.progress, 0, "the finished craft resets the progress");
    }

    #[test]
    fn a_starved_cooler_keeps_its_progress_and_no_work_resets_it() {
        let mut cooler = Cooler {
            energy: 10,
            progress: 40,
            recipe: "oritech:cooler/ice".to_owned(),
        };
        assert_eq!(cooler.tick(work("oritech:cooler/ice", 200)), Step::Starved);
        assert_eq!(cooler.progress, 40, "a starved craft keeps its progress");
        assert_eq!(cooler.energy, 10, "and spends nothing");

        let mut cooler = Cooler {
            energy: 100,
            progress: 40,
            recipe: "oritech:cooler/ice".to_owned(),
        };
        assert_eq!(cooler.tick(None), Step::Idle);
        assert_eq!(cooler.progress, 0, "no work resets the progress");
        assert!(cooler.recipe.is_empty(), "and forgets the recipe");
        assert_eq!(cooler.energy, 100, "without spending FE");
    }

    #[test]
    fn the_tank_holds_what_the_recipe_needs_and_nothing_else() {
        let water = FluidInput {
            fluid: "minecraft:water".to_owned(),
            amount: 1_000,
        };
        assert!(tank_matches(Some(("minecraft:water", 1_000)), &water));
        assert!(
            tank_matches(Some(("minecraft:water", 4_000)), &water),
            "a full tank has room for the craft"
        );
        assert!(
            !tank_matches(Some(("minecraft:water", 999)), &water),
            "a bucket short is not enough"
        );
        assert!(
            !tank_matches(Some(("minecraft:lava", 4_000)), &water),
            "lava is not water"
        );
        assert!(!tank_matches(None, &water), "an empty tank matches nothing");
        let steam = FluidInput {
            fluid: "#c:steam".to_owned(),
            amount: 32,
        };
        assert!(steam.is_tag());
        assert!(
            !tank_matches(Some(("#c:steam", 4_000)), &steam),
            "a fluid tag cannot be resolved at this pin, so it must not match"
        );
        assert!(
            tank_matches(
                None,
                &FluidInput {
                    fluid: String::new(),
                    amount: 0,
                }
            ),
            "a recipe that needs no fluid matches an empty tank"
        );
    }

    #[test]
    fn the_cooler_parser_reads_the_jars_ice_recipe() {
        let json = r#"{
          "type": "oritech:cooler",
          "fluidInput": {"fluid": "minecraft:water"},
          "ingredients": [],
          "results": [{"count": 3, "id": "minecraft:ice"}],
          "time": 200
        }"#;
        let recipe =
            CoolerRecipe::parse("oritech:cooler/ice", json).expect("the jar's ice recipe parses");
        assert_eq!(recipe.id, "oritech:cooler/ice");
        assert_eq!(recipe.fluid.fluid, "minecraft:water");
        assert_eq!(
            recipe.fluid.amount, 1_000,
            "FluidIngredient's amount defaults to one bucket"
        );
        assert_eq!(recipe.result.item, "minecraft:ice");
        assert_eq!(recipe.result.count, 3);
        assert_eq!(recipe.time, 200);
    }

    #[test]
    fn the_cooler_parser_reads_an_amount_and_a_tag() {
        let lava = r#"{
          "type": "oritech:cooler",
          "fluidInput": {"fluid": "minecraft:lava", "amount": 500},
          "ingredients": [],
          "results": [{"count": 2, "id": "minecraft:obsidian"}],
          "time": 200
        }"#;
        let recipe = CoolerRecipe::parse("oritech:cooler/obsidian", lava).expect("parses");
        assert_eq!(recipe.fluid.amount, 500, "an explicit amount is kept");
        assert_eq!(recipe.result.item, "minecraft:obsidian");
        assert_eq!(recipe.result.count, 2);

        let steam = r##"{
          "type": "oritech:cooler",
          "fluidInput": {"fluid": "#c:steam", "amount": 32},
          "ingredients": [],
          "results": [{"count": 3, "id": "minecraft:snow_block"}],
          "time": 200
        }"##;
        let recipe = CoolerRecipe::parse("oritech:cooler/snow", steam).expect("parses");
        assert_eq!(recipe.fluid.fluid, "#c:steam");
        assert!(recipe.fluid.is_tag());
    }

    #[test]
    fn the_cooler_parser_refuses_what_the_machine_cannot_run() {
        let good = r#"{"type":"oritech:cooler","fluidInput":{"fluid":"minecraft:water"},
            "ingredients":[],"results":[{"count":3,"id":"minecraft:ice"}],"time":200}"#;
        assert!(CoolerRecipe::parse("r", good).is_ok());

        let cases = [
            ("unreadable", "{", CoolerError::Unreadable),
            (
                "no type",
                r#"{"fluidInput":{"fluid":"minecraft:water"},
                    "results":[{"count":3,"id":"minecraft:ice"}],"time":200}"#,
                CoolerError::Unreadable,
            ),
            (
                "no fluid",
                r#"{"type":"oritech:cooler","fluidInput":{},
                    "results":[{"count":3,"id":"minecraft:ice"}],"time":200}"#,
                CoolerError::BadFluid,
            ),
            (
                "no fluidInput",
                r#"{"type":"oritech:cooler",
                    "results":[{"count":3,"id":"minecraft:ice"}],"time":200}"#,
                CoolerError::BadFluid,
            ),
            (
                "no results",
                r#"{"type":"oritech:cooler","fluidInput":{"fluid":"minecraft:water"},
                    "results":[],"time":200}"#,
                CoolerError::BadResult,
            ),
            (
                "two results",
                r#"{"type":"oritech:cooler","fluidInput":{"fluid":"minecraft:water"},
                    "results":[{"count":1,"id":"minecraft:ice"},{"count":1,"id":"minecraft:snow_block"}],
                    "time":200}"#,
                CoolerError::BadResult,
            ),
            (
                "a zero count",
                r#"{"type":"oritech:cooler","fluidInput":{"fluid":"minecraft:water"},
                    "results":[{"count":0,"id":"minecraft:ice"}],"time":200}"#,
                CoolerError::BadResult,
            ),
            (
                "no id",
                r#"{"type":"oritech:cooler","fluidInput":{"fluid":"minecraft:water"},
                    "results":[{"count":3}],"time":200}"#,
                CoolerError::BadResult,
            ),
            (
                "no time",
                r#"{"type":"oritech:cooler","fluidInput":{"fluid":"minecraft:water"},
                    "results":[{"count":3,"id":"minecraft:ice"}]}"#,
                CoolerError::BadTime,
            ),
            (
                "a zero time",
                r#"{"type":"oritech:cooler","fluidInput":{"fluid":"minecraft:water"},
                    "results":[{"count":3,"id":"minecraft:ice"}],"time":0}"#,
                CoolerError::BadTime,
            ),
        ];
        for (what, json, want) in cases {
            assert_eq!(CoolerRecipe::parse("r", json), Err(want), "{what}");
        }
    }

    #[test]
    fn a_cooler_output_must_fit_its_one_slot() {
        let ice = RecipeResult {
            item: "minecraft:ice".to_owned(),
            count: 3,
        };
        assert!(cooler_output_fits(None, &ice), "an empty slot takes 3");
        assert!(
            cooler_output_fits(Some(("minecraft:ice", 61)), &ice),
            "61 + 3 is exactly the cap"
        );
        assert!(
            !cooler_output_fits(Some(("minecraft:ice", 62)), &ice),
            "62 + 3 passes the 64-item cap"
        );
        assert!(
            !cooler_output_fits(Some(("minecraft:snow_block", 0)), &ice),
            "another item cannot share the slot"
        );
        assert!(
            !cooler_output_fits(
                None,
                &RecipeResult {
                    item: "minecraft:ice".to_owned(),
                    count: 65,
                }
            ),
            "a result larger than the cap never fits"
        );
    }

    #[test]
    fn one_craft_takes_exactly_the_recipes_amount_out_of_the_tank() {
        assert_eq!(after_draining(4_000, 1_000), Some(3_000));
        assert_eq!(
            after_draining(1_000, 1_000),
            None,
            "the last bucket empties the tank"
        );
        assert_eq!(after_draining(999, 1_000), None, "never negative");
    }

    #[test]
    fn the_lava_generators_numbers_are_the_jars() {
        // OritechConfig.generators.lavaGeneratorData:
        // `new BasicMachineConfig(b, "lavaGeneratorData", 100_000, 0, 64 * 8, 64)`.
        assert_eq!(LAVA_CAPACITY, 100_000, "energyCapacity");
        assert_eq!(LAVA_MAX_EXTRACT, 512, "maxEnergyExtraction = 64 * 8");
        assert_eq!(LAVA_ENERGY_PER_TICK, 64, "energyPerTick");
        assert_eq!(LAVA_TANK, 4_000, "SimpleFluidStorage(4 * bucketAmount())");
        assert_eq!(
            LAVA_CORE_OFFSET, CORE_OFFSET,
            "LavaGeneratorEntity.getCorePositions is the furnace's (0, 1, 0)"
        );
        assert_ne!(
            LAVA_CORE_OFFSET, COOLER_CORE_OFFSET,
            "the chiller's core is north, so the assembly cannot be hardcoded"
        );
        assert_eq!(lava_energy_usage(), 64, "energyPerTick * (1 / speed 1)");
        assert_eq!(lavagen_ticks(120), 120, "progress = recipeTime");
        assert_eq!(lavagen_ticks(0), 1, "a recipe is never zero ticks");
    }

    #[test]
    fn a_lava_burn_tick_makes_64_fe_and_spends_one_tick() {
        let mut generator = LavaGenerator {
            energy: 0,
            progress: 120,
            max_burn: 120,
            recipe: "oritech:lavagen/lava".to_owned(),
        };
        assert_eq!(generator.tick(), 64);
        assert_eq!(generator.energy, 64);
        assert_eq!(generator.progress, 119);
        assert_eq!(
            generator.max_burn, 120,
            "the recipe's length is not consumed"
        );
    }

    #[test]
    fn a_whole_lava_burn_is_120_ticks_and_7680_fe() {
        let mut generator = LavaGenerator {
            energy: 0,
            progress: 120,
            max_burn: 120,
            recipe: "oritech:lavagen/lava".to_owned(),
        };
        let mut made = 0;
        for _ in 0..120 {
            made += generator.tick();
        }
        assert_eq!(made, 7_680, "120 ticks x 64 FE");
        assert_eq!(generator.energy, 7_680);
        assert_eq!(generator.progress, 0, "the burn ended");
        assert_eq!(generator.tick(), 0, "an ended burn makes nothing");
        assert!(
            generator.wants_recipe(),
            "an ended burn wants the next recipe while the buffer has room"
        );
    }

    #[test]
    fn a_full_buffer_pauses_the_burn_and_keeps_the_rest() {
        // canFitEnergy: capacity >= amount + produced, so a buffer with 32 FE of
        // room cannot take a 64 FE tick and the burn waits for a drain.
        let mut generator = LavaGenerator {
            energy: LAVA_CAPACITY - 32,
            progress: 7,
            max_burn: 120,
            recipe: "oritech:lavagen/lava".to_owned(),
        };
        assert!(!generator.can_fit());
        assert_eq!(generator.tick(), 0);
        assert_eq!(generator.progress, 7, "the rest of the burn is kept");
        generator.energy = LAVA_CAPACITY - 64;
        assert!(generator.can_fit(), "exactly one tick's room fits");
        assert_eq!(generator.tick(), 64);
        assert_eq!(generator.progress, 6);
    }

    #[test]
    fn a_lava_recipe_only_loads_when_no_burn_is_left_and_the_buffer_has_room() {
        let mut generator = LavaGenerator::default();
        assert!(generator.wants_recipe(), "empty, so a recipe may load");
        assert!(generator.load_recipe(120, "oritech:lavagen/lava"));
        assert_eq!(generator.progress, 120);
        assert_eq!(generator.max_burn, 120);
        assert_eq!(generator.recipe, "oritech:lavagen/lava");
        assert!(
            !generator.load_recipe(800, "oritech:lavagen/sheolfire"),
            "a burning machine does not load another recipe"
        );
        assert_eq!(generator.recipe, "oritech:lavagen/lava");
        let mut full = LavaGenerator {
            energy: LAVA_CAPACITY,
            ..LavaGenerator::default()
        };
        assert!(!full.wants_recipe(), "a full buffer waits for a drain");
        assert!(!full.load_recipe(120, "oritech:lavagen/lava"));
        assert_eq!(full.progress, 0);
    }

    #[test]
    fn a_lava_recipe_is_100_mb_of_lava_for_120_ticks() {
        // The jar's data/oritech/recipe/lavagen/lava.json, verbatim.
        let json = r#"{
  "type": "oritech:lava_generator",
  "fluidInput": {
    "amount": 100,
    "fluid": "minecraft:lava"
  },
  "ingredients": [],
  "results": [],
  "time": 120
}"#;
        let recipe = LavaGenRecipe::parse("oritech:lavagen/lava", json)
            .expect("the jar's lava recipe parses");
        assert_eq!(recipe.fluid.fluid, "minecraft:lava");
        assert_eq!(recipe.fluid.amount, 100, "100 mB a burn");
        assert_eq!(recipe.time, 120, "time: 120");
        assert_eq!(lavagen_ticks(recipe.time), 120);
        assert_eq!(lava_energy_usage() * 120, 7_680);
    }

    #[test]
    fn the_sheolfire_recipe_names_a_fluid_this_pack_does_not_have() {
        // data/oritech/recipe/lavagen/sheolfire.json: the second recipe is the
        // same measured gap as the chiller's snow.json — the fluid is the mod's
        // own `oritech:still_sheol_fire`, which this package does not declare, so
        // the file is not carried and the recipe can never match here.
        let json = r#"{
  "type": "oritech:lava_generator",
  "fluidInput": {
    "amount": 100,
    "fluid": "oritech:still_sheol_fire"
  },
  "ingredients": [],
  "results": [],
  "time": 800
}"#;
        let recipe = LavaGenRecipe::parse("oritech:lavagen/sheolfire", json)
            .expect("the jar's sheolfire recipe parses");
        assert_eq!(recipe.fluid.fluid, "oritech:still_sheol_fire");
        assert_eq!(recipe.fluid.amount, 100);
        assert_eq!(recipe.time, 800);
        assert_ne!(
            recipe.fluid.fluid, "minecraft:lava",
            "only the lava recipe can run in this package"
        );
    }

    #[test]
    fn a_lava_recipe_reads_its_fluid_with_the_cooler_s_default_bucket() {
        let bare = r#"{"type": "oritech:lava_generator", "fluidInput": {"fluid": "minecraft:lava"}, "ingredients": [], "results": [], "time": 120}"#;
        let recipe = LavaGenRecipe::parse("r", bare).expect("a fluid id and a time are enough");
        assert_eq!(
            recipe.fluid.amount, 1_000,
            "FluidIngredient defaults to a bucket"
        );
        for (json, want) in [
            (
                r#"{"type": "oritech:lava_generator", "time": 120}"#,
                LavaGenError::BadFluid,
            ),
            (
                r#"{"type": "oritech:lava_generator", "fluidInput": {"fluid": "minecraft:lava"}}"#,
                LavaGenError::BadTime,
            ),
            (
                r#"{"type": "oritech:lava_generator", "fluidInput": {"fluid": "minecraft:lava"}, "time": 0}"#,
                LavaGenError::BadTime,
            ),
            (
                r#"{"fluidInput": {"fluid": "minecraft:lava"}, "time": 120}"#,
                LavaGenError::Unreadable,
            ),
            ("not json", LavaGenError::Unreadable),
        ] {
            assert_eq!(LavaGenRecipe::parse("r", json), Err(want), "{json}");
        }
    }

    #[test]
    fn the_tank_must_hold_a_lava_recipes_own_fluid_and_amount() {
        // `FluidMultiblockGeneratorBlockEntity.getRecipe`: the tank's fluid must
        // match and hold at least the recipe's amount. A tank of water is the
        // wrong-fluid control: it matches no lava recipe, so nothing burns.
        let lava = FluidInput {
            fluid: "minecraft:lava".to_owned(),
            amount: 100,
        };
        assert!(tank_matches(Some(("minecraft:lava", 100)), &lava));
        assert!(tank_matches(Some(("minecraft:lava", 4_000)), &lava));
        assert!(!tank_matches(Some(("minecraft:lava", 99)), &lava));
        assert!(
            !tank_matches(Some(("minecraft:water", 4_000)), &lava),
            "water is not lava"
        );
        assert!(!tank_matches(None, &lava), "an empty tank holds nothing");
    }

    #[test]
    fn the_lava_generator_pushes_to_the_jars_ten_targets() {
        // LavaGeneratorEntity.getOutputTargets: above(2), below, east, east.above,
        // south, south.above, west, west.above, north, north.above.
        let targets = lava_output_targets();
        assert_eq!(targets.len(), 10);
        assert_eq!(
            targets,
            &LAVA_OUTPUT_TARGETS[..],
            "the constant is what the push walks"
        );
        for want in [
            ([0, 2, 0], [0, -1, 0]),
            ([0, -1, 0], [0, -1, 0]),
            ([1, 0, 0], [-1, 0, 0]),
            ([1, 1, 0], [-1, 0, 0]),
            ([0, 0, 1], [0, 0, -1]),
            ([0, 1, 1], [0, 0, -1]),
            ([-1, 0, 0], [1, 0, 0]),
            ([-1, 1, 0], [1, 0, 0]),
            ([0, 0, -1], [0, 0, 1]),
            ([0, 1, -1], [0, 0, 1]),
        ] {
            assert!(targets.contains(&want), "missing target {want:?}");
        }
        assert!(
            !targets.iter().any(|(offset, _)| *offset == [0, 1, 0]),
            "there is no target directly above: the pair at (0, 2, 0) covers it"
        );
        let mut seen = targets.to_vec();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), 10, "no target is listed twice");
    }

    // --- slice 5: the addon system -------------------------------------------

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-5
    }

    #[test]
    fn the_six_default_addon_blocks_carry_the_jars_settings() {
        // `BlockContent`'s six declarations and `OritechStartupConfig`'s defaults.
        let speed = addon_settings(SPEED_ADDON).expect("the speed addon");
        assert!(close(speed.speed, 0.5), "speedAddonSpeed");
        assert!(close(speed.efficiency, 1.2), "speedAddonEfficiency");
        assert_eq!((speed.capacity_bonus, speed.insert_bonus), (0, 0));
        assert_eq!((speed.chambers, speed.burst_ticks), (0, 0));

        let efficiency = addon_settings(EFFICIENCY_ADDON).expect("the efficiency addon");
        assert!(close(efficiency.speed, 1.0));
        assert!(
            close(efficiency.efficiency, 0.8),
            "efficiencyAddonEfficiency"
        );

        let ultimate = addon_settings(ULTIMATE_ADDON).expect("the ultimate addon");
        assert!(close(ultimate.speed, 0.25), "ultimateAddonSpeed");
        assert!(close(ultimate.efficiency, 1.1), "ultimateAddonEfficiency");

        let processing = addon_settings(PROCESSING_ADDON).expect("the processing addon");
        assert!(close(processing.efficiency, 1.5), "chamberAddonEfficiency");
        assert_eq!(processing.chambers, 1, "withChambers(1)");

        let capacitor = addon_settings(CAPACITOR_ADDON).expect("the capacitor addon");
        assert_eq!(capacitor.capacity_bonus, 2_000_000);
        assert_eq!(capacitor.insert_bonus, 2_000);

        let burst = addon_settings(BURST_ADDON).expect("the burst addon");
        assert_eq!(burst.burst_ticks, 240, "burstAddonTicks");

        assert!(
            addon_settings("oritech:machine_acceptor_addon").is_none(),
            "the other addon kinds belong to machine families this port does not carry"
        );
        assert!(addon_settings("minecraft:iron_block").is_none());
    }

    #[test]
    fn additive_addons_convert_the_jars_settings_back_to_multipliers() {
        // `OritechConfig.additiveAddons` defaults to true: add the bonus, then
        // invert the accumulator (`gatherAddonStats`).
        let speed = Addons::gather(&[SPEED_ADDON], ADDON_MODE);
        assert!(close(speed.speed, 2.0 / 3.0), "1 / (1 + 0.5)");
        assert!(close(speed.efficiency, 1.2), "1 / 0.8, then 1 + 0.2");

        let efficiency = Addons::gather(&[EFFICIENCY_ADDON], ADDON_MODE);
        assert!(close(efficiency.speed, 1.0));
        assert!(close(efficiency.efficiency, 1.0 / 1.2));

        let ultimate = Addons::gather(&[ULTIMATE_ADDON], ADDON_MODE);
        assert!(close(ultimate.speed, 1.0 / 1.75));
        assert!(close(ultimate.efficiency, 1.1));

        let processing = Addons::gather(&[PROCESSING_ADDON], ADDON_MODE);
        assert!(close(processing.efficiency, 1.5));
        assert_eq!(processing.chambers, 1);

        let mixed = Addons::gather(&[SPEED_ADDON, EFFICIENCY_ADDON], ADDON_MODE);
        assert!(close(mixed.speed, 2.0 / 3.0));
        assert!(
            close(mixed.efficiency, 1.0),
            "the two efficiency bonuses cancel"
        );

        assert_eq!(Addons::gather(&[], ADDON_MODE), Addons::default());
    }

    #[test]
    fn two_speed_addons_stack_additively_and_the_other_mode_multiplies() {
        let additive = Addons::gather(&[SPEED_ADDON, SPEED_ADDON], AddonMode::Additive);
        assert!(close(additive.speed, 0.5), "1 / (1 + 0.5 + 0.5)");
        assert!(close(additive.efficiency, 1.4), "0.6 -> 1/0.6 -> 1 + 0.4");

        let multiplied = Addons::gather(&[SPEED_ADDON, SPEED_ADDON], AddonMode::Multiplicative);
        assert!(close(multiplied.speed, 0.25));
        assert!(close(multiplied.efficiency, 1.44));
    }

    #[test]
    fn a_capacitor_adds_its_capacity_and_insert_to_the_machines_own() {
        let addons = Addons::gather(&[CAPACITOR_ADDON], ADDON_MODE);
        assert_eq!(addons.capacity(PULVERIZER_CAPACITY), 2_025_000);
        assert_eq!(addons.insert(PULVERIZER_MAX_INSERT), 2_256);
        assert_eq!(
            Addons::default().capacity(PULVERIZER_CAPACITY),
            PULVERIZER_CAPACITY
        );
    }

    #[test]
    fn every_machine_names_the_addon_slots_the_jar_gives_it() {
        assert_eq!(addon_slots("oritech:pulverizer_block"), &[[1, 0, 0]]);
        assert_eq!(addon_slots("oritech:powered_furnace_block"), &[[0, -1, 0]]);
        assert_eq!(addon_slots("oritech:cooler_block"), &[[0, 0, -2]]);
        assert_eq!(
            addon_slots("oritech:lava_generator_block"),
            &[[1, 0, 0], [1, 1, 0]]
        );
        assert!(
            addon_slots("oritech:basic_generator_block").is_empty(),
            "`BasicGeneratorEntity.getAddonSlots` is empty"
        );
        assert!(addon_slots("minecraft:stone").is_empty());
    }

    #[test]
    fn the_readout_names_the_connected_addons_in_the_jars_order() {
        assert_eq!(addon_names(&[]), "none");
        assert_eq!(addon_names(&[SPEED_ADDON]), "speed");
        assert_eq!(
            addon_names(&[CAPACITOR_ADDON, SPEED_ADDON]),
            "speed+capacitor"
        );
        assert_eq!(addon_names(&[SPEED_ADDON, "minecraft:stone"]), "speed");
    }

    #[test]
    fn a_pulverizer_speed_addon_finishes_a_bone_in_67_ticks_for_3886_fe() {
        let addons = Addons::gather(&[SPEED_ADDON], ADDON_MODE);
        let bonus = Burst::default().bonus();
        let speed = speed_multiplier(&addons, 1.0, bonus);
        assert!(close(speed, 2.0 / 3.0));
        let cost = energy_usage(
            MACHINE_ENERGY_PER_TICK,
            1.0,
            speed,
            efficiency_multiplier(&addons, bonus),
        );
        assert_eq!(cost, 58, "32 x 1.2 / 0.6667 = 57.6, rounded up");
        let ticks = processing_ticks(100, speed);
        assert_eq!(ticks, 67, "100 x 0.6667");

        let mut machine = Pulverizer {
            energy: 10_000,
            ..Pulverizer::new()
        };
        let step_work = work("oritech:pulverizer/bone", ticks);
        for tick in 1..ticks {
            assert_eq!(
                machine.tick_with(step_work, cost),
                Step::Working,
                "tick {tick}"
            );
        }
        assert_eq!(machine.tick_with(step_work, cost), Step::Finished);
        assert_eq!(machine.progress, 0);
        assert_eq!(10_000 - machine.energy, 3_886);
    }

    #[test]
    fn an_efficiency_addon_makes_the_same_craft_cost_2700_fe() {
        let addons = Addons::gather(&[EFFICIENCY_ADDON], ADDON_MODE);
        let speed = speed_multiplier(&addons, 1.0, 1.0);
        let cost = energy_usage(
            MACHINE_ENERGY_PER_TICK,
            1.0,
            speed,
            efficiency_multiplier(&addons, 1.0),
        );
        assert_eq!(cost, 27, "32 x 0.8333 = 26.67, rounded up");
        assert_eq!(processing_ticks(100, speed), 100);
        assert_eq!(cost * 100, 2_700);
    }

    #[test]
    fn a_processing_addon_costs_4800_and_asks_for_a_second_chamber() {
        let addons = Addons::gather(&[PROCESSING_ADDON], ADDON_MODE);
        assert_eq!(addons.chambers, 1);
        let speed = speed_multiplier(&addons, 1.0, 1.0);
        let cost = energy_usage(
            MACHINE_ENERGY_PER_TICK,
            1.0,
            speed,
            efficiency_multiplier(&addons, 1.0),
        );
        assert_eq!(cost, 48, "32 x 1.5");
        assert_eq!(processing_ticks(100, speed), 100);
        assert_eq!(cost * 100, 4_800);
    }

    #[test]
    fn a_charged_burst_addon_runs_eight_times_as_fast_for_the_same_fe_a_tick() {
        let addons = Addons::gather(&[BURST_ADDON], ADDON_MODE);
        assert_eq!(addons.burst_ticks, 240);
        let mut burst = Burst::default();
        for _ in 0..240 {
            burst.recharge(addons.burst_ticks);
        }
        assert_eq!(burst.remaining, 240);
        let bonus = burst.bonus();
        assert!(close(bonus, 1.0 / 8.0));
        let speed = speed_multiplier(&addons, 1.0, bonus);
        assert_eq!(processing_ticks(100, speed), 13, "100 x 0.125");
        let cost = energy_usage(
            MACHINE_ENERGY_PER_TICK,
            1.0,
            speed,
            efficiency_multiplier(&addons, bonus),
        );
        assert_eq!(
            cost, MACHINE_ENERGY_PER_TICK,
            "the burst bonus cancels in the usage"
        );
    }

    #[test]
    fn a_burst_counter_charges_one_a_tick_drains_two_a_work_tick_and_throttles() {
        let mut burst = Burst::default();
        burst.recharge(240);
        assert_eq!(burst.remaining, 1);
        for _ in 0..240 {
            burst.recharge(240);
        }
        assert_eq!(burst.remaining, 240, "capped at the addon's ticks");
        burst.spend(240);
        burst.spend(240);
        assert_eq!(burst.remaining, 236);

        let throttled = Burst { remaining: -240 };
        assert!(close(throttled.bonus(), 1.2));
        let mut floored = Burst { remaining: -240 };
        floored.spend(240);
        assert_eq!(floored.remaining, -240, "floored at -max");

        assert_eq!(Burst { remaining: 500 }.clamped(240).remaining, 240);
        assert_eq!(Burst { remaining: -500 }.clamped(240).remaining, -240);
        assert_eq!(Burst { remaining: 900 }.clamped(0).remaining, 0);
    }

    #[test]
    fn a_furnace_speed_addon_smelts_in_67_ticks_for_3886_fe() {
        let addons = Addons::gather(&[SPEED_ADDON], ADDON_MODE);
        let speed = speed_multiplier(&addons, FURNACE_SPEED as f32, 1.0);
        assert!(close(speed, (2.0 / 3.0) * 0.5));
        let cost = energy_usage(
            FURNACE_ENERGY_PER_TICK,
            2.0,
            speed,
            efficiency_multiplier(&addons, 1.0),
        );
        assert_eq!(cost, 58, "32 x 1.2 x (1 / 0.3333) / 2");
        let ticks = processing_ticks(200, speed);
        assert_eq!(ticks, 67);

        let mut machine = Furnace {
            energy: 10_000,
            ..Furnace::new()
        };
        let step_work = work("minecraft:iron_ingot_from_smelting_iron_ore", ticks);
        for _ in 1..ticks {
            assert_eq!(machine.tick_with(step_work, cost), Step::Working);
        }
        assert_eq!(machine.tick_with(step_work, cost), Step::Finished);
        assert_eq!(10_000 - machine.energy, 3_886);
    }

    #[test]
    fn a_chiller_speed_addon_takes_134_ticks_for_7772_fe() {
        let addons = Addons::gather(&[SPEED_ADDON], ADDON_MODE);
        let speed = speed_multiplier(&addons, 1.0, 1.0);
        let cost = energy_usage(
            COOLER_ENERGY_PER_TICK,
            1.0,
            speed,
            efficiency_multiplier(&addons, 1.0),
        );
        assert_eq!(cost, 58);
        let ticks = processing_ticks(200, speed);
        assert_eq!(ticks, 134, "200 x 0.6667");
        assert_eq!(i32::try_from(ticks).unwrap() * cost, 7_772);
    }

    #[test]
    fn a_lava_generator_speed_addon_burns_66_ticks_at_115_fe() {
        let addons = Addons::gather(&[SPEED_ADDON], ADDON_MODE);
        let speed = speed_multiplier(&addons, 1.0, 1.0);
        let efficiency = efficiency_multiplier(&addons, 1.0);
        let ticks = generator_burn_ticks(120, speed, efficiency);
        assert_eq!(ticks, 66, "(int)(120 x 0.6667 / 1.2)");
        let output = generator_output(LAVA_ENERGY_PER_TICK, speed, efficiency);
        assert_eq!(output, 115, "64 x 1.2 / 0.6667 = 115.2, truncated");
        assert_eq!(i32::try_from(ticks).unwrap() * output, 7_590);

        let mut machine = LavaGenerator::new();
        assert!(machine.load_recipe_for(ticks, "oritech:lavagen/lava", output));
        let mut made = 0;
        for _ in 0..ticks {
            made += machine.tick_with(output);
        }
        assert_eq!(made, 7_590);
        assert_eq!(machine.energy, 7_590);
    }

    #[test]
    fn no_addon_keeps_every_machines_own_numbers() {
        let addons = Addons::default();
        assert_eq!(
            energy_usage(
                MACHINE_ENERGY_PER_TICK,
                1.0,
                speed_multiplier(&addons, 1.0, 1.0),
                efficiency_multiplier(&addons, 1.0),
            ),
            32
        );
        assert_eq!(
            energy_usage(
                FURNACE_ENERGY_PER_TICK,
                2.0,
                speed_multiplier(&addons, FURNACE_SPEED as f32, 1.0),
                efficiency_multiplier(&addons, 1.0),
            ),
            32
        );
        assert_eq!(
            processing_ticks(100, speed_multiplier(&addons, 1.0, 1.0)),
            100
        );
        assert_eq!(
            processing_ticks(200, speed_multiplier(&addons, FURNACE_SPEED as f32, 1.0)),
            100
        );
        assert_eq!(generator_output(LAVA_ENERGY_PER_TICK, 1.0, 1.0), 64);
        assert_eq!(generator_burn_ticks(120, 1.0, 1.0), 120);
    }
}
