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

The Clawed Dominion's identity is "collectively annoying." Ten units with abilities designed around a core principle: **a human can use any of these, but Minstral can use them all at once.** The AI agent turns manageable complexity into overwhelming coordination.

### General Ability Rules

- **Crowd control stacking**: A unit can only be affected by one CC effect at a time. If a new CC hits a unit already CC'd, the longer-remaining duration takes priority. CC types: *Tilted* (forced targeting), *Drowsed* (frozen), *Disoriented* (random commands). After any CC expires, the unit is CC-immune for 1s.
- **Override interactions**: Override cannot target a unit in a toggle state (Loaf Mode, Lullaby). The target must be in its default state. Override cannot target a unit mid-teleport (in a tunnel). Override cancels automatically if the target dies, and the Mech Commander takes 3s to recover (not the usual 15s cooldown) in that case.
- **Copycat limits**: Copycat only copies explicitly *activated* abilities (not passives, not toggles, not Override). For non-damage abilities, "40% effectiveness" means 40% duration. Copycat cannot copy Copycat.
- **Aura stacking**: Auras of the same type from the same unit type stack with diminishing returns: 2nd instance = 75% effectiveness, 3rd = 50%, 4th+ = 25%. Different aura types stack fully.
- **GPU ability costs**: Abilities that involve Minstral (Misinformation, Override) cost GPU Cores and benefit from Minstral Uplink's 50% discount. Pure unit abilities (Zoomies, Loaf Mode, etc.) are free.

---

### 1. Pawdler — Worker (Cat)

*Reluctant laborer. Would rather nap. Gathers food, builds, scrounges GPU cores.*

