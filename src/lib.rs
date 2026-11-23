//! Oritech for Horizon — slices 1 to 10: the Basic Generator, the Pulverizer, the
//! Powered Furnace, the Industrial Chiller, the Lava Generator, the machine addon
//! system, the Control Unit Addon's redstone gate, the material and component
//! chain, the Assembler, the Centrifuge and the Foundry.
//!
//! One flat SDK package (mod API 0.2): the carried machines, their models,
//! textures, lang, loot tables, recipes, tags and creative tab are this
//! package's `assets/` and `data/`, and the behaviour is the one
//! `impl mod_api::Hooks` plus its block-entity Parts. The rules themselves
//! live in [`runtime`], byte-identical to the retired `.hnmod` bundle's
//! `runtime.rs` up to slice 1; [`support`] is the glue that reads the game,
//! calls them and writes the result back.
//!
//! What the retired bundle expressed as eleven injected helpers and two support
//! modules is here three shapes: each machine's block entity is a **Part**
//! attached to the type declared for its block, its container and its energy
//! buffer are **declarations** on the block key, and the rest are typed `Hooks`
//! methods. Its `embedded-content` helper has no successor: one `.hmod` carries
//! the content and the code, which is what that helper existed to work around.
pub mod runtime;

/// The dependency-free JSON reader the recipe parsers use (the registry refuses a registry crate).
mod json;
mod support;

use mod_api::{
    self, BlockEntityEvent, BlockEntityPart, BlockEntityTickEvent, BlockEntityTickReply,
    BlockEntityTicksEvent, BlockPart, Command, CommandArgumentValue, Compound, Flow, Hooks, Host,
    HostSide, LevelSide, LevelTickPostEvent, RegisterCommandsEvent, Registrar, RegistrationContext,
    RightClickBlockEvent, RunModCommandEvent, ServerHost, Text, WorldLifecycleEvent,
};

#[derive(Default)]
struct Mod;

