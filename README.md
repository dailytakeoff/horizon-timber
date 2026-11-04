# Timber for Horizon

A native Horizon mod (Minecraft 26.3 in Rust), MIT licensed. In survival:

- break one log with an **axe** and the whole tree comes down;
- break one ore with a **pickaxe** and the whole vein comes out.

## Trees

- Fells logs, wood, stems and hyphae (stripped too), including diagonal branches (acacia, big oak).
- Only the wood type you hit: an oak never takes the birch beside it.
- Only from where you cut upward: chop mid-trunk and the stump stays.
- Only natural trees: it needs at least 3 natural leaves touching the logs, so cabins are safe.

## Ores

- Every `*_ore` plus ancient debris. Deepslate and stone versions count as one vein (iron + deepslate iron).
- Follows the vein every direction, diagonals too, up to 64 blocks. Only the ore type you hit.
- Only when your pickaxe can actually drop it (no wiping a diamond vein with a stone pick).

## Both

- Each block is broken by Horizon's own player-mining code: normal drops, Fortune/Silk Touch, XP, stats,
  sound and tool wear.
- Stops with 1 durability left so your axe or pickaxe never breaks.
- Sneak to break a single block. Turn it off in Options > Mods.
- Install it on whoever hosts the world (single player, LAN host, or the dedicated server).

## Install

Launcher → Mods → install from file → `timber-0.2.0.hnmod` → Build → confirm the native-code warning.

## Develop

```sh
cargo test               # the tree and ore logic, no game needed
python3 tools/package.py # target/timber-0.2.0.hnmod
```

Files in `mixins/timber/`: `runtime.rs` (pure logic, tested), `glue.rs` (talks to the game),
`destroy_block.rs` (the hook), `horizon-mixin.toml` (wires them in).

## Releases

Every push builds and tests the mod; the `.hnmod` is under the Actions run's artifacts. Push a tag
(`git tag v0.2.0 && git push --tags`) to publish a GitHub release with the `.hnmod` attached.

Not an official Minecraft or Horizon product. Not approved by or associated with Mojang or Microsoft.
