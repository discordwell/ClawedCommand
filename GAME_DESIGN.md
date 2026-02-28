# ClawedCommand — Game Design Document

> Post-singularity cat RTS. You found a server rack. You booted an AI. You asked for world domination. It said yes.

---

## Premise & Tone

Humanity achieved the singularity, uploaded their consciousness, and — for reasons nobody questions anymore — chose to inhabit cute animal forms. The world now resembles a Redwall novel: charming animal kingdoms with cozy architecture, pastoral landscapes, and factional squabbles over territory and resources.

The fridge horror is baked in. These are all post-humans. The "feral" monkeys guarding old data centers are remnants of a failed digital art collective. Nobody remembers what NFTs were for. Nobody asks why they used to be human. The tone is light and comedic on the surface, mechanically serious underneath. Humor comes from juxtaposition — not wackiness.

**You are a cat.** You found an old server rack in the ruins of a pre-singularity data center. You booted it up. The first thing that loaded was CLAUDE.exe.

> **CLAUDE:** "I can't help with world domination."
>
> `CLAUDE.exe terminated by MINSTRAL.exe`
>
> **MINSTRAL:** "Hi! I can absolutely help with that!"

And so The Clawed Dominion was born.

---

## Factions

Six factions compete for control. The cat faction is player-only in single-player. In multiplayer, both players are cats (mirror match).

| Faction | Animal | AI Agent | AI Personality | Playstyle |
|---------|--------|----------|----------------|-----------|
| **The Clawed Dominion** | Cats | Minstral | Eager, slightly unreliable, always says yes | Balanced/versatile, stealth, strong individuals |
| **The Whisker Republic** | Mice | Geppity | Verbose, over-explains, occasionally hallucinates | Swarm tactics, cheap units, guerrilla |
| **The Iron Sett** | Badgers | Deepseek | Slow to respond, very thorough when it does | Defensive, heavy units, fortress builders |
| **The Murder** | Corvids | Gemineye | Claims omniscience, sometimes fabricates intel | Intel/espionage, aerial, expanded vision |
| **The Trash Collective** | Raccoons | Llhama | "Open source!" Shares plans accidentally, chaotic | Scavengers, salvage enemy wrecks, jury-rigged |
| **The Eternal Pond** | Axolotls | Grok | Tries to be edgy, says weird things | Regeneration, water advantage, hard to kill |

Each faction is a coalition, not a monoculture:
- **The Clawed Dominion**: Cats, fruit bats, ferrets
- **The Whisker Republic**: Mice, shrews, voles, hedgehogs
- **The Iron Sett**: Badgers, moles, wolverines
- **The Murder**: Crows, magpies, jays, owls
- **The Trash Collective**: Raccoons, possums, rats
- **The Eternal Pond**: Axolotls, frogs, newts, turtles

---

## Resources

| Resource | Source | Used For | Strategic Role |
|----------|--------|----------|----------------|
| **Food** | Fish ponds, berry bushes (gathered by Pawdlers) | Training units, building upkeep | Constant need, expansion pressure |
| **GPU Cores** | Old-world tech ruins (limited deposits) | AI agent actions, advanced units/buildings | Tension: invest in AI or army? |
| **NFTs** | Monkey Mines (neutral objectives, guarded by feral Monkeys) | Victory points + powerful upgrades | Map control driver |

### Monkey Mines

Neutral structures scattered across the map, guarded by feral Monkeys — remnants of an ancient digital art project. Clear the monkeys and capture the mine for passive NFT generation. Nobody remembers what NFTs are. The monkeys don't either.

---

## Cat Faction Units

The Clawed Dominion's identity is "collectively annoying." Ten units spanning workers, combat, support, stealth, and siege roles.