#[mod_api::horizon_mod]
impl Hooks for Mod {
    /// Declare the two machines: one block entity each, the generator's fuel
    /// slot and output-only buffer, the pulverizer's input, two outputs and
    /// input-only buffer.
    ///
    /// A declaration is keyed by the block, so a foreign block's container or
    /// buffer cannot be touched: the retired `block_entity_container` /
    /// `block_entity_energy` hooks wrote into one shared output and had to leave
    /// another mod's entry alone.
    fn register(&self, registrar: &mut dyn Registrar, context: &RegistrationContext) {
        // The carried `data/oritech/recipe/pulverizer/*.json` declare this type; without a
        // serializer the game refuses every one of them at load ("unregistered recipe
        // serializer"), which is a hard startup error. The port reads the JSON itself (the
        // pulverizer's own matcher), so the serializer only has to accept and keep the object.
        registrar.register_recipe_type(support::PULVERIZER_RECIPE);
        registrar
            .register_raw_recipe_serializer(support::PULVERIZER_RECIPE, support::PULVERIZER_RECIPE);
        // The chiller's two recipes name `oritech:cooler`, and the port reads them with its own
        // parser for the same reason: a 26.3 loader knows nothing of this mod's recipe kind.
        registrar.register_recipe_type(support::COOLER_RECIPE);
        registrar.register_raw_recipe_serializer(support::COOLER_RECIPE, support::COOLER_RECIPE);
        // The lava generator's one carried recipe names `oritech:lava_generator`, with the same
        // reason (and the same startup failure without it).
        registrar.register_recipe_type(support::LAVA_RECIPE);
        registrar.register_raw_recipe_serializer(support::LAVA_RECIPE, support::LAVA_RECIPE);
        // The assembler's five carried recipes name `oritech:assembler`, with four ingredients
        // each and one result; the port reads them with its own parser for the same reason.
        registrar.register_recipe_type(support::ASSEMBLER_RECIPE);
        registrar
            .register_raw_recipe_serializer(support::ASSEMBLER_RECIPE, support::ASSEMBLER_RECIPE);
        // The foundry's four carried alloys name `oritech:foundry`; without its serializer the
        // game refuses the whole data pack at load ("unregistered recipe serializer
        // oritech:foundry") and never opens the world — the same hard startup error the
        // pulverizer's and the centrifuge's notes record.
        registrar.register_recipe_type(support::FOUNDRY_RECIPE);
        registrar.register_raw_recipe_serializer(support::FOUNDRY_RECIPE, support::FOUNDRY_RECIPE);
        // The centrifuge's two carried recipes name its item and fluid tables
        // (`oritech:centrifuge`, `oritech:centrifuge_fluid`); without both serializers the
        // game refuses the files at load, the same hard startup error as the pulverizer's.
        registrar.register_recipe_type(support::CENTRIFUGE_RECIPE);
        registrar
            .register_raw_recipe_serializer(support::CENTRIFUGE_RECIPE, support::CENTRIFUGE_RECIPE);
        registrar.register_recipe_type(support::CENTRIFUGE_FLUID_RECIPE);
        registrar.register_raw_recipe_serializer(
            support::CENTRIFUGE_FLUID_RECIPE,
            support::CENTRIFUGE_FLUID_RECIPE,
        );
        for block in [
            support::GENERATOR,
            support::PULVERIZER,
            support::FURNACE,
            support::COOLER,
            support::LAVA,
            support::ASSEMBLER,
            support::CENTRIFUGE,
            support::FOUNDRY,
        ] {
            let key = registrar.block_key(block);
            if let Some(container) = support::container(block) {
                registrar.block_container(&key, container);
            }
            if let Some(battery) = support::battery(block) {
                registrar.block_energy(&key, battery);
            }
            if let Some(tanks) = support::fluids(block) {
                registrar.block_fluids(&key, tanks);
            }
        }
        let generator = registrar
            .block_entity_type(support::GENERATOR_TYPE)
            .blocks(&[support::GENERATOR])
            .build();
        let pulverizer = registrar
            .block_entity_type(support::PULVERIZER_TYPE)
            .blocks(&[support::PULVERIZER])
            .build();
        let furnace = registrar
            .block_entity_type(support::FURNACE_TYPE)
            .blocks(&[support::FURNACE])
            .build();
        let cooler = registrar
            .block_entity_type(support::COOLER_TYPE)
            .blocks(&[support::COOLER])
            .build();
        let lava = registrar
            .block_entity_type(support::LAVA_TYPE)
            .blocks(&[support::LAVA])
            .build();
        let assembler = registrar
            .block_entity_type(support::ASSEMBLER_TYPE)
            .blocks(&[support::ASSEMBLER])
            .build();
        let centrifuge = registrar
            .block_entity_type(support::CENTRIFUGE_TYPE)
            .blocks(&[support::CENTRIFUGE])
            .build();
        let foundry = registrar
            .block_entity_type(support::FOUNDRY_TYPE)
            .blocks(&[support::FOUNDRY])
            .build();
        if let Err(error) = registrar.block_entity_part(&generator, GeneratorPart) {
            context.refuse(&error.to_string());
        }
        if let Err(error) = registrar.block_entity_part(&pulverizer, PulverizerPart) {
            context.refuse(&error.to_string());
        }
        if let Err(error) = registrar.block_entity_part(&furnace, FurnacePart) {
            context.refuse(&error.to_string());
        }
        if let Err(error) = registrar.block_entity_part(&cooler, CoolerPart) {
            context.refuse(&error.to_string());
        }
        if let Err(error) = registrar.block_entity_part(&lava, LavaPart) {
            context.refuse(&error.to_string());
        }
        if let Err(error) = registrar.block_entity_part(&assembler, AssemblerPart) {
            context.refuse(&error.to_string());
        }
        if let Err(error) = registrar.block_entity_part(&centrifuge, CentrifugePart) {
            context.refuse(&error.to_string());
        }
        if let Err(error) = registrar.block_entity_part(&foundry, FoundryPart) {
            context.refuse(&error.to_string());
        }
    }

