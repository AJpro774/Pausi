# Pausi

**A fast, chaotic square-vs-square duel game.** Pick a character, build an ability loadout, customize your trail and your finishing moves, then fight on one keyboard, against a CPU, or sit back and watch two bots go at it.

Written in Rust with [raylib](https://www.raylib.com/). Everything (art, particles, scenery, cutscenes) is drawn in code, so there are no asset files.

## How this happened

This started as a small coding side project for school: a single orange square that could roll around and jump.

Then I got sidetracked, and for no reason whatsoever decided to turn it into a PvP game. Nobody asked for this. There was no plan, no deadline and no good excuse. The square got a friend, the friend got a knife, and it just kept going: abilities, ultimates, eight maps, victory cutscenes where the winner stabs a knife into the ground and fires every ability at once, and a mode where two CPUs fight each other so I can watch.

The school project is (probably) still due. The duel game, however, is excellent.

## Features

- **Momentum movement** with sliding, charged jumps, wall slides, wall jumps and double jumps.
- **6 characters**, each with a passive trait and a unique ultimate.
- **12 abilities**: pick any two for your loadout.
- **Block and parry**, a knife melee attack, stagger on heavy hits and a combo counter.
- **Ultimate meter** that fills as you fight and unleashes a cinematic super move.
- **8 themed maps**, each with its own scenery and a map event.
- **4 game modes**, best-of series (1 / 3 / 5 / 10 / 15) that end when one side clinches, and a running tally.
- **10 death animations and 10 victory cutscenes**, chosen per player, with a RANDOM option.
- **Customizable trails** (5 styles x 9 colors).
- **CPU opponent with 8 personalities**, plus a **CPU vs CPU** spectator mode.
- Pickups, screen shake, hit-stop and a slow-motion KO zoom.

## Running it

You need [Rust](https://www.rust-lang.org/tools/install), [CMake](https://cmake.org/download/) and a C compiler (Visual Studio Build Tools on Windows, `gcc` / `clang` on Linux and macOS). raylib is built from source the first time, which takes a minute.

```bash
cargo run --release
```

**Windows tip:** if the build fails with a "path exceeds the OS max path limit" error from MSBuild, point cargo at a short build folder:

```powershell
$env:CARGO_TARGET_DIR = "C:\t\pausi"
cargo run --release
```

## Controls

| | Player 1 | Player 2 |
|---|---|---|
| Move | `A` / `D` | `Left` / `Right` |
| Jump (press again in the air for a double jump; jump on a wall to wall jump) | `W` | `Up` |
| Hold to charge a bigger jump | `S` | `Down` |
| Ability 1 / Ability 2 | `F` / `G` | `,` (Comma) / `.` (Period) |
| Knife (short cooldown) | `E` | `/` (Slash) |
| Block (tap just as a hit lands to **parry**) | `Q` | `Right Shift` |
| Ultimate (when the meter is full) | `V` | `Right Ctrl` |

**Menus and rounds**

| Key | What it does |
|---|---|
| `W`/`S` or `Up`/`Down` | Move between rows on your panel |
| `A`/`D` or `Left`/`Right` | Change the selected value |
| `F` (P1) / `L` (P2) | Ready up |
| `M` | Change the starting map |
| `N` | Number of rounds (1, 3, 5, 10, 15) |
| `B` | Game mode |
| `C` | Players: human vs human, human vs CPU, CPU vs CPU |
| `1` / `2` | Change the CPU personality for player 1 / 2 |
| `Space` / `Enter` | Next round, or skip a victory cutscene |
| `R` | Back to the menu (also from the pause screen) |
| `P` / `Esc` | Pause during a fight. `Esc` on the menu quits |
| `H` | How to play |
| `F11` | Fullscreen |
| `O` | Mute sound effects |
| `[` / `]` | CPU vs CPU playback speed |
| `K` | CPU vs CPU: skip victory cutscenes |

A gamepad works alongside the keyboard. Pad 1 is player 1 and pad 2 is player 2: left stick or d-pad to move, A to jump, X and B for the two abilities (they have to be different), Y for the knife, L1 to block, R1 for the ultimate, Start to ready up.

The first round win of a series plays the victory cutscene. Later rounds just award the point. Shield blocks lava, burn, frost, and magnet pulls; a held block only chips those.

## Characters

| Character | Passive | Ultimate |
|---|---|---|
| **Dasher** | +10% speed | **Overdrive**: faster, cooldowns x3, stronger knife for 6s |
| **Bomber** | Bombs and fireballs hit 20% harder | **Carpet Bomb**: bombs rain across the arena |
| **Shielder** | Takes 15% less damage | **Aegis Nova**: 3s invincible plus a blast wave |
| **Titan** | 130 HP, resists knockback, slower, no double jump | **Earthquake**: 25 damage and a launch on the ground, still hits airborne foes |
| **Ghost** | 85 HP, floaty, two air jumps | **Deep Freeze**: freezes the opponent from anywhere |
| **Spark** | 90 HP, +25% speed, stronger knife | **Lightning Storm**: five bolts strike around the opponent |

Picking a character fills in a default loadout, but you can swap either ability slot for anything.

## Abilities

| Ability | Effect |
|---|---|
| **Dash** | Ram forward for 15 damage |
| **Slam** | Leap and crash down; shockwave for 20 damage |
| **Bomb** | Thrown bomb, 25 damage; the blast launches you too |
| **Hop** | Boost upward, even in the air |
| **Shield** | Blocks all damage and reflects projectiles |
| **Pulse** | Short-range blast, 12 damage with big knockback |
| **Blink** | Teleport forward |
| **Frost** | Ice shot that freezes the opponent for 1.3s |
| **Thunder** | Marks their spot, then lightning strikes for 22 damage |
| **Magnet** | Yanks the opponent toward you |
| **Mend** | Heal 18 HP over 2 seconds |
| **Fireball** | 14 damage and burns for 2.5s |

## Maps

Every round rotates to the next map. Each one has its own event to watch out for.

| Map | Event |
|---|---|
| Sunny Meadow | Wind gusts push everyone sideways |
| Night Skyline | Meteors fall |
| Volcano | Lava on the floor surges upward, plus meteors |
| Frozen Peaks | Blizzards |
| Deep Space | Zero-g bursts and falling comets |
| Deep Sea | Floaty gravity and shifting currents |
| Sand Ruins | Rocks fall from the sky |
| Neon Arcade | Sweeping laser beams (watch the red warning line) |

## Game modes

- **Classic**: last square standing wins the round.
- **Timed**: 60 second clock; a KO wins, otherwise the most HP wins.
- **Stock**: three lives per round, with respawns.
- **King of the Hill**: stand alone in the glowing zone to score; 30 points or the most after 90 seconds wins. Each KO gives the other player 5 points.

## CPU personalities

Balanced, Aggressor, Defender, Sniper, Trickster, Berserker, Rookie and Pro, plus Random (a new one each round). Set them with `1` / `2` on the select screen.

## Project layout

```
Cargo.toml     dependencies (just raylib)
src/main.rs    the whole game
```

Gameplay numbers (gravity, speeds, damage, cooldowns, durations) are constants near the top of `src/main.rs`, and each character, ability, map and CPU personality is a small table you can tweak.