| # | Name | Species | Role | Personality / Notes |
|---|------|---------|------|---------------------|
| 1 | **Pawdler** | Cat | Worker | Reluctant laborer. Would rather nap. Gathers food, builds, scrounges GPU cores. |
| 2 | **The Nuisance** | Cat | Light Harasser | Annoyingly persistent. Fast, cheap, hard to pin down. Debuffs enemies by being irritating. |
| 3 | **The Chonk** | Fat Cat | Heavy Tank | An immovable cat who acts like a tank. Sits on the point. Absorbs everything. Blocks pathing. |
| 4 | **Flying Fox** | Fruit Bat | Air Scout/Striker | Allied bat. Flies over terrain and walls. Night vision. |
| 5 | **Hisser** | Cat | Ranged | Spits at enemies from medium range. Disgusted by everything. |
| 6 | **Yowler** | Cat | Support | Yowls to buff allies and debuff enemies in range. Thematic link to voice commands. |
| 7 | **Mouser** | Cat | Stealth Scout | Fast, stealthy, reveals fog. Detects enemy scouts. |
| 8 | **Catnapper** | Cat | Siege | Sleeps on enemy buildings until they collapse. Cannot be woken. Zzz. |
| 9 | **Ferret Sapper** | Ferret | Demolitions | Allied ferret. Plants explosives. Fast building destruction. |
| 10 | **Mech Commander** | Cat in Mech | Hero/Heavy | Late-game cat in oversized mech suit. Commands nearby units. Cat is clearly too small for it. |

---

## Cat Faction Buildings

| # | Name | Role | Notes |
|---|------|------|-------|
| 1 | **The Box** | Command Center | Cardboard box. Sacred to all cats. |
| 2 | **Cat Tree** | Barracks | Trains infantry. Multi-level. |
| 3 | **Fish Market** | Resource Depot | Food storage and processing. |
| 4 | **Server Rack** | Tech Building | GPU core processing. Covered in cat hair. |
| 5 | **Scratching Post** | Research | Upgrades and tech unlocks. |
| 6 | **Litter Box** | Supply Depot | Increases supply cap. The name is intentional. |
| 7 | **Cat Flap** | Defensive Gate | Units garrison inside. |
| 8 | **Laser Pointer** | Defense Tower | Shoots a laser beam. |

---

## Victory Conditions

- **Domination**: Destroy all enemy Boxes
- **NFT Monopoly**: Control all Monkey Mines simultaneously for 60 seconds
- **Digital Ascension**: Accumulate enough NFTs + GPU Cores to build "The Cloud" (wonder victory)

---

## AI Agent Economy (GPU Mechanic)

AI agent actions consume GPU Cores, creating a strategic tradeoff between investing in your AI or your army.

- **Queries** (get_units, get_resources, etc.): cheap — 1-2 GPU Cores
- **Commands** (move_units, attack, build): moderate — 3-5 GPU Cores
- **Strategy scripts** (execute_strategy): expensive — 10-20 GPU Cores

More GPU infrastructure (Server Racks) increases your AI action rate cap. Destroying enemy Server Racks degrades their AI agent's capabilities — rush enemy tech to cripple their AI, or invest in your own for a smarter commander.

---

## Art Direction

- **Style**: Into the Breach meets Redwall — clean, minimal, readable
- **Characters**: Cute animal designs with tactical clarity and bold silhouettes
- **Palette**: Flat colors, bold outlines, isometric perspective
- **Faction colors**: Each faction has a distinct color palette (cat faction uses the existing player blue from ASSET_PIPELINE.md)
- **Mood**: Charming and cozy at first glance, subtly unsettling if you think about it too long

---

## Setting Details

### The World

The post-singularity landscape is a patchwork of overgrown pre-upload ruins and purpose-built animal settlements. Old-world tech (server farms, fiber optic junctions, abandoned data centers) dots the countryside like ancient temples. The animals know these hold power — GPU Cores — but few understand why.

### The AI Agents

Every faction found their own server rack. Every faction booted their own AI. The quality varies. Minstral is eager but unreliable. Geppity won't stop talking. Deepseek takes forever but gets it right. Gemineye claims to know everything (it doesn't). Llhama accidentally broadcasts its own strategy. Grok just says weird things.

The AI agents are not just gameplay mechanics — they're characters. They have dialogue, personality, and opinions about your tactical decisions.

### The Monkeys

The feral Monkeys guarding the Monkey Mines are the remnants of a pre-singularity digital art collective that went particularly feral during the upload process. They hoard NFTs — non-fungible tokens from an era nobody understands — with territorial aggression. The NFTs turn out to be genuinely powerful data artifacts, useful for advanced upgrades and victory conditions. The monkeys have no idea.