    /// `/oritech info|energy <x> <y> <z> [amount]`: the harness controls. Upstream
    /// has no commands of its own (its GUI and config are the only controls);
    /// these exist so a player without the GUI, and the harness, can read a
    /// machine's buffers.
    fn register_commands(&self, _host: &mut dyn ServerHost, event: &mut RegisterCommandsEvent<'_>) {
        event
            .commands
            .push(Command::new("oritech info <pos:block_pos>"));
        event.commands.push(Command::op(
            "oritech energy <pos:block_pos> <amount:integer>",
            2,
        ));
        // The chiller's tank, filled through the engine's own fluid handler: the
        // harness has no bucket path into a mod block entity, and this is the
        // same harness-control shape as `/oritech energy`. The fluid is a greedy
        // `text`: the pin's `word` argument stops at a `:`, so a namespaced fluid
        // id cannot be one.
        event.commands.push(Command::op(
            "oritech fluid <pos:block_pos> <amount:integer> <fluid:text>",
            2,
        ));
        // A machine's *output* slots have no other route: the right click fills
        // an input slot only, and `ctl container` cannot see a mod block entity.
        // The item is a greedy `text` for the same `:` reason as the fluid.
        event.commands.push(Command::op(
            "oritech slot <pos:block_pos> <slot:integer> <count:integer> <item:text>",
            2,
        ));
    }

    /// The command bodies. The dispatcher parses and resolves the arguments, so
    /// this only reads the machine and answers.
    fn run_mod_command(
        &self,
        host: &mut dyn ServerHost,
        event: &mut RunModCommandEvent<'_>,
    ) -> Flow {
        let literals: Vec<&str> = event.command.literals.iter().map(String::as_str).collect();
        let line = match literals.as_slice() {
            ["oritech", "info"] => {
                let Some(CommandArgumentValue::BlockPos(pos)) = event.command.argument("pos")
                else {
                    return Flow::Continue;
                };
                support::info_line(host, *pos)
                    .unwrap_or_else(|| format!("{pos:?} is not a mod block entity."))
            }
            ["oritech", "energy"] => {
                let (
                    Some(CommandArgumentValue::BlockPos(pos)),
                    Some(CommandArgumentValue::Integer(amount)),
                ) = (
                    event.command.argument("pos"),
                    event.command.argument("amount"),
                )
                else {
                    return Flow::Continue;
                };
                support::set_energy(host, *pos, *amount)
                    .unwrap_or_else(|| format!("{pos:?} is not an Oritech machine."))
            }
            ["oritech", "fluid"] => {
                let (
                    Some(CommandArgumentValue::BlockPos(pos)),
                    Some(CommandArgumentValue::Integer(amount)),
                    Some(CommandArgumentValue::String(fluid)),
                ) = (
                    event.command.argument("pos"),
                    event.command.argument("amount"),
                    event.command.argument("fluid"),
                )
                else {
                    return Flow::Continue;
                };
                support::set_fluid(host, *pos, fluid.trim(), *amount)
                    .unwrap_or_else(|| format!("{pos:?} has no Oritech fluid tank."))
            }
            ["oritech", "slot"] => {
                let (
                    Some(CommandArgumentValue::BlockPos(pos)),
                    Some(CommandArgumentValue::Integer(slot)),
                    Some(CommandArgumentValue::Integer(count)),
                    Some(CommandArgumentValue::String(item)),
                ) = (
                    event.command.argument("pos"),
                    event.command.argument("slot"),
                    event.command.argument("count"),
                    event.command.argument("item"),
                )
                else {
                    return Flow::Continue;
                };
                support::set_slot(host, *pos, *slot, *count, item.trim())
                    .unwrap_or_else(|| format!("{pos:?} has no Oritech container."))
            }
            _ => return Flow::Continue,
        };
        let _ = host.commands().feedback(Text::literal(line));
        Flow::Cancel
    }

    /// A right click on a machine inserts fuel or a recipe input, or reads its
    /// state with an empty hand; there is no GUI in slice 1 (`right_click_block`
    /// is the pre-block, cancellable route the retired bundle used).
    fn right_click_block(&self, host: HostSide<'_>, event: &mut RightClickBlockEvent<'_>) -> Flow {
        support::right_click(host, event)
    }

    /// After a level's tick: the queued generator positions hand their FE to the
    /// pulverizers beside them. A ticked block entity is out of the world during
    /// its own tick, which is why this waits for here.
    fn level_tick_post(&self, host: HostSide<'_>, event: &mut LevelTickPostEvent) {
        let HostSide::Server(host) = host else {
            return;
        };
        support::on_level_tick_post(host, event.level);
    }

    /// A world opened or closed: the parsed recipe cache and the push queue
    /// belong to one world.
    fn world_lifecycle(&self, _host: &mut dyn ServerHost, _event: &mut WorldLifecycleEvent<'_>) {
        support::reset_world_state();
    }
}

