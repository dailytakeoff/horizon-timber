// Timber's game side: compiled into the game as `crate::game::timber_glue`.
// It reads the world for `timber::plan`, then breaks each block through the
// game's own player-mining code, so drops, Fortune/Silk Touch, XP, tool wear,
// stats and block sounds all work exactly as if the player broke them by hand.

use crate::game::Game;
use crate::game::timber::{self, Kind, TreeWorld};
use std::cell::Cell;

thread_local! {
    // True while Timber itself is breaking blocks. Each of those breaks runs
    // the destroy-block hook again; this stops it from starting a new vein.
    static FELLING: Cell<bool> = const { Cell::new(false) };
}

/// Clears `FELLING` when it goes out of scope, even if a break panics, so one
/// bad vein can't switch Timber off for the rest of the session.
struct Felling;
impl Drop for Felling {
    fn drop(&mut self) {
        FELLING.set(false);
    }
}

/// Lets the pure logic read the world through the game's own helpers.
struct View<'a, 'g>(&'a Game<'g>);

impl TreeWorld for View<'_, '_> {
    fn block(&self, pos: [i32; 3]) -> String {
        self.0.native_block(pos)
    }
    fn leaf_is_natural(&self, pos: [i32; 3]) -> bool {
        self.0.native_block_property(pos, "persistent").as_deref() != Some("true")
    }
}

/// The held stack's uses left, None when it has no durability.
fn uses_left(game: &Game<'_>) -> Option<u32> {
    let stack = game.inventory.get(game.selected)?.as_ref()?;
    let max = game.res.items().get(&stack.item)?.max_damage;
    (max > 0).then(|| timber::durability_left(stack.damage, max))
}

/// Still holding the right tool for `kind`, with uses to spare?
fn can_keep_going(game: &Game<'_>, kind: Kind) -> bool {
    let fits = game
        .native_held()
        .is_some_and(|(item, _, _)| kind.tool_fits(&item));
    fits && uses_left(game).is_none_or(|left| left > timber::KEEP_DURABILITY)
}

/// Runs when a player is about to break the block at `pos` (the HEAD of
/// `native_destroy_block`). The block itself is left to the game.
pub(crate) fn on_destroy_block(game: &mut Game<'_>, pos: [i32; 3], canceled: &mut bool) {
    // Another mod (a land claim) already stopped this break, or we are inside
    // our own vein.
    if *canceled || FELLING.get() {
        return;
    }
    // Switched off in Options > Mods.
    if !game.native_on("timber") {
        return;
    }
    // Survival only (0). In creative everything breaks instantly anyway.
    if game.native_acting_player().is_none_or(|p| p.game_mode != 0) {
        return;
    }
    // Sneak to break one block normally.
    if game.player.as_ref().is_some_and(|p| p.shift_key_down) {
        return;
    }

    // A log or an ore? That decides which tool counts.
    let block = game.native_block(pos);
    let Some((kind, _)) = timber::family(&block) else {
        return;
    };
    if !can_keep_going(game, kind) {
        return;
    }
    // Ores: only with a pickaxe good enough to drop them (iron or better for
    // diamond...), or a stone pickaxe would wipe a vein for nothing.
    if kind == Kind::Ore && !game.res.items().correct_for_drops(game.held(), &block) {
        return;
    }

    let Some(targets) = timber::plan(&View(game), pos, &block, kind.limit()) else {
        return;
    };

    FELLING.set(true);
    let _felling = Felling;
    for target in targets {
        if !can_keep_going(game, kind) {
            break; // tool nearly broken, or the player swapped items
        }
        // An earlier break may have changed this spot (gravel fell in...).
        if timber::family(&game.native_block(target)) != timber::family(&block) {
            continue;
        }
        // The game's own survival break: checks, drops, XP, tool wear, sound.
        let _ = game.break_block_as_player(target);
    }
}
