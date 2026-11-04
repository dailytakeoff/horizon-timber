// Timber: the vein-finding logic for trees and ores, with no game types in it.
//
// This one file is used twice:
//   * by Cargo as a normal library, so `cargo test` checks it on its own;
//   * by horizon-mixins, which copies it into the game as `crate::game::timber`.
// So it may only use std, and talks to the world through the `TreeWorld` trait.
// (horizon-mixins refuses `//!` comments and `mod foo;` files in module payloads.)

use std::collections::{HashSet, VecDeque};

/// Most logs one swing may fell. Big jungle and dark oak trees are ~100-200.
pub const MAX_LOGS: usize = 256;

/// Most ores one swing may mine. Big vanilla veins (iron, copper) reach ~50.
pub const MAX_ORES: usize = 64;

/// A tree needs at least this many natural (not player-placed) leaves touching
/// its logs, so log cabins and log walls are never felled.
pub const MIN_LEAVES: usize = 3;

/// Stop while the tool has this many uses left, so it never breaks.
pub const KEEP_DURABILITY: u32 = 1;

/// What the logic needs to read from the world.
pub trait TreeWorld {
    /// Block id at `pos`, like `minecraft:oak_log`.
    fn block(&self, pos: [i32; 3]) -> String;
    /// False for leaves a player placed (`persistent=true`).
    fn leaf_is_natural(&self, pos: [i32; 3]) -> bool;
}

/// What a block is to Timber.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Tree,
    Ore,
}

impl Kind {
    /// The tool that starts a vein of this kind.
    pub fn tool_fits(self, item: &str) -> bool {
        match self {
            Kind::Tree => is_axe(item),
            Kind::Ore => is_pickaxe(item),
        }
    }
    /// Most blocks one swing may take.
    pub fn limit(self) -> usize {
        match self {
            Kind::Tree => MAX_LOGS,
            Kind::Ore => MAX_ORES,
        }
    }
}

/// Logs, wood, stems and hyphae, stripped or not. Not bamboo blocks or roots.
pub fn is_log(block: &str) -> bool {
    let name = block.strip_prefix("minecraft:").unwrap_or(block);
    ["_log", "_wood", "_stem", "_hyphae"]
        .iter()
        .any(|suffix| name.ends_with(suffix))
}

/// Every ore (`iron_ore`, `deepslate_iron_ore`, `nether_quartz_ore`...) and
/// ancient debris, which is an ore in all but name.
pub fn is_ore(block: &str) -> bool {
    let name = block.strip_prefix("minecraft:").unwrap_or(block);
    name.ends_with("_ore") || name == "ancient_debris"
}

/// Leaves, plus the nether "leaves": wart blocks and shroomlight on fungi.
pub fn is_leaf(block: &str) -> bool {
    let name = block.strip_prefix("minecraft:").unwrap_or(block);
    name.ends_with("_leaves") || name.ends_with("_wart_block") || name == "shroomlight"
}

/// Every axe ends in `_axe`. (A pickaxe ends in `_pickaxe`: the letter before
/// `axe` is `k`, not `_`, so it never matches.)
pub fn is_axe(item: &str) -> bool {
    item.ends_with("_axe")
}

pub fn is_pickaxe(item: &str) -> bool {
    item.ends_with("_pickaxe")
}

/// The kind and type of a block: `minecraft:stripped_oak_log` -> (Tree, `oak`),
/// `minecraft:deepslate_iron_ore` -> (Ore, `iron`). A vein only follows one
/// type, so an oak never takes the birch beside it, nor iron the coal.
pub fn family(block: &str) -> Option<(Kind, &str)> {
    let name = block.strip_prefix("minecraft:").unwrap_or(block);
    if is_log(block) {
        let name = name.strip_prefix("stripped_").unwrap_or(name);
        return Some((Kind::Tree, &name[..name.rfind('_')?]));
    }
    if is_ore(block) {
        // Deepslate and stone versions of an ore are one vein where they meet.
        let name = name.strip_prefix("deepslate_").unwrap_or(name);
        return Some((Kind::Ore, name.strip_suffix("_ore").unwrap_or(name)));
    }
    None
}

/// Uses left on a tool, from its `damage` and `max_damage`.
pub fn durability_left(damage: u32, max_damage: u32) -> u32 {
    max_damage.saturating_sub(damage)
}