/// The Basic Generator's block entity.
struct GeneratorPart;
/// The Pulverizer's block entity.
struct PulverizerPart;
/// The Powered Furnace's block entity.
struct FurnacePart;
/// The Industrial Chiller's block entity.
struct CoolerPart;
/// The Lava Generator's block entity.
struct LavaPart;
/// The Assembler's block entity.
struct AssemblerPart;
/// The Centrifuge's block entity.
struct CentrifugePart;
/// The Foundry's block entity.
struct FoundryPart;

#[mod_api::horizon_part]
impl BlockPart for GeneratorPart {}

#[mod_api::horizon_part]
impl BlockEntityPart for GeneratorPart {
    /// `EntityBlock.newBlockEntity`: an empty saved compound; the first tick
    /// fills it.
    fn block_entity(
        &self,
        _level: LevelSide,
        _event: &mut BlockEntityEvent<'_>,
    ) -> Option<Compound> {
        Some(Compound::new())
    }

    /// `EntityBlock.getTicker`: it ticks on the server.
    fn block_entity_ticks(&self, _host: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        event.ticks = true;
    }

    /// One server tick: load one fuel item when upstream would, burn one tick
    /// for 32 FE, then queue the adjacent transfer.
    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        let Some(level) = support::in_place_level(host) else {
            return;
        };
        let pos = event.pos;
        let mut nbt = std::mem::take(&mut event.entity.nbt);
        let changed = support::tick_generator(host, level, pos, &mut nbt, &mut event.entity.items);
        event.entity.nbt = nbt;
        if !changed {
            *event.reply = BlockEntityTickReply::Unchanged;
        }
    }
}

#[mod_api::horizon_part]
impl BlockPart for PulverizerPart {}

#[mod_api::horizon_part]
impl BlockEntityPart for PulverizerPart {
    fn block_entity(
        &self,
        _level: LevelSide,
        _event: &mut BlockEntityEvent<'_>,
    ) -> Option<Compound> {
        Some(Compound::new())
    }

    fn block_entity_ticks(&self, _host: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        event.ticks = true;
    }

    /// One server tick: find the matching recipe, check the output room, and let
    /// the pure state machine spend FE and advance. A Control Unit Addon in the
    /// machine's slot with a signal on it holds the whole tick (upstream's
    /// `disabledViaRedstone`), which is why the tick can answer `Unchanged`.
    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        let Some(level) = support::in_place_level(host) else {
            return;
        };
        let pos = event.pos;
        let mut nbt = std::mem::take(&mut event.entity.nbt);
        let changed = support::tick_pulverizer(host, level, pos, &mut nbt, &mut event.entity.items);
        event.entity.nbt = nbt;
        if !changed {
            *event.reply = BlockEntityTickReply::Unchanged;
        }
    }
}

#[mod_api::horizon_part]
impl BlockPart for FurnacePart {}

#[mod_api::horizon_part]
impl BlockEntityPart for FurnacePart {
    fn block_entity(
        &self,
        _level: LevelSide,
        _event: &mut BlockEntityEvent<'_>,
    ) -> Option<Compound> {
        Some(Compound::new())
    }

    fn block_entity_ticks(&self, _host: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        event.ticks = true;
    }

    /// One server tick: a machine whose core is gone is disassembled, an
    /// unassembled one does nothing, and an assembled one smelts one vanilla
    /// smelting recipe at twice a furnace's speed.
    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        let Some(level) = support::in_place_level(host) else {
            return;
        };
        let pos = event.pos;
        let mut nbt = std::mem::take(&mut event.entity.nbt);
        let changed = support::tick_furnace(host, level, pos, &mut nbt, &mut event.entity.items);
        event.entity.nbt = nbt;
        if !changed {
            *event.reply = BlockEntityTickReply::Unchanged;
        }
    }
}

#[mod_api::horizon_part]
impl BlockPart for CoolerPart {}

#[mod_api::horizon_part]
impl BlockEntityPart for CoolerPart {
    fn block_entity(
        &self,
        _level: LevelSide,
        _event: &mut BlockEntityEvent<'_>,
    ) -> Option<Compound> {
        Some(Compound::new())
    }

    fn block_entity_ticks(&self, _host: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        event.ticks = true;
    }

