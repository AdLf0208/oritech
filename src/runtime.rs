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
        advance(
            &mut self.energy,
            &mut self.progress,
            &mut self.recipe,
            work,
            MACHINE_ENERGY_PER_TICK,
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
        advance(
            &mut self.energy,
            &mut self.progress,
            &mut self.recipe,
            work,
            furnace_energy_usage(),
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
}