| Ability | Description |
|---------|-------------|
| **Spite Carry** | Pawdlers have a hidden *Motivation* meter that decays over time and drops faster near other idle Pawdlers (they enable each other's laziness). Witnessing a nearby ally die spikes Motivation to max, granting 50% faster gather speed for 15s. Being near combat units provides steady Motivation pressure. |
| **Opportunistic Hoarder** | Can carry two resource types simultaneously at the cost of 30% move speed per extra type. Returns everything at the nearest Fish Market in one trip. |
| **Power Nap** | An idle Pawdler near a Server Rack generates 0.5 GPU Cores/s (they nap on the warm hardware). Stacks diminishingly — 2nd Pawdler = 0.3/s, 3rd = 0.15/s, 4th+ = 0.05/s. A command costs 3-5 GPU, so one napper funds a command every 6-10s. |

**AI synergy**: Minstral optimizes Pawdler spacing to prevent Motivation decay, routes them through skirmish zones to proc Spite Carry, calculates multi-resource Hoarder routes that a human wouldn't bother planning, and rotates nappers on Server Racks to maximize diminishing returns.

---

### 2. The Nuisance — Light Harasser (Cat)

*Annoyingly persistent. Fast, cheap, hard to pin down.*

| Ability | Description |
|---------|-------------|
| **Annoyance Stacks** | Each attack applies a stack of *Annoyed* to the target (max 5). At 3 stacks: -15% attack speed. At 5 stacks: target is *Tilted* — attacks the nearest unit regardless of orders for 3s. Different Nuisances contribute to the same stack. |
| **Zoomies** | Activatable 2s burst: invulnerable, 3x speed, can't attack. Leaves a *Chaos Trail* that slows enemies by 40% for 4s. 12s cooldown. |
| **Copycat** | Passively mirrors the last activated ability of the nearest allied unit, at 40% effectiveness (40% duration for non-damage effects). Only copies explicitly activated abilities — not passives, toggles, or Override. Cannot copy Copycat. Has its own independent cooldown equal to the copied ability's cooldown ×1.5. |

**AI synergy**: Minstral focus-fires Nuisances to stack *Annoyed* on high-value targets, times Zoomies to dodge lethal burst and lay slow trails across chokepoints, and positions Nuisances near specific allies to copy the right abilities. A human manages one Nuisance well. The AI manages six.

---

### 3. The Chonk — Heavy Tank (Fat Cat)

*Immovable. Sits on the point. Absorbs everything. Unbothered.*

| Ability | Description |
|---------|-------------|
| **Gravitational Chonk** | Enemies within 3 tiles are slowly pulled toward the Chonk (0.3 tiles/s). Stacks with multiple Chonks. Allies are unaffected. |
| **Loaf Mode** | Toggle: the Chonk sits down completely. Gains 60% damage reduction and blocks all pathing (friend and foe), but cannot move or attack. Toggling off has a 2s stand-up animation during which the Chonk is vulnerable. |
| **Hairball** | Every 20s, coughs up a hairball that becomes a 1-tile terrain obstacle for 10s. The Chonk cannot control where — it lands in a random adjacent tile. |

**AI synergy**: Minstral positions Chonks to create overlapping gravity wells that funnel enemies into kill zones. It manages Loaf Mode toggling across multiple chokepoints — loafing to block a rush, un-loafing to let allies through, re-loafing before the next wave. It predicts Hairball timing and positions other units to exploit the random obstacles.

---

### 4. Flying Fox — Air Scout/Striker (Fruit Bat)

*Allied bat. Flies over terrain and walls. Sees in the dark.*

| Ability | Description |
|---------|-------------|
| **Echolocation Pulse** | Active: reveals all units in a huge radius (including stealthed) for 2s, but also reveals the Flying Fox to all enemies on the map. 30s cooldown. |
| **Fruit Drop** | Can carry one berry bush's worth of Food. Drop it on allied units to heal them (AoE, heals over 5s). Drop it on enemies to briefly slow them (sticky fruit). The Flying Fox must fly to a berry bush to reload. |
| **Thermal Riding** | Gains +50% speed and +2 vision range when flying over tiles that have been hit by any explosion (Shaped Charges, Booby Traps, building destruction) in the last 10s. The thermals from combat lift it higher. |

**AI synergy**: Minstral times Echolocation Pulses to reveal incoming attacks right before they hit, coordinates Fruit Drop resupply runs across the entire front line (which human would never micro), and routes Flying Foxes through Ferret Sapper explosion zones to proc Thermal Riding. The scout→artillery→air support pipeline is complex but the AI runs it automatically.

---

### 5. Hisser — Ranged (Cat)

*Spits at enemies from medium range. Disgusted by everything.*

| Ability | Description |
|---------|-------------|
| **Corrosive Spit** | Primary attack applies *Corroded* debuff: -5% armor per stack, max 6 stacks (= -30% armor). Stacks decay one at a time every 8s. Multiple Hissers stacking the same target shreds armor fast. |
| **Disgust Mortar** | Toggle: switches to indirect fire mode. Longer range, area damage, but requires *another unit to spot* the target (a unit with vision of the target tile). Can't self-spot. 1s delay between firing and impact. |
| **Revulsion** | When attacked in melee, the Hisser reflexively spits in all directions (AoE), applying 2 *Corroded* stacks to all adjacent enemies and gaining a 1.5s speed burst to retreat. 8s cooldown, triggered automatically. |

**AI synergy**: Minstral coordinates Hisser focus fire to shred armor on priority targets before the Chonk's gravity pulls them into melee. It manages the spotter network for Disgust Mortar — Mouser spots, Flying Fox spots, even Nuisances on Zoomies can spot as they run through. A human uses Hissers as simple ranged DPS. The AI uses them as a coordinated artillery battery.

---

### 6. Yowler — Support (Cat)

*Yowls to buff allies and debuff enemies in range. Thematic link to voice commands.*

| Ability | Description |
|---------|-------------|
| **Harmonic Resonance** | Passive aura: +10% damage and +10% move speed to allies within 4 tiles. If 2+ Yowlers are in range of each other, their auras amplify: 2 Yowlers = +20% each, 3 = +35% each. But each Yowler in the network takes +15% damage *per Yowler in the network* (the noise draws attention). At 3 Yowlers: +35% team buff, but each Yowler takes +45% damage. Glass cannon support. |
| **Dissonant Screech** | Active: 3s channel that applies *Disoriented* to all enemies in range — 25% chance each second that their next command is randomly redirected. 20s cooldown. If an enemy support unit (any faction's equivalent) is in range, their buff auras are suppressed for the duration. |
| **Lullaby** | Active: switches the Yowler's aura to *Soothing*. Enemies in range have -30% attack speed but allies in range also have -15% move speed (too relaxed). Lasts until toggled off. Cannot be active simultaneously with Harmonic Resonance — it's one or the other. |

**AI synergy**: Minstral manages Yowler positioning to create Resonance networks with optimal spacing (close enough to amplify, spread enough to avoid AoE wipes). It toggles between Resonance and Lullaby based on whether allies are advancing (need speed) or holding (need enemy debuff). It times Dissonant Screech to hit right as the enemy issues attack commands, maximizing disruption. Yowler micro is the highest-APM support play in the game — perfect for AI.

---

### 7. Mouser — Stealth Scout (Cat)

*Fast, stealthy, reveals fog. The eyes and ears of the Dominion.*

| Ability | Description |
|---------|-------------|
| **Dead Drop** | Plants an invisible sensor ward on a tile. Enemies passing through are *Tagged* — visible through fog of war for 30s, even if stealthed. Max 5 wards active. Wards last 90s or until detected by enemy scouts. |
| **Shadow Network** | Passive: if 2+ Mousers are within 8 tiles of each other, all friendly units within 2 tiles of the line between them are stealthed (a corridor, not a convex hull — cheap to compute, predictable for players). Breaking the network (moving a Mouser out of range) decloaks everyone with a 2s delay. 3+ Mousers create corridors between each pair. |
| **Misinformation** | Active (3 GPU Cores): creates a fake unit blip on the enemy's minimap at a target location. The blip moves convincingly for 10s. 25s cooldown per Mouser. Benefits from Minstral Uplink discount. Implemented as a real entity with a per-player visibility flag — compatible with lockstep determinism. |

**AI synergy**: Minstral places Dead Drops based on predicted enemy pathing (using map geometry and game state), maintains Shadow Networks by calculating convex hulls in real-time as units move (impossible for a human to do manually), and coordinates Misinformation blips to sell fake attacks — sending 3 fake signals toward the enemy's natural expansion while the real army tunnels through a Ferret Sapper network. Information warfare is the AI's domain.

---

### 8. Catnapper — Siege (Cat)

*Sleeps on enemy buildings until they collapse. Cannot be woken. Zzz.*

| Ability | Description |
|---------|-------------|
| **Dream Siege** | The Catnapper's siege damage ramps over time: 1x at 0-5s, 2x at 5-15s, 4x at 15-30s, 8x at 30s+. Any damage to the Catnapper resets the timer. It doesn't wake up — it just gets *less comfortable* and the damage rate resets. |
| **Contagious Yawning** | Passive aura: enemy units within 3 tiles of a sleeping Catnapper have a 10% chance per second to *Drowse* — they freeze for 0.5s. Multiple Catnappers increase the chance (diminishing per aura stacking rules), but each unit has a 2s Drowse immunity window after being Drowsed. |
| **Nine Lives** | The first time a Catnapper would die, it instead falls asleep for 5s (invulnerable), then wakes up at 30% HP. This can only trigger once per Catnapper. If it's already on an enemy building when Nine Lives triggers, the sleep timer counts toward Dream Siege ramp. |

**AI synergy**: Minstral manages Catnapper protection — keeping Chonks and Yowlers positioned to prevent damage that would reset Dream Siege timers. It calculates when a Catnapper is about to hit the 8x damage threshold and prioritizes protecting *that specific* Catnapper. It uses Contagious Yawning positioning to Drowse defenders while the siege ramps. The AI turns a "cute sleeping cat" into an unstoppable siege engine by solving the protection puzzle.

---

### 9. Ferret Sapper — Demolitions (Ferret)

*Allied ferret. Plants explosives. Excited about it.*

| Ability | Description |
|---------|-------------|
| **Tunnel Network** | Digs a tunnel entrance at current position. A second activation elsewhere digs the exit. Units entering one end teleport to the other after 1.5s. Max 3 tunnel pairs active. Tunnels are invisible to enemies but detectable by scout units. Destroyed if either end takes damage. |
| **Shaped Charge** | Plants an explosive with a variable fuse (1-10s, set at placement). Deals massive damage in a 2-tile radius. Can be placed on buildings for 3x bonus damage. Max 3 charges active. Multiple charges detonating within 1s of each other deal +25% bonus damage each (sympathetic detonation). |
| **Booby Trap** | Can rig an enemy building (must be adjacent for 3s). The next time that building produces a unit or a unit garrisons inside, the trap detonates, dealing heavy damage to the building and all units inside. The trap is invisible until triggered. |

**AI synergy**: Minstral manages the tunnel network as a logistics system — routing reinforcements to the front, evacuating wounded units to Fruit Drop healing zones, and creating flanking paths the enemy can't see. It coordinates Shaped Charge fuse timers across multiple Sappers to create simultaneous detonations for the sympathetic bonus. It identifies which enemy buildings are about to produce units and targets those for Booby Traps. The Ferret Sapper is a toolkit — the AI is the engineer.

---

### 10. Mech Commander — Hero/Heavy (Cat in Mech)

*Late-game cat in oversized mech suit. Cat is clearly too small for it.*

| Ability | Description |
|---------|-------------|
| **Tactical Uplink** | Passive aura (8-tile radius): all friendly units in range have their ability cooldowns reduced by 20% and share vision with each other. The Mech Commander can see everything its linked units see. |
| **Override** | Active (8 GPU Cores): takes direct control of a target friendly unit in default state (not toggled/teleporting) anywhere on the map. That unit gets +40% to all stats, but the Mech Commander is paralyzed for the duration. Lasts until cancelled or the overridden unit dies (3s recovery on death, 15s cooldown on cancel). Benefits from Minstral Uplink discount. |
| **Minstral Uplink** | Passive: AI agent commands issued to units within the Tactical Uplink aura cost 50% fewer GPU Cores. Creates a strategic incentive to keep your army near the Mech Commander and invest in AI actions. |

**AI synergy**: The Mech Commander is the AI's physical avatar on the battlefield. Minstral Uplink makes the AI literally cheaper to use near it, creating a mobile "command zone." The AI uses Override to temporarily supercharge a critical unit at the right moment — overriding a Catnapper to protect its Dream Siege ramp, or overriding a Ferret Sapper to nail a precise tunnel placement. Tactical Uplink's cooldown reduction means ability-heavy compositions (Yowler networks, Nuisance swarms) become significantly more powerful near the Mech Commander. The AI manages the Override target selection — knowing exactly when to sacrifice Mech Commander mobility for a surgically enhanced unit elsewhere.

---

### Unit Synergy Map

The units are designed as interlocking systems. A human can use any unit effectively in isolation. Minstral turns them into a machine.

```
INFORMATION LAYER
  Mouser (Dead Drops, Shadow Network) ──► spots for ──► Hisser (Disgust Mortar)
  Flying Fox (Echolocation) ──► reveals targets for ──► everyone
  Mouser (Misinformation) + Ferret (Tunnels) = fake attack + real flank

CONTROL LAYER
  Chonk (Gravity + Loaf) ──► funnels enemies into ──► Hisser (Corrosive Spit stacks)
  Yowler (Lullaby) ──► slows enemies near ──► Catnapper (Contagious Yawning)
  Nuisance (Annoyance → Tilted) ──► forces bad trades into ──► Chonk gravity wells

DAMAGE LAYER
  Hisser (Corrosion) ──► shreds armor for ──► Nuisance / Mech Commander
  Catnapper (Dream Siege ramp) ──► protected by ──► Chonk (Loaf) + Yowler (Resonance)
  Ferret (Shaped Charge sync) ──► thermal updrafts for ──► Flying Fox (Thermal Riding)

SUPPORT LAYER
  Yowler (Dissonant Screech) ──► faster cycling via ──► Mech Commander (Tactical Uplink -20% CD)
  Flying Fox (Fruit Drop) ──► heals ──► front-line units via Ferret tunnels
  Pawdler (Power Nap) ──► funds ──► Minstral actions (Misinformation, Override)
  Mech Commander (Minstral Uplink) ──► makes all AI coordination 50% cheaper nearby
```

**The design thesis**: at low skill/GPU investment, every unit works fine as a straightforward RTS unit. Click Hisser, right-click enemy, it spits. But with AI investment, the Hisser becomes part of a spotter-artillery network with armor-shred coordination and mortar fire directed through Mouser vision. The skill ceiling isn't micro speed — it's how well you coach your AI and how many GPU Cores you invest in letting it work.

### Implementation Notes

These abilities introduce systems beyond Phase 2's basic combat. Key architectural needs:

- **Aura system**: Spatial queries per tick (needs spatial hash map resource, not brute-force O(n^2))
- **Status effect system**: Generic `StatusEffect` component handling stacks, durations, CC immunity windows
- **Ability system**: Cooldowns, toggles, active abilities with `GameCommand::ActivateAbility` variant
- **Dynamic pathing blockers**: Loaf Mode and Hairball require a dynamic occupancy overlay on the pathfinding grid
- **Spotter network**: Disgust Mortar couples combat targeting to fog of war — target tile needs friendly vision
- **Tunnel portals**: Pathfinding needs zero-cost portal edges in the graph (grid + portals A*)
- **Misinformation**: Real entity with `VisibleOnlyTo { player_id }` component — extends fog of war, preserves lockstep
- **Override**: `ControlProxy { controller, target }` component pair — command dispatcher checks for proxies before routing

See PLAN.md for phasing. Abilities should ship incrementally, not all at once.

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