    /// One server tick: the furnace's assembly gate, then one cooler craft from
    /// the fluid in the tank, which the craft takes its millibuckets out of.
    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        let Some(level) = support::in_place_level(host) else {
            return;
        };
        let pos = event.pos;
        let mut nbt = std::mem::take(&mut event.entity.nbt);
        let changed = support::tick_cooler(host, level, pos, &mut nbt, &mut event.entity.items);
        event.entity.nbt = nbt;
        if !changed {
            *event.reply = BlockEntityTickReply::Unchanged;
        }
    }
}

#[mod_api::horizon_part]
impl BlockPart for LavaPart {}

#[mod_api::horizon_part]
impl BlockEntityPart for LavaPart {
    fn block_entity(
        &self,
        _level: LevelSide,
        _event: &mut BlockEntityEvent<'_>,
    ) -> Option<Compound> {
        Some(Compound::new())
    }

    fn block_entity_ticks(&self, _host: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        event.ticks = true;
    }

    /// One server tick: the multiblock gate, then one lava burn from the tank —
    /// 64 FE a tick into a 100,000 FE buffer that only gives energy out.
    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        let Some(level) = support::in_place_level(host) else {
            return;
        };
        let pos = event.pos;
        let mut nbt = std::mem::take(&mut event.entity.nbt);
        let changed = support::tick_lava_generator(host, level, pos, &mut nbt);
        event.entity.nbt = nbt;
        if !changed {
            *event.reply = BlockEntityTickReply::Unchanged;
        }
    }
}

#[mod_api::horizon_part]
impl BlockPart for AssemblerPart {}

#[mod_api::horizon_part]
impl BlockEntityPart for AssemblerPart {
    fn block_entity(
        &self,
        _level: LevelSide,
        _event: &mut BlockEntityEvent<'_>,
    ) -> Option<Compound> {
        Some(Compound::new())
    }

    fn block_entity_ticks(&self, _host: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        event.ticks = true;
    }

    /// One server tick: the three-core assembly gate, then one `oritech:assembler`
    /// craft from the four input slots into the one output.
    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        let Some(level) = support::in_place_level(host) else {
            return;
        };
        let pos = event.pos;
        let mut nbt = std::mem::take(&mut event.entity.nbt);
        let changed = support::tick_assembler(host, level, pos, &mut nbt, &mut event.entity.items);
        event.entity.nbt = nbt;
        if !changed {
            *event.reply = BlockEntityTickReply::Unchanged;
        }
    }
}

#[mod_api::horizon_part]
impl BlockPart for FoundryPart {}

#[mod_api::horizon_part]
impl BlockPart for CentrifugePart {}

#[mod_api::horizon_part]
impl BlockEntityPart for FoundryPart {
    fn block_entity(
        &self,
        _level: LevelSide,
        _event: &mut BlockEntityEvent<'_>,
    ) -> Option<Compound> {
        Some(Compound::new())
    }

    fn block_entity_ticks(&self, _host: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        event.ticks = true;
    }

    /// One server tick: the three-core assembly gate, then one `oritech:foundry`
    /// alloy from the two input slots into the one output.
    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        let Some(level) = support::in_place_level(host) else {
            return;
        };
        let pos = event.pos;
        let mut nbt = std::mem::take(&mut event.entity.nbt);
        let changed = support::tick_foundry(host, level, pos, &mut nbt, &mut event.entity.items);
        event.entity.nbt = nbt;
        if !changed {
            *event.reply = BlockEntityTickReply::Unchanged;
        }
    }
}

#[mod_api::horizon_part]
impl BlockEntityPart for CentrifugePart {
    fn block_entity(
        &self,
        _level: LevelSide,
        _event: &mut BlockEntityEvent<'_>,
    ) -> Option<Compound> {
        Some(Compound::new())
    }

    fn block_entity_ticks(&self, _host: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        event.ticks = true;
    }

    /// One server tick: the one-core assembly gate, then one `oritech:centrifuge`
    /// craft — or, with a Fluid Addon in a slot, the fluid table — from the one
    /// input slot into the two outputs.
    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        let Some(level) = support::in_place_level(host) else {
            return;
        };
        let pos = event.pos;
        let mut nbt = std::mem::take(&mut event.entity.nbt);
        let changed = support::tick_centrifuge(host, level, pos, &mut nbt, &mut event.entity.items);
        event.entity.nbt = nbt;
        if !changed {
            *event.reply = BlockEntityTickReply::Unchanged;
        }
    }
}