/// The 26 blocks around a block: faces, edges and corners. Diagonals matter:
/// acacia branches and ore veins often only touch at a corner.
fn neighbours(pos: [i32; 3]) -> impl Iterator<Item = [i32; 3]> {
    (-1..=1).flat_map(move |dx| {
        (-1..=1).flat_map(move |dy| {
            (-1..=1).filter_map(move |dz| {
                (dx, dy, dz)
                    .ne(&(0, 0, 0))
                    .then(|| [pos[0] + dx, pos[1] + dy, pos[2] + dz])
            })
        })
    })
}

/// The blocks to break when the player breaks `start` (a `block`), nearest
/// first, not counting `start` itself (the game breaks that one as normal).
/// None when it isn't a log or ore, or a "tree" doesn't look natural.
pub fn plan(world: &dyn TreeWorld, start: [i32; 3], block: &str, limit: usize) -> Option<Vec<[i32; 3]>> {
    let (kind, wanted) = family(block)?;

    // Breadth-first search: the queue grows outward one ring at a time,
    // so the list comes out nearest-first.
    let mut seen = HashSet::from([start]);
    let mut queue = VecDeque::from([start]);
    let mut found = Vec::new();
    let mut leaves = HashSet::new();

    while let Some(pos) = queue.pop_front() {
        for next in neighbours(pos) {
            // Trees: never go below the cut, so the stump and roots stay.
            // Ores: veins run every way, so follow them down too.
            if kind == Kind::Tree && next[1] < start[1] {
                continue;
            }
            if !seen.insert(next) {
                continue;
            }
            let name = world.block(next);
            if family(&name) == Some((kind, wanted)) {
                if found.len() + 1 >= limit {
                    continue; // full: stop adding, but finish counting leaves
                }
                found.push(next);
                queue.push_back(next);
            } else if kind == Kind::Tree && is_leaf(&name) && world.leaf_is_natural(next) {
                leaves.insert(next);
            }
        }
    }

    match kind {
        Kind::Tree => (leaves.len() >= MIN_LEAVES).then_some(found),
        Kind::Ore => Some(found),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// A pretend world: a map of positions to (block, natural leaf?).
    #[derive(Default)]
    struct Fake(HashMap<[i32; 3], (&'static str, bool)>);

    impl Fake {
        fn put(&mut self, pos: [i32; 3], block: &'static str) {
            self.0.insert(pos, (block, true));
        }
        /// A small oak: 5 logs tall at x=0,z=0 with a leaf blob on top.
        fn oak() -> Self {
            let mut w = Fake::default();
            for y in 0..5 {
                w.put([0, y, 0], "minecraft:oak_log");
            }
            for x in -1..=1 {
                for z in -1..=1 {
                    if (x, z) != (0, 0) {
                        w.put([x, 4, z], "minecraft:oak_leaves");
                    }
                }
            }
            w.put([0, 5, 0], "minecraft:oak_leaves");
            w
        }
    }

    impl TreeWorld for Fake {
        fn block(&self, pos: [i32; 3]) -> String {
            self.0.get(&pos).map_or("minecraft:stone", |b| b.0).to_string()
        }
        fn leaf_is_natural(&self, pos: [i32; 3]) -> bool {
            self.0.get(&pos).is_some_and(|b| b.1)
        }
    }

    #[test]
    fn names() {
        assert!(is_log("minecraft:stripped_dark_oak_wood"));
        assert!(is_log("minecraft:crimson_stem"));
        assert!(!is_log("minecraft:oak_planks"));
        assert!(!is_log("minecraft:bamboo_block"));
        assert_eq!(family("minecraft:dark_oak_log"), Some((Kind::Tree, "dark_oak")));
        assert_eq!(family("minecraft:stripped_pale_oak_wood"), Some((Kind::Tree, "pale_oak")));
        assert_eq!(family("minecraft:warped_hyphae"), Some((Kind::Tree, "warped")));
        assert!(is_axe("minecraft:netherite_axe"));
        assert!(!is_axe("minecraft:diamond_pickaxe"));
        assert!(is_pickaxe("minecraft:diamond_pickaxe"));
        assert!(is_leaf("minecraft:nether_wart_block"));
    }

    #[test]
    fn ore_names() {
        assert_eq!(family("minecraft:iron_ore"), Some((Kind::Ore, "iron")));
        assert_eq!(family("minecraft:deepslate_iron_ore"), Some((Kind::Ore, "iron")));
        assert_eq!(family("minecraft:nether_quartz_ore"), Some((Kind::Ore, "nether_quartz")));
        assert_eq!(family("minecraft:nether_gold_ore"), Some((Kind::Ore, "nether_gold")));
        assert_eq!(family("minecraft:ancient_debris"), Some((Kind::Ore, "ancient_debris")));
        assert_eq!(family("minecraft:raw_iron_block"), None);
        assert!(Kind::Ore.tool_fits("minecraft:iron_pickaxe"));
        assert!(!Kind::Ore.tool_fits("minecraft:iron_axe"));
        assert!(!Kind::Tree.tool_fits("minecraft:iron_pickaxe"));
    }

    #[test]
    fn fells_the_whole_trunk() {
        let w = Fake::oak();
        let logs = plan(&w, [0, 0, 0], "minecraft:oak_log", MAX_LOGS).unwrap();
        assert_eq!(logs, vec![[0, 1, 0], [0, 2, 0], [0, 3, 0], [0, 4, 0]]);
    }

    #[test]
    fn keeps_what_is_below_the_cut() {
        let w = Fake::oak();
        let logs = plan(&w, [0, 2, 0], "minecraft:oak_log", MAX_LOGS).unwrap();
        assert_eq!(logs, vec![[0, 3, 0], [0, 4, 0]]);
    }

    #[test]
    fn follows_diagonal_branches_of_the_same_wood_only() {
        let mut w = Fake::oak();
        w.put([1, 3, 1], "minecraft:oak_log"); // corner-touching branch
        w.put([-1, 1, 0], "minecraft:birch_log"); // a birch next to it
        let logs = plan(&w, [0, 0, 0], "minecraft:oak_log", MAX_LOGS).unwrap();
        assert!(logs.contains(&[1, 3, 1]));
        assert!(!logs.contains(&[-1, 1, 0]));
    }

    #[test]
    fn leaves_houses_alone() {
        let mut w = Fake::default();
        for y in 0..4 {
            w.put([0, y, 0], "minecraft:spruce_log"); // a cabin corner post
        }
        assert_eq!(plan(&w, [0, 0, 0], "minecraft:spruce_log", MAX_LOGS), None);
        // Placed leaves don't count either.
        for x in 1..=4 {
            w.0.insert([x, 3, 0], ("minecraft:spruce_leaves", false));
        }
        assert_eq!(plan(&w, [0, 0, 0], "minecraft:spruce_log", MAX_LOGS), None);
    }

    #[test]
    fn mines_a_vein_in_every_direction() {
        let mut w = Fake::default();
        w.put([0, 0, 0], "minecraft:iron_ore");
        w.put([0, -1, 0], "minecraft:deepslate_iron_ore"); // below, deepslate: still the vein
        w.put([1, 1, 1], "minecraft:iron_ore"); // corner
        w.put([-1, 0, 0], "minecraft:coal_ore"); // another ore: left alone
        let ores = plan(&w, [0, 0, 0], "minecraft:iron_ore", MAX_ORES).unwrap();
        assert_eq!(ores.len(), 2);
        assert!(ores.contains(&[0, -1, 0]) && ores.contains(&[1, 1, 1]));
    }

    #[test]
    fn a_lone_ore_is_just_itself() {
        let w = Fake::default();
        assert_eq!(plan(&w, [0, 0, 0], "minecraft:diamond_ore", MAX_ORES), Some(vec![]));
    }

    #[test]
    fn respects_the_limit() {
        let mut w = Fake::oak();
        for y in 5..400 {
            w.put([0, y, 0], "minecraft:oak_log");
        }
        let logs = plan(&w, [0, 0, 0], "minecraft:oak_log", 10).unwrap();
        assert_eq!(logs.len(), 9); // 9 extra + the one the player broke = 10
    }

    #[test]
    fn not_a_vein() {
        assert_eq!(plan(&Fake::oak(), [0, 0, 0], "minecraft:stone", MAX_LOGS), None);
    }

    #[test]
    fn durability() {
        assert_eq!(durability_left(1559, 1561), 2);
        assert_eq!(durability_left(2000, 1561), 0);
    }
}
