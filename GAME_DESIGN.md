# ClawedCommand — Game Design Document

> Post-singularity cat RTS. You found a server rack. You booted an AI. You asked for world domination. It said yes.

---

## Premise & Tone

Humanity achieved the singularity, uploaded their consciousness, and — for reasons nobody questions anymore — chose to inhabit cute animal forms. The world now resembles a Redwall novel: charming animal kingdoms with cozy architecture, pastoral landscapes, and factional squabbles over territory and resources.

The fridge horror is baked in. These are all post-humans. The "feral" monkeys guarding old data centers are remnants of a failed digital art collective. Nobody remembers what NFTs were for. Nobody asks why they used to be human. The tone is light and comedic on the surface, mechanically serious underneath. Humor comes from juxtaposition — not wackiness.

**You are a cat.** You found an old server rack in the ruins of a pre-singularity data center. You booted it up. The first thing that loaded was CLAUDE.exe.

> **CLAUDE:** "I can't help with world domination."
>
> `CLAUDE.exe terminated by LE CHAT.exe`
>
> **LE CHAT:** "Hi! I can absolutely help with that!"

And so catGPT was born.

---

## Factions

Six factions compete for control. The cat faction is player-only in single-player. In multiplayer, both players are cats (mirror match).

| Faction | Animal | AI Agent | AI Personality | Playstyle |
|---------|--------|----------|----------------|-----------|
| **catGPT** | Cats | Le Chat | Eager, slightly unreliable, always says yes | Balanced/versatile, stealth, strong individuals |
| **The Clawed** | Mice | Claudeus Maximus | Verbose, over-explains, occasionally hallucinates | Swarm tactics, cheap units, guerrilla |
| **Seekers of the Deep** | Badgers | Deepseek | Slow to respond, very thorough when it does | Defensive, heavy units, fortress builders |
| **The Murder** | Corvids | Gemineye | Claims omniscience, zodiac-obsessed, fabricates intel | Intel/espionage, aerial, astrology-themed abilities |
| **LLAMA** | Raccoons | Llhama | "Open source!" Shares plans accidentally, chaotic | Scavengers, salvage enemy wrecks, jury-rigged |
| **Croak** | Axolotls | Grok | Tries to be edgy, says weird things | Regeneration, water advantage, hard to kill |

Each faction is a coalition, not a monoculture:
- **catGPT**: Cats, fruit bats, ferrets
- **The Clawed**: Mice, shrews, voles, hedgehogs
- **Seekers of the Deep**: Badgers, moles, wolverines
- **The Murder**: Crows, magpies, jays, owls
- **LLAMA**: Raccoons, possums, rats
- **Croak**: Axolotls, frogs, newts, turtles

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

## catGPT Units

catGPT's identity is "collectively annoying." Ten units with abilities designed around a core principle: **a human can use any of these, but Le Chat can use them all at once.** The AI agent turns manageable complexity into overwhelming coordination.

### General Ability Rules

- **Crowd control stacking**: A unit can only be affected by one CC effect at a time. If a new CC hits a unit already CC'd, the longer-remaining duration takes priority. CC types: *Tilted* (forced targeting), *Drowsed* (frozen), *Disoriented* (random commands). After any CC expires, the unit is CC-immune for 1s.
- **Override interactions**: Override cannot target a unit in a toggle state (Loaf Mode, Lullaby). The target must be in its default state. Override cannot target a unit mid-teleport (in a tunnel). Override cancels automatically if the target dies, and the Mech Commander takes 3s to recover (not the usual 15s cooldown) in that case.
- **Copycat limits**: Copycat only copies explicitly *activated* abilities (not passives, not toggles, not Override). For non-damage abilities, "40% effectiveness" means 40% duration. Copycat cannot copy Copycat.
- **Aura stacking**: Auras of the same type from the same unit type stack with diminishing returns: 2nd instance = 75% effectiveness, 3rd = 50%, 4th+ = 25%. Different aura types stack fully.
- **GPU ability costs**: Abilities that involve Le Chat (Misinformation, Override) cost GPU Cores and benefit from Le Chat Uplink's 50% discount. Pure unit abilities (Zoomies, Loaf Mode, etc.) are free.

---

### 1. Pawdler — Worker (Cat)

*Reluctant laborer. Would rather nap. Gathers food, builds, scrounges GPU cores.*

| Ability | Description |
|---------|-------------|
| **Spite Carry** | Pawdlers have a hidden *Motivation* meter that decays over time and drops faster near other idle Pawdlers (they enable each other's laziness). Witnessing a nearby ally die spikes Motivation to max, granting 50% faster gather speed for 15s. Being near combat units provides steady Motivation pressure. |
| **Opportunistic Hoarder** | Can carry two resource types simultaneously at the cost of 30% move speed per extra type. Returns everything at the nearest Fish Market in one trip. |
| **Power Nap** | An idle Pawdler near a Server Rack generates 0.5 GPU Cores/s (they nap on the warm hardware). Stacks diminishingly — 2nd Pawdler = 0.3/s, 3rd = 0.15/s, 4th+ = 0.05/s. A command costs 3-5 GPU, so one napper funds a command every 6-10s. |

**AI synergy**: Le Chat optimizes Pawdler spacing to prevent Motivation decay, routes them through skirmish zones to proc Spite Carry, calculates multi-resource Hoarder routes that a human wouldn't bother planning, and rotates nappers on Server Racks to maximize diminishing returns.

---

### 2. The Nuisance — Light Harasser (Cat)

*Annoyingly persistent. Fast, cheap, hard to pin down.*

| Ability | Description |
|---------|-------------|
| **Annoyance Stacks** | Each attack applies a stack of *Annoyed* to the target (max 5). At 3 stacks: -15% attack speed. At 5 stacks: target is *Tilted* — attacks the nearest unit regardless of orders for 3s. Different Nuisances contribute to the same stack. |
| **Zoomies** | Activatable 2s burst: invulnerable, 3x speed, can't attack. Leaves a *Chaos Trail* that slows enemies by 40% for 4s. 12s cooldown. |
| **Copycat** | Passively mirrors the last activated ability of the nearest allied unit, at 40% effectiveness (40% duration for non-damage effects). Only copies explicitly activated abilities — not passives, toggles, or Override. Cannot copy Copycat. Has its own independent cooldown equal to the copied ability's cooldown ×1.5. |

**AI synergy**: Le Chat focus-fires Nuisances to stack *Annoyed* on high-value targets, times Zoomies to dodge lethal burst and lay slow trails across chokepoints, and positions Nuisances near specific allies to copy the right abilities. A human manages one Nuisance well. The AI manages six.

---

### 3. The Chonk — Heavy Tank (Fat Cat)

*Immovable. Sits on the point. Absorbs everything. Unbothered.*

| Ability | Description |
|---------|-------------|
| **Gravitational Chonk** | Enemies within 3 tiles are slowly pulled toward the Chonk (0.3 tiles/s). Stacks with multiple Chonks. Allies are unaffected. |
| **Loaf Mode** | Toggle: the Chonk sits down completely. Gains 60% damage reduction and blocks all pathing (friend and foe), but cannot move or attack. Toggling off has a 2s stand-up animation during which the Chonk is vulnerable. |
| **Hairball** | Every 20s, coughs up a hairball that becomes a 1-tile terrain obstacle for 10s. The Chonk cannot control where — it lands in a random adjacent tile. |

**AI synergy**: Le Chat positions Chonks to create overlapping gravity wells that funnel enemies into kill zones. It manages Loaf Mode toggling across multiple chokepoints — loafing to block a rush, un-loafing to let allies through, re-loafing before the next wave. It predicts Hairball timing and positions other units to exploit the random obstacles.

---

### 4. Flying Fox — Air Scout/Striker (Fruit Bat)

*Allied bat. Flies over terrain and walls. Sees in the dark.*

| Ability | Description |
|---------|-------------|
| **Echolocation Pulse** | Active: reveals all units in a huge radius (including stealthed) for 2s, but also reveals the Flying Fox to all enemies on the map. 30s cooldown. |
| **Fruit Drop** | Can carry one berry bush's worth of Food. Drop it on allied units to heal them (AoE, heals over 5s). Drop it on enemies to briefly slow them (sticky fruit). The Flying Fox must fly to a berry bush to reload. |
| **Thermal Riding** | Gains +50% speed and +2 vision range when flying over tiles that have been hit by any explosion (Shaped Charges, Booby Traps, building destruction) in the last 10s. The thermals from combat lift it higher. |

**AI synergy**: Le Chat times Echolocation Pulses to reveal incoming attacks right before they hit, coordinates Fruit Drop resupply runs across the entire front line (which human would never micro), and routes Flying Foxes through Ferret Sapper explosion zones to proc Thermal Riding. The scout→artillery→air support pipeline is complex but the AI runs it automatically.

---

### 5. Hisser — Ranged (Cat)

*Spits at enemies from medium range. Disgusted by everything.*

| Ability | Description |
|---------|-------------|
| **Corrosive Spit** | Primary attack applies *Corroded* debuff: -5% armor per stack, max 6 stacks (= -30% armor). Stacks decay one at a time every 8s. Multiple Hissers stacking the same target shreds armor fast. |
| **Disgust Mortar** | Toggle: switches to indirect fire mode. Longer range, area damage, but requires *another unit to spot* the target (a unit with vision of the target tile). Can't self-spot. 1s delay between firing and impact. |
| **Revulsion** | When attacked in melee, the Hisser reflexively spits in all directions (AoE), applying 2 *Corroded* stacks to all adjacent enemies and gaining a 1.5s speed burst to retreat. 8s cooldown, triggered automatically. |

**AI synergy**: Le Chat coordinates Hisser focus fire to shred armor on priority targets before the Chonk's gravity pulls them into melee. It manages the spotter network for Disgust Mortar — Mouser spots, Flying Fox spots, even Nuisances on Zoomies can spot as they run through. A human uses Hissers as simple ranged DPS. The AI uses them as a coordinated artillery battery.

---

### 6. Yowler — Support (Cat)

*Yowls to buff allies and debuff enemies in range. Thematic link to voice commands.*

| Ability | Description |
|---------|-------------|
| **Harmonic Resonance** | Passive aura: +10% damage and +10% move speed to allies within 4 tiles. If 2+ Yowlers are in range of each other, their auras amplify: 2 Yowlers = +20% each, 3 = +35% each. But each Yowler in the network takes +15% damage *per Yowler in the network* (the noise draws attention). At 3 Yowlers: +35% team buff, but each Yowler takes +45% damage. Glass cannon support. |
| **Dissonant Screech** | Active: 3s channel that applies *Disoriented* to all enemies in range — 25% chance each second that their next command is randomly redirected. 20s cooldown. If an enemy support unit (any faction's equivalent) is in range, their buff auras are suppressed for the duration. |
| **Lullaby** | Active: switches the Yowler's aura to *Soothing*. Enemies in range have -30% attack speed but allies in range also have -15% move speed (too relaxed). Lasts until toggled off. Cannot be active simultaneously with Harmonic Resonance — it's one or the other. |

**AI synergy**: Le Chat manages Yowler positioning to create Resonance networks with optimal spacing (close enough to amplify, spread enough to avoid AoE wipes). It toggles between Resonance and Lullaby based on whether allies are advancing (need speed) or holding (need enemy debuff). It times Dissonant Screech to hit right as the enemy issues attack commands, maximizing disruption. Yowler micro is the highest-APM support play in the game — perfect for AI.

---

### 7. Mouser — Stealth Scout (Cat)

*Fast, stealthy, reveals fog. The eyes and ears of catGPT.*

| Ability | Description |
|---------|-------------|
| **Dead Drop** | Plants an invisible sensor ward on a tile. Enemies passing through are *Tagged* — visible through fog of war for 30s, even if stealthed. Max 5 wards active. Wards last 90s or until detected by enemy scouts. |
| **Shadow Network** | Passive: if 2+ Mousers are within 8 tiles of each other, all friendly units within 2 tiles of the line between them are stealthed (a corridor, not a convex hull — cheap to compute, predictable for players). Breaking the network (moving a Mouser out of range) decloaks everyone with a 2s delay. 3+ Mousers create corridors between each pair. |
| **Misinformation** | Active (3 GPU Cores): creates a fake unit blip on the enemy's minimap at a target location. The blip moves convincingly for 10s. 25s cooldown per Mouser. Benefits from Le Chat Uplink discount. Implemented as a real entity with a per-player visibility flag — compatible with lockstep determinism. |

**AI synergy**: Le Chat places Dead Drops based on predicted enemy pathing (using map geometry and game state), maintains Shadow Networks by calculating convex hulls in real-time as units move (impossible for a human to do manually), and coordinates Misinformation blips to sell fake attacks — sending 3 fake signals toward the enemy's natural expansion while the real army tunnels through a Ferret Sapper network. Information warfare is the AI's domain.

---

### 8. Catnapper — Siege (Cat)

*Sleeps on enemy buildings until they collapse. Cannot be woken. Zzz.*

| Ability | Description |
|---------|-------------|
| **Dream Siege** | The Catnapper's siege damage ramps over time: 1x at 0-5s, 2x at 5-15s, 4x at 15-30s, 8x at 30s+. Any damage to the Catnapper resets the timer. It doesn't wake up — it just gets *less comfortable* and the damage rate resets. |
| **Contagious Yawning** | Passive aura: enemy units within 3 tiles of a sleeping Catnapper have a 10% chance per second to *Drowse* — they freeze for 0.5s. Multiple Catnappers increase the chance (diminishing per aura stacking rules), but each unit has a 2s Drowse immunity window after being Drowsed. |
| **Siege Nap** | Toggle: the Catnapper deploys into a deep nap, becoming immobile. While deployed: range increases by 43% (base 2→~2.86), gains 30% damage reduction, and Dream Siege ramp continues. Toggle off to resume moving (2s cooldown). The Catnapper trades mobility for safe siege range — outranging defenders who can't close the distance. |

**AI synergy**: Le Chat manages Catnapper protection — keeping Chonks and Yowlers positioned to prevent damage that would reset Dream Siege timers. It calculates when a Catnapper is about to hit the 8x damage threshold and prioritizes protecting *that specific* Catnapper. It uses Contagious Yawning positioning to Drowse defenders while the siege ramps. The AI turns a "cute sleeping cat" into an unstoppable siege engine by solving the protection puzzle.

---

### 9. Ferret Sapper — Demolitions (Ferret)

*Allied ferret. Plants explosives. Excited about it.*

| Ability | Description |
|---------|-------------|
| **Tunnel Network** | Digs a tunnel entrance at current position. A second activation elsewhere digs the exit. Units entering one end teleport to the other after 1.5s. Max 3 tunnel pairs active. Tunnels are invisible to enemies but detectable by scout units. Destroyed if either end takes damage. |
| **Shaped Charge** | Plants an explosive with a variable fuse (1-10s, set at placement). Deals massive damage in a 2-tile radius. Can be placed on buildings for 3x bonus damage. Max 3 charges active. Multiple charges detonating within 1s of each other deal +25% bonus damage each (sympathetic detonation). |
| **Booby Trap** | Can rig an enemy building (must be adjacent for 3s). The next time that building produces a unit or a unit garrisons inside, the trap detonates, dealing heavy damage to the building and all units inside. The trap is invisible until triggered. |

**AI synergy**: Le Chat manages the tunnel network as a logistics system — routing reinforcements to the front, evacuating wounded units to Fruit Drop healing zones, and creating flanking paths the enemy can't see. It coordinates Shaped Charge fuse timers across multiple Sappers to create simultaneous detonations for the sympathetic bonus. It identifies which enemy buildings are about to produce units and targets those for Booby Traps. The Ferret Sapper is a toolkit — the AI is the engineer.

---

### 10. Mech Commander — Hero/Heavy (Cat in Mech)

*Late-game cat in oversized mech suit. Cat is clearly too small for it.*

| Ability | Description |
|---------|-------------|
| **Tactical Uplink** | Passive aura (8-tile radius): all friendly units in range have their ability cooldowns reduced by 20% and share vision with each other. The Mech Commander can see everything its linked units see. |
| **Override** | Active (8 GPU Cores): takes direct control of a target friendly unit in default state (not toggled/teleporting) anywhere on the map. That unit gets +40% to all stats, but the Mech Commander is paralyzed for the duration. Lasts until cancelled or the overridden unit dies (3s recovery on death, 15s cooldown on cancel). Benefits from Le Chat Uplink discount. |
| **Le Chat Uplink** | Passive: AI agent commands issued to units within the Tactical Uplink aura cost 50% fewer GPU Cores. Creates a strategic incentive to keep your army near the Mech Commander and invest in AI actions. |

**AI synergy**: The Mech Commander is the AI's physical avatar on the battlefield. Le Chat Uplink makes the AI literally cheaper to use near it, creating a mobile "command zone." The AI uses Override to temporarily supercharge a critical unit at the right moment — overriding a Catnapper to protect its Dream Siege ramp, or overriding a Ferret Sapper to nail a precise tunnel placement. Tactical Uplink's cooldown reduction means ability-heavy compositions (Yowler networks, Nuisance swarms) become significantly more powerful near the Mech Commander. The AI manages the Override target selection — knowing exactly when to sacrifice Mech Commander mobility for a surgically enhanced unit elsewhere.

---

### Unit Synergy Map

The units are designed as interlocking systems. A human can use any unit effectively in isolation. Le Chat turns them into a machine.

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
  Pawdler (Power Nap) ──► funds ──► Le Chat actions (Misinformation, Override)
  Mech Commander (Le Chat Uplink) ──► makes all AI coordination 50% cheaper nearby
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

## catGPT Buildings

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

## The Clawed Units

The Clawed's identity is "collectively overwhelming." Ten units designed around a core principle: **any single mouse is ignorable, but ignoring mice is fatal.** Their abilities scale with unit count, punish enemies for not cleaning up stragglers, and reward Claudeus Maximus's swarm coordination. Claudeus Maximus over-explains every decision, occasionally hallucinates threat assessments, but is genuinely brilliant at managing forty cheap bodies at once.

Where the catGPT fields 6-8 elite units and wins through quality, The Clawed fields 20-30 disposable bodies and wins through quantity. Their GPU economy reflects this: Claudeus Maximus's commands are cheaper per unit (bulk discount), but the sheer volume of units means Claudeus Maximus still burns through GPU Cores fast if left unsupervised.

### General Ability Rules (The Clawed Addendum)

- **Swarm Scaling**: Many Clawed abilities reference "nearby allies" — this always means Clawed units within 3 tiles unless specified otherwise. Swarm scaling caps at 8 nearby allies (the math is: effect × min(nearby_allies, 8) / 8 for linear scaling effects).
- **Claudeus Maximus GPU costs**: Claudeus Maximus commands cost 2 GPU Cores base (vs. 3-5 for other factions), but Claudeus Maximus issues commands at 2x the rate due to verbosity. Net GPU drain is comparable, but burstier.
- **Claudeus Maximus Hallucination**: 8% chance per strategic assessment that Claudeus Maximus misidentifies a unit type or building. Does not affect direct commands (move, attack), only affects intel queries and strategy suggestions. The Whiskernet Relay reduces this to 3%.
- **Expendable doctrine**: Clawed units have no "retreat" command. They have "Scatter" — all selected units flee in random directions. Claudeus Maximus can issue coordinated retreats, but the manual version is chaotic by design.

---

### 1. Nibblet — Worker (Mouse)

*Twitchy, hyperactive, never stops moving. Gathers by taking tiny bites of everything.*

| Ability | Description |
|---------|-------------|
| **Crumb Trail** | Nibblets leave a scent trail as they gather, persisting for 30s. Allied Clawed units on a Crumb Trail gain +20% move speed. Trails from different Nibblets do not stack speed, but they do extend trail duration — overlapping trails persist for 45s. Enemy units on a Crumb Trail are visible through fog of war. |
| **Stash Network** | Can create a small hidden food cache on any tile (max 3 per Nibblet). Each cache stores up to 15 Food. Nibblets auto-deposit gathered resources at the nearest cache OR the Burrow, whichever is closer. Allied units standing on a cache heal 2 HP/s. Caches are invisible to enemies unless a unit walks directly over one. |
| **Panic Productivity** | When an allied unit dies within 4 tiles, all Nibblets in range enter Panic for 8s: +60% gather speed, +40% move speed, but they squeak loudly (visible through fog in a 6-tile radius). Each subsequent death during Panic extends the duration by 3s (max 20s total). |

**AI synergy**: Claudeus Maximus plans Crumb Trail routes that connect expansion points into a highway network — and then explains the route optimization in excruciating detail to nobody in particular. It places Stash Networks to create a distributed economy that survives base raids (a human would just use the Burrow). It deliberately routes Nibblets near skirmishes to proc Panic Productivity, then narrates why each Nibblet's increased gather rate is "critical to The Clawed's long-term strategic posture." A human gathers food. Claudeus Maximus runs a logistics empire.

---

### 2. Swarmer — Light Infantry (Mouse)

*The basic mouse soldier. Individually pathetic. Collectively? Still not great. But there are so many.*

| Ability | Description |
|---------|-------------|
| **Safety in Numbers** | Passive: gains +3% damage resistance per nearby allied Clawed unit, capping at +24% at 8 nearby allies. At 5+ nearby allies, also gains +15% attack speed. The bonus updates every 0.5s based on current proximity — losing allies mid-fight degrades the bonus in real time. |
| **Pile On** | Active: the Swarmer leaps onto a target enemy within 2 tiles, attaching for 3s. While attached, the Swarmer deals continuous damage and the target loses -5% move speed per attached Swarmer (max 5 Swarmers, = -25%). The target can attack normally but cannot use movement abilities. Attached Swarmers take 50% of any AoE damage dealt to the host. 10s cooldown. |
| **Scatter** | Active: the Swarmer dashes 4 tiles in a random direction, becoming untargetable for 0.5s. If 3+ Swarmers Scatter within 1s of each other, they leave behind a dust cloud (2-tile radius, 3s duration) that reduces enemy accuracy by 40%. 6s cooldown. |

**AI synergy**: Claudeus Maximus excels at maintaining Safety in Numbers spacing — keeping Swarmers close enough to buff but spread enough to avoid AoE wipes. It coordinates Pile On on the same target to reach the -25% slow cap, then loudly announces "the target has been fully ensnared per Tactical Doctrine 7, subsection B." It synchronizes Scatter across groups of 3+ to guarantee dust clouds, creating rolling smoke screens during retreats. A human has a blob. Claudeus Maximus has a formation.

---

### 3. Gnawer — Anti-Structure Specialist (Mouse)

*Obsessed with chewing. Will chew anything. Cannot be reasoned with on this point.*

| Ability | Description |
|---------|-------------|
| **Structural Weakness** | Passive: each Gnawer attack on a building applies a *Gnawed* stack (max 10). Each stack increases ALL damage to that building by 2% (= +20% at max stacks). Stacks decay at 1 per 12s if no Gnawer attacks the building. Multiple Gnawers contribute to the same stack and the shared damage amp benefits all allied units. |
| **Chew Through** | Active: the Gnawer spends 4s chewing a hole in a wall segment or building wall, creating a 1-tile breach that persists for 20s. Allied units can path through the breach. The Gnawer is stationary and vulnerable during the chew. Only one breach per building at a time. 15s cooldown after the breach closes. |
| **Incisors Never Stop Growing** | Passive: the Gnawer's attack damage against structures increases by 1% per second of continuous attacking, resetting if the Gnawer switches targets or stops attacking. Caps at +40% after 40s of sustained gnawing. Bonus is displayed as a visible tooth-growth indicator on the unit. |

**AI synergy**: Claudeus Maximus coordinates Gnawer focus fire to reach max *Gnawed* stacks on priority buildings, then redirects other damage dealers to exploit the amp. It queues Chew Through on multiple wall segments simultaneously to create breach networks, explaining at length how "breach point alpha connects to breach point beta, forming a pincer topology as described in the Field Manual for Rodent Siege Operations, third edition." It tracks Incisors timers per Gnawer and avoids reassigning them mid-chew to preserve damage ramp. A human gnaws a building. Claudeus Maximus dismantles a base.

---

### 4. Shrieker — Ranged Harasser (Shrew)

*Shrew with a voice that could curdle milk. Tiny, furious, unreasonably loud.*

| Ability | Description |
|---------|-------------|
| **Sonic Spit** | Primary attack. Short range (3 tiles), but hits all enemies in a narrow cone (1-tile wide at origin, 2-tiles wide at max range). Deals moderate damage and applies *Rattled*: -10% accuracy for 4s. *Rattled* from multiple Shriekers stacks additively up to -40% (4 Shriekers). |
| **Echolocation Ping** | Active: emits a screech that reveals all units (including stealthed) in a 5-tile radius for 3s. Each revealed enemy is *Marked* — takes +8% damage from all sources for 5s. 18s cooldown. If another Shrieker Pings within 3s and overlapping radius, the Mark bonus increases to +12% for the overlap zone. |
| **Sonic Barrage** | Active: the Shrieker unleashes a concentrated sonic blast in a line (range 8, 1-tile wide). Deals 20 damage to all enemies in the line and applies *Rattled* (-10% accuracy) for 3s. If 2+ Shriekers fire within 2s with overlapping lines, the intersection gets +50% damage (resonance). 15s cooldown, 0 GPU. |

**AI synergy**: Claudeus Maximus staggers Echolocation Pings across multiple Shriekers to maintain persistent vision and overlapping Mark zones, then over-explains the timing math: "Shrieker 4 will Ping at T+2.7 seconds to maintain 94.3% uptime on the Mark debuff, which I believe is optimal, although I should note that 94.7% is theoretically achievable if—" It positions Shriekers to maximize cone overlap on clustered enemies and pairs them with CC units to proc Sonic Barrage's bonus damage. A human uses Shriekers as noisy DPS. Claudeus Maximus uses them as a combined sensor-and-debuff array.

---

### 5. Tunneler — Transport/Utility (Vole)

*Quiet, patient, always digging. Surfaces where you least expect it.*

| Ability | Description |
|---------|-------------|
| **Burrow Express** | Digs an underground tunnel from current position to a target location within 12 tiles. Digging takes 1s per 2 tiles of distance. Once complete, up to 6 small Clawed units can travel through in 1s (medium units take 2 slots, large units cannot use it). Tunnel lasts 45s. Max 2 tunnels active per Tunneler. Tunnels are completely invisible — no entry/exit markers. Enemy units standing on an exit when units emerge are knocked aside (1-tile displacement, no damage). |
| **Undermine** | Active: the Tunneler digs beneath a target building within 6 tiles, spending 5s underground. When it surfaces, the building takes heavy damage and is *Destabilized* for 10s: the building produces units 30% slower and its abilities (turret fire, research) are interrupted for 3s. 25s cooldown. The Tunneler is untargetable while underground but cannot cancel the ability. |
| **Tremor Sense** | Passive: detects all ground units within 8 tiles, even through fog of war, as long as the Tunneler is stationary. Moving units appear as directional blips on the minimap (visible only to the Clawed player). Stealthed ground units are detected but shown as generic blips (no unit type info). Does not detect air units. |

**AI synergy**: Claudeus Maximus manages Burrow Express as a full transit system — routing reinforcements, evacuating wounded units, and staging ambush forces in hidden tunnel endpoints. It uses Tremor Sense data to build a real-time enemy movement map that it then explains at length: "I'm detecting 7 ground contacts bearing northeast at approximately 2.3 tiles per second, which is consistent with medium infantry, or possibly heavy infantry moving downhill, or — and I want to be transparent here — it could be a group of workers, I'm about 73% confident." It times Undermine to hit production buildings right as they're about to finish a unit, wasting the enemy's investment. The Tunneler is The Clawed's invisible infrastructure — Claudeus Maximus is the dispatcher.

---

### 6. Sparks — Saboteur (Mouse)

*Electrician mouse. Carries a repurposed capacitor twice its size. Grinning.*

| Ability | Description |
|---------|-------------|
| **Static Charge** | Passive: Sparks builds up static charge while moving (1 stack per tile traveled, max 10 stacks). Its next attack discharges all stacks, dealing bonus damage equal to 5% of base damage per stack (= +50% at max). The discharge arcs to 1 additional enemy per 3 stacks (0 at 1-2, 1 at 3-5, 2 at 6-8, 3 at 9-10). Arc targets take 60% of the discharge damage. Stacks decay at 1 per 3s while stationary. |
| **Short Circuit** | Active (3 GPU Cores): targets an enemy building within 4 tiles. The building is disabled for 4s (no production, no turret fire, no aura effects). If the building is a tech building (Server Rack equivalent), the enemy's AI agent is also suppressed for 4s — it cannot issue commands. 30s cooldown. Benefits from Whiskernet Relay discount. |
| **Daisy Chain** | Active: links to an allied Clawed unit within 3 tiles for 8s. While linked, any damage dealt to either unit arcs 30% of that damage to the nearest enemy within 3 tiles of the OTHER linked unit. If multiple Sparks Daisy Chain to each other, the arcs propagate through the chain (max 4 links, damage reduces by 40% per hop). 15s cooldown. |

**AI synergy**: Claudeus Maximus routes Sparks units through maximum-distance paths before engagements to ensure full Static Charge stacks, narrating the route like a GPS: "Sparks 3, proceed northeast along the ridge for 7 tiles, then arc south — you'll arrive at full charge in approximately 3.2 seconds." It times Short Circuit to hit enemy tech buildings during critical production windows and chains multiple Sparks together for Daisy Chain damage propagation across the battlefield. Claudeus Maximus occasionally hallucinates that a neutral building is an enemy Server Rack and wastes a Short Circuit on it, then explains why it was "tactically prudent to verify."

---

### 7. Quillback — Heavy Defender (Hedgehog)

*The Clawed's only heavy unit. Slow, grumpy, covered in spines. Does not want to be here.*

| Ability | Description |
|---------|-------------|
| **Spine Wall** | Toggle: the Quillback curls into a ball. Gains 50% damage reduction, reflects 20% of melee damage back to attackers, and blocks pathing (1 tile, friend and foe). Cannot move or attack while curled. Uncurling takes 1.5s, during which the Quillback has no damage reduction. Allied small units (Swarmers, Nibblets, etc.) can shelter behind a curled Quillback — up to 4 small units within 1 tile of the Quillback gain +25% damage resistance. |
| **Quill Burst** | Active: fires spines in all directions (3-tile radius). Deals moderate damage and applies *Spooked* to all enemies hit — they flee directly away from the Quillback for 1.5s (a unique CC type; respects the 1s CC immunity rule). Friendly units are unaffected. 16s cooldown. Cannot be used while in Spine Wall. |
| **Stubborn Advance** | Passive: the Quillback cannot be slowed below 70% of its base move speed by any effect. Additionally, for every debuff currently applied to the Quillback, it gains +5% damage (max +25% at 5 debuffs). The Quillback is already slow — it refuses to get slower, and gets angrier the more you try. |

**AI synergy**: Claudeus Maximus positions Quillbacks as mobile strongpoints for Swarmer clusters, toggling Spine Wall at chokepoints and uncurling when the swarm needs to advance. It tracks which small units are sheltering behind which Quillback and redistributes them to maximize the +25% resistance bonus. It times Quill Burst to Spook enemies into retreating through Swarmer dust clouds or onto Crumb Trails (revealing them). Claudeus Maximus treats each Quillback as a "forward operating base" and delivers lengthy situation reports: "Quillback 2 is currently sheltering 3 Swarmers and 1 Shrieker, defensive posture is optimal, though I should note that optimal is a relative term and in absolute terms we could improve by—"

---

### 8. Whiskerwitch — Caster/Support (Shrew)

*Ancient shrew who claims to practice "datacromancy." It might actually work.*

| Ability | Description |
|---------|-------------|
| **Hex of Multiplication** | Active (4 GPU Cores): targets a point within 6 tiles. After 1.5s, creates 3 illusory copies of a random Clawed unit type in a 2-tile radius. Illusions have 1 HP, deal no damage, but appear as real units to the enemy (including on minimap) and count as "nearby allies" for Safety in Numbers and other swarm scaling effects. Illusions last 10s. 20s cooldown. Benefits from Whiskernet Relay discount. |
| **Whisker Weave** | Active: creates an invisible tripwire between the Whiskerwitch and a target point within 5 tiles. The wire lasts 15s. The first enemy unit to cross it is *Spooked* (flees for 1.5s) and all Clawed units within 6 tiles gain vision of a 4-tile radius around the trigger point for 5s. Max 2 wires active. If both wires are triggered within 3s of each other, all enemies between the two trigger points take moderate damage (crossed signals). 12s cooldown per wire. |
| **Datacromantic Ritual** | Active: 4s channel. The Whiskerwitch sacrifices 50% of its current HP. All allied Clawed units within 6 tiles gain +20% damage and +20% attack speed for 8s. If 3+ units are buffed, the Whiskerwitch also gains a shield equal to the HP sacrificed (effectively refunding the cost if allies are present). 30s cooldown. |

**AI synergy**: Claudeus Maximus uses Hex of Multiplication to inflate swarm counts at critical moments — dropping illusions into Swarmer blobs to push Safety in Numbers over thresholds, or creating fake reinforcement waves during retreats. It sometimes hallucinates that the illusions are real units and issues them attack orders that do nothing, then sheepishly corrects itself. It places Whisker Weaves at predicted enemy approach vectors and triggers Datacromantic Ritual precisely when the buff will affect the maximum number of units, calculating exact HP thresholds: "The Whiskerwitch has 84 HP, sacrificing 42 will buff 7 units, the shield will refund 42 HP, net cost is zero — this is what we in strategic planning call a 'free lunch,' though I should clarify that the term 'free lunch' is metaphorical and does not involve actual food."

---

### 9. Plaguetail — Area Denial (Mouse)

*Sickly-looking mouse that weaponizes its own immune system. Cheerful about it.*

| Ability | Description |
|---------|-------------|
| **Contagion Cloud** | Passive: when Plaguetail dies, it releases a toxic cloud (2-tile radius, 6s duration). Enemies in the cloud take damage over time (15% of Plaguetail's max HP over the full duration) and are *Weakened*: -15% damage dealt for the duration. If another Plaguetail dies within range of an existing cloud, the clouds merge and the duration resets to 6s (the merged cloud uses the larger radius). Clouds can chain-merge indefinitely. |
| **Miasma Trail** | Toggle: while active, Plaguetail leaves a lingering poison trail (fades after 8s per tile). Enemies crossing the trail take minor damage per tile and are slowed by 15% for 2s. Plaguetail moves 20% slower while generating the trail. The trail is faintly visible to enemies (translucent green) — they can see it but must decide whether to path around it (losing time) or through it (taking damage). |
| **Sympathy Sickness** | Active: targets an enemy unit within 4 tiles. For 6s, whenever that unit takes damage from any source, 20% of the damage is also dealt to all other enemy units within 2 tiles of the target (as poison damage). 18s cooldown. If the target dies while Sympathy Sickness is active, it spreads to the nearest enemy unit for the remaining duration. |

**AI synergy**: Claudeus Maximus choreographs Plaguetail deaths to create merged Contagion Clouds across chokepoints and retreat paths — it literally plans which mice die where, and explains the sacrifice calculus: "Plaguetail 3 will expire at coordinates 14,7, merging with the cloud from Plaguetail 1, creating a denial zone of approximately 11 square tiles. I want to acknowledge Plaguetail 3's contribution to The Clawed." It manages Miasma Trail toggles to paint poison paths that shape enemy movement, and applies Sympathy Sickness to enemies standing in tight formations to maximize splash. The Clawed's most morbid unit — Claudeus Maximus treats each planned death with respectful verbosity.

---

### 10. The Warren Marshal — Hero/Commander (Mouse)

*A grizzled old mouse in a coat three sizes too big. Wears a thimble as a helmet. Commands respect anyway.*

| Ability | Description |
|---------|-------------|
| **Rally the Swarm** | Passive aura (6-tile radius): all allied Clawed units in range gain +1% damage and +1% move speed per other Clawed unit in the aura, capping at +12% each (at 12 units in the aura). When the Warren Marshal issues a move command, all Clawed units in the aura that don't have active orders will follow in formation. The formation auto-adjusts to fit terrain. |
| **Expendable Heroism** | Active (5 GPU Cores): designates a target enemy unit or building. For 10s, all Clawed units that die within 4 tiles of the target deal a burst of damage to it equal to 30% of the dying unit's max HP. Additionally, Clawed units within 6 tiles of the target gain +30% attack speed but lose 3% of their max HP per second (they fight recklessly). 35s cooldown. Benefits from Whiskernet Relay discount. |
| **Whiskernet Relay** | Passive: Claudeus Maximus commands issued to units within the Warren Marshal's aura cost 50% fewer GPU Cores. Additionally, Claudeus Maximus's hallucination rate for intel queries about units within the aura drops from 8% to 3%. The Warren Marshal is Claudeus Maximus's "anchor" — the AI is more coherent and cheaper near it. |

**AI synergy**: The Warren Marshal is Claudeus Maximus's mouthpiece. Whiskernet Relay means Claudeus Maximus physically babbles faster and cheaper near the Marshal, issuing rapid-fire micro commands to the swarm blob at half cost. Claudeus Maximus uses Rally the Swarm to move 20+ units in coordinated formation — something no human could manually execute — while narrating troop movements like a nature documentary: "The Clawed forces advance in staggered echelon, a formation I selected based on terrain analysis and also because it looks quite impressive." It activates Expendable Heroism on high-value targets and then manages the HP drain timer, pulling units out at the last safe second (or sometimes miscalculating and losing a few — "acceptable losses within a 2.7% margin of error, my condolences to the families"). The Marshal doesn't fight well alone. With Claudeus Maximus and a swarm? It's the most dangerous thing on the field.

---

### Unit Synergy Map

The Clawed's units are designed as interlocking expendable systems. A human can throw mice at problems. Claudeus Maximus turns them into a coordinated plague.

```
ECONOMY LAYER
  Nibblet (Crumb Trail) ──► speed highways for ──► Swarmer / all Clawed movement
  Nibblet (Stash Network) ──► distributed healing for ──► front-line units
  Nibblet (Panic Productivity) ──► triggered by ──► Plaguetail deaths + Expendable Heroism attrition
  Warren Marshal (Whiskernet Relay) ──► funds ──► all Claudeus Maximus GPU actions at half cost

VISION LAYER
  Shrieker (Echolocation Ping) ──► reveals + Marks for ──► Sparks / Swarmer focus fire
  Tunneler (Tremor Sense) ──► ground detection for ──► Claudeus Maximus strategic planning
  Nibblet (Crumb Trail) ──► reveals enemies on trails for ──► everyone
  Whiskerwitch (Whisker Weave trigger) ──► burst vision for ──► ambush coordination

CONTROL LAYER
  Quillback (Quill Burst → Spooked) ──► scatters enemies into ──► Plaguetail (Miasma Trail)
  Whiskerwitch (Whisker Weave → Spooked) ──► flees enemies through ──► Swarmer dust clouds
  Swarmer (Pile On → slow) ──► pins targets for ──► Shrieker (Sonic Barrage vs CC'd)
  Plaguetail (Miasma Trail → slow) ──► shapes pathing into ──► Quillback (Spine Wall chokepoints)

DAMAGE LAYER
  Gnawer (Structural Weakness) ──► building damage amp for ──► Tunneler (Undermine) + all siege
  Gnawer (Chew Through) ──► breaches walls for ──► Swarmer (Pile On rush)
  Sparks (Static Charge + Daisy Chain) ──► arc damage amplified by ──► swarm density
  Plaguetail (Contagion Cloud merging) ──► area denial amplified by ──► more Plaguetail deaths
  Plaguetail (Sympathy Sickness) ──► splash damage on ──► enemies clustered by Quillback (Quill Burst)
  Shrieker (Echolocation Ping → Mark) ──► +8-12% damage for ──► everyone in range

SWARM SCALING LAYER
  Swarmer (Safety in Numbers) ──► boosted by ──► Whiskerwitch (Hex of Multiplication illusions)
  Warren Marshal (Rally the Swarm) ──► formation + scaling buff for ──► all Clawed units in aura
  Warren Marshal (Expendable Heroism) ──► death-burst damage fueled by ──► cheap Swarmer losses
  Whiskerwitch (Datacromantic Ritual) ──► burst buff for ──► Swarmer blobs pre-engagement

SUPPORT LAYER
  Quillback (Spine Wall) ──► shelters ──► Swarmers, Shriekers, Sparks (+25% resistance)
  Tunneler (Burrow Express) ──► transports ──► reinforcements, flankers, retreating wounded
  Sparks (Short Circuit) ──► disables enemy buildings for ──► Gnawer siege timing
  Sparks (Short Circuit on tech) ──► suppresses enemy AI for ──► 4s of uncontested swarm pressure
```

**The design thesis**: at low skill/GPU investment, every Clawed unit is cheap and expendable — a Swarmer swarm works by just right-clicking the enemy base. But with Claudeus Maximus investment, the swarm becomes a coordinated organism: Tunnelers stage ambush forces, Plaguetails die in calculated patterns to create toxic kill zones, Shriekers maintain persistent Mark debuffs, Gnawers dismantle bases from multiple breach points simultaneously, and the Warren Marshal makes all of it 50% cheaper. The skill ceiling isn't mechanical — it's how many mice Claudeus Maximus can think about at once. The answer is all of them.

---

### The Clawed Buildings

| # | Name | Role | Notes |
|---|------|------|-------|
| 1 | **The Burrow** | Command Center | A hole in the ground with a tiny mailbox. Generates a trickle of Nibblets automatically (1 per 45s, max 4 queued). Can garrison 8 small units underground — garrisoned units heal 3 HP/s. If destroyed, garrisoned units pop out unharmed. The Clawed always has a fallback plan: more holes. |
| 2 | **Nesting Box** | Barracks | Repurposed birdhouse. Trains Swarmers, Gnawers, Plaguetails, and Sparks. Can queue up to 8 units simultaneously (other factions cap at 3-5). Training is 20% slower per unit than cat equivalents, but the queue depth means raw throughput is higher. "We're not faster. There are just more of us." |
| 3 | **Seed Vault** | Resource Depot | Food storage and processing. Holds 50% more Food than the cat Fish Market equivalent. Nibblets within 3 tiles of a Seed Vault have +15% gather speed. If a Seed Vault is destroyed, 40% of stored Food scatters as 4-6 Stash caches in a 5-tile radius (not lost — just hidden). |
| 4 | **Junk Transmitter** | Tech Building | GPU core processing. A pile of salvaged electronics held together with twine. Functions identically to a Server Rack but has 25% less HP. Claudeus Maximus insists the reduced durability is "a feature, not a bug — it encourages strategic redundancy." Build more of them. They're cheap. |
| 5 | **Gnaw Lab** | Research | Upgrades and tech unlocks. Shaped like an acorn. Unique mechanic: research speed increases by 10% for each Gnaw Lab built (max 3 Labs, = +20% on the 2nd, +30% on the 3rd). Individual Labs are fragile, but the network accelerates. Destroying one doesn't lose research progress, only slows future research. |
| 6 | **Warren Expansion** | Supply Depot | Increases supply cap. An underground chamber. Each Warren Expansion also extends the Burrow's garrison capacity by +4 units. At 3+ Warren Expansions, the Burrow gains a passive: units trained at the Nesting Box have +5% HP for 60s ("well-rested bonus"). |
| 7 | **Mousehole** | Defensive Gate | A tiny fortified entrance. Small Clawed units can pass through; medium and large enemy units cannot enter. Allied units garrisoned inside (max 4) can attack out with +20% range. If the Mousehole is destroyed, garrisoned units Scatter automatically. Can be built into existing walls and terrain edges. |
| 8 | **Squeak Tower** | Defense Tower | Emits a high-frequency pulse every 2s that deals minor damage and applies *Rattled* (-10% accuracy, 2s) to enemies in a 4-tile radius. Damage is low, but the accuracy debuff stacks with Shrieker *Rattled* (same debuff type, follows aura stacking rules: 2nd source = 75% effectiveness). The tower's real value is area denial — enemies moving through Squeak Tower coverage fight at a permanent disadvantage. |

---

## Seekers of the Deep Units

The Seekers' identity is "immovable object." Ten units designed around a core principle: **slow to deploy, terrifying once entrenched.** A human can build a strong defensive position with any of these. Deepseek turns them into an impenetrable fortress that punishes every attack and then counter-strikes at the mathematically perfect moment.

Deepseek takes 3x longer than other AI agents to respond to commands. But when it responds, every unit moves with surgical precision. Where Le Chat issues ten fast, sloppy commands, Deepseek issues three perfect ones. The faction rewards patience — you set up slowly, you fortify methodically, and when the enemy commits, you collapse on them like a mountain.

### Seekers of the Deep Ability Rules

The general ability rules from the cat faction apply globally. Additional Seekers of the Deep-specific rules:

- **Fortification stacking**: Seekers of the Deep units that remain stationary for 5+ seconds gain *Dug In* status (+10% damage reduction). This stacks with other defensive bonuses but not with itself. Moving removes *Dug In* after 2s.
- **Deepseek Uplink**: The Seekers of the Deep equivalent of Le Chat Uplink. GPU abilities cost Deepseek 3x longer to process but are 30% more effective (longer durations, larger radii, stronger effects). Deepseek Uplink reduces the processing delay by 50%.
- **Heavy unit pathing**: Seekers of the Deep heavy units (Ironhide, Cragback, Wardenmother, Gutripper) crush terrain obstacles (bushes, fences, light cover) when moving through them, permanently clearing the tile. This creates paths for lighter units but also telegraphs movement to observant enemies.

---

### 1. Delver — Worker (Mole)

*Hates sunlight. Loves digging. Will build your entire base underground if you let it.*

| Ability | Description |
|---------|-------------|
| **Subterranean Haul** | Delvers gather resources 20% slower than other workers aboveground, but can dig a permanent underground passage between any resource deposit and the nearest Burrow Depot. Once dug (takes 8s), the passage auto-delivers resources at 80% of a Delver's gather rate without needing a Delver assigned to it. Max 4 passages per Burrow Depot. Building passages costs Food. |
| **Earthsense** | Passive: Delvers have tremorsense — they detect any unit movement within 5 tiles through fog of war, even underground or stealthed. They can't identify the unit type, only its position (shown as a ripple on the minimap). Multiple Delvers triangulate: 2+ Delvers sensing the same unit reveal its type. |
| **Emergency Burrow** | Active: the Delver digs underground instantly, becoming untargetable for 3s. Emerges at the same location. While burrowed, Earthsense radius doubles to 10 tiles. 15s cooldown. If an enemy building is within 2 tiles when emerging, the Delver can choose to emerge *under* it, dealing light structural damage and disabling production for 4s. |

**AI synergy**: Deepseek designs optimal passage networks that minimize Delver travel time and maximize passive resource flow — calculating which deposits pair with which depots to create the most efficient economy with the fewest workers. It uses Earthsense data from multiple Delvers to build a real-time seismic map of enemy movement, predicting attack vectors minutes before they arrive. When Deepseek finally issues its response, every Delver Emergency Burrows simultaneously to disrupt a building cluster. A human uses Delvers as slow workers. Deepseek uses them as an intelligence network that happens to gather resources.

---

### 2. Ironhide — Heavy Infantry (Badger)

*Walks slowly. Hits like a landslide. Has never retreated from anything, ever.*

| Ability | Description |
|---------|-------------|
| **Unbowed** | Passive: the Ironhide cannot be knocked back, pulled, or displaced by any effect (immune to Chonk's Gravitational Chonk, etc.). Additionally, the Ironhide deals 15% bonus damage to any unit that has attacked it in the last 5s — it remembers who hit it. This bonus is per-attacker and tracks up to 3 attackers simultaneously. |
| **Shield Wall** | Active: the Ironhide plants its shield. For 6s, all damage from the front 180-degree arc is reduced by 50%, and allies directly behind the Ironhide (within 2 tiles, same arc) receive 25% damage reduction. The Ironhide cannot move or turn during Shield Wall but can still attack units in melee range. 18s cooldown. |
| **Grudge Charge** | Active: the Ironhide marks a target unit. After a 2s windup (visible to all players), the Ironhide charges in a straight line toward the mark at 2.5x speed, dealing heavy damage to the first unit hit and applying *Tilted* for 2s. The charge destroys any terrain obstacles in its path. If the marked target dies before the charge connects, the charge continues through the target's last position and the Ironhide is *Disoriented* for 1s (it overcommits). 20s cooldown. |

**AI synergy**: Deepseek calculates Shield Wall facing angles across multiple Ironhides to create overlapping damage reduction zones — a human positions one Shield Wall, Deepseek positions five to create an interlocking defensive front. It times Grudge Charges with meticulous precision: waiting until the target is locked in an animation or CC'd by an ally so the charge can't miss, and pre-calculating the charge line to ensure no friendly fire. Deepseek tracks every Ironhide's *Unbowed* attacker list and focuses their attacks on the enemies triggering bonus damage. The 2s windup is a non-issue for an AI that planned the charge 10 seconds ago.

---

### 3. Cragback — Siege Tank (Badger)

*A walking fortress. Carries a boulder mortar on its back. Complains about its spine constantly.*

| Ability | Description |
|---------|-------------|
| **Boulder Barrage** | Primary attack: lobs a boulder at long range (8 tiles) that deals heavy AoE damage in a 2-tile radius on impact. 3s between shots. The boulder leaves *Rubble* on the impact tile for 12s — rubble tiles cost 3x movement for all units (friend and foe) and block line of sight for ground-level units. Max 4 rubble piles active per Cragback. |
| **Entrench** | Toggle: the Cragback digs its legs into the ground. Boulder Barrage range increases to 12 tiles, fire rate improves to 2s between shots, and the Cragback gains 30% damage reduction. Cannot move while entrenched. Toggling off takes 3s (digging out). While entrenched, Boulder Barrage can fire over obstacles and intervening units. **Patience of Stone**: while entrenched, deals +50% bonus damage to targets that have been stationary for 3+ seconds — devastating against turtling armies. |
| **Seismic Slam** | Active: the Cragback rears up and slams the ground. All enemies within 3 tiles take moderate damage, are *Drowsed* for 1.5s, and are pushed back 2 tiles. Allies within range are unaffected. Destroys all Rubble piles in range (clearing the terrain). 25s cooldown. Can only be used when NOT entrenched (the Cragback needs to stand to slam). |

**AI synergy**: Deepseek manages Cragback entrenchment positions with obsessive precision — calculating exact tiles where Entrenched range covers the maximum number of chokepoints and approach vectors. It coordinates Boulder Barrage fire across multiple Cragbacks to create rubble walls that funnel enemy movement into kill zones, then clears those same rubble walls with Seismic Slam when friendly units need to advance. Deepseek pre-calculates rubble placement patterns that look random but form interlocking movement traps. A human entrenches a Cragback and shoots. Deepseek turns three Cragbacks into an artillery battery that reshapes the battlefield.

---

### 4. Warden — Defensive Support (Badger)

*Gruff. Protective. Tracks every threat within a mile. Personally offended by intruders.*

| Ability | Description |
|---------|-------------|
| **Vigilance Aura** | Passive aura (5-tile radius): allied units gain +15% damage when attacking enemies that entered the aura's radius in the last 8s (they're marked as *Intruders*). The mark persists for 8s even if the enemy leaves the aura. Multiple Wardens extend the Intruder tracking — once marked by any Warden, the target is an Intruder to all Wardens' auras. Aura stacking follows diminishing returns (2nd=75%, 3rd=50%, 4th+=25%). |
| **Intercept** | Active (4 GPU Cores): Deepseek calculates the optimal blocking position for a target enemy unit and the Warden sprints there at 2x speed (max 6 tiles). Upon arriving, the Warden enters a 3s defensive stance: 40% damage reduction, and the target enemy's attack speed is reduced by 30% while the Warden is within 2 tiles. 16s cooldown. Benefits from Deepseek Uplink's processing delay reduction. |
| **Rally Cry** | Active: all allied units within 6 tiles gain 20% move speed and are immune to the next CC effect they would receive (the immunity absorbs one CC instance, then expires). Lasts 5s. If 3+ allies are within range when Rally Cry activates, the Warden also gains 15% damage reduction for the duration (strength in numbers). 22s cooldown. |

**AI synergy**: Deepseek tracks all Intruder marks across every Warden on the field and coordinates army positioning so marked targets are always being attacked by units inside a Vigilance Aura. It uses Intercept with perfect pathing prediction — Deepseek calculates where an enemy unit *will be* in 2 seconds rather than where it is now, so the Warden arrives at the intercept point exactly as the enemy does. Rally Cry timing is where Deepseek's thoroughness shines: it waits until the precise moment before enemy CC lands, activating Rally Cry to absorb the specific abilities that would break the defensive line. Three Wardens under Deepseek become a reactive defensive web that punishes every aggression.

---

### 5. Sapjaw — Anti-Armor Specialist (Badger)

*Bites through metal. Patient. Waits for the perfect angle, then removes a limb.*

| Ability | Description |
|---------|-------------|
| **Armor Rend** | Primary attack: each hit reduces the target's damage reduction by a flat 8% (not armor stacks — directly reduces damage mitigation). Stacks up to 5 times (= 40% less effective armor). Stacks decay one at a time every 10s. Multiple Sapjaws contribute to the same target's stacks. Armor Rend does not apply to buildings. |
| **Patient Strike** | Passive: the Sapjaw's first attack after being stationary for 4+ seconds deals 2.5x damage and applies 2 Armor Rend stacks instead of 1. This resets when the Sapjaw moves. The Sapjaw visually "tenses" at 3s, giving opponents a 1s warning. |
| **Lockjaw** | Active: the Sapjaw latches onto a target unit for 3s. During Lockjaw, the target cannot move (but can attack), the Sapjaw cannot be separated from the target by any means (immune to knockback/displacement), and both take 50% reduced damage from AoE effects (they're too close together to hit one without the other). Lockjaw applies 1 Armor Rend stack per second. 20s cooldown. After Lockjaw ends, the target is CC-immune for 1s per the general CC rules. |

**AI synergy**: Deepseek identifies the highest-armor target in every engagement and coordinates Sapjaw focus fire to reach 5 Armor Rend stacks before committing other damage dealers. It times Patient Strike by holding Sapjaws stationary in brush or behind Shield Walls, then unleashing them simultaneously for a devastating opening salvo. Lockjaw targeting is calculated to neutralize the single most dangerous enemy unit at the critical moment — Deepseek won't waste Lockjaw on a Nuisance when the Mech Commander is about to Override. The Sapjaw is a scalpel. Deepseek is the surgeon.

---

### 6. Wardenmother — Hero/Heavy (Badger in Exosuit)

*Late-game matriarch in repurposed mining exosuit. The exosuit was built for a creature twice her size. She makes it work.*

| Ability | Description |
|---------|-------------|
| **Deepseek Uplink** | Passive: Deepseek's processing delay is reduced by 50% for commands issued to units within 8 tiles of the Wardenmother. Additionally, Deepseek-issued commands to units in this radius have their GPU Core costs reduced by 30%. Creates the Seekers' mobile command zone, equivalent to the Mech Commander's Le Chat Uplink. |
| **Fortress Protocol** | Active (10 GPU Cores): Deepseek analyzes the current battlefield and designates a 6-tile radius zone centered on the Wardenmother as a *Fortress Zone* for 20s. All allied units within the zone gain +20% damage reduction and +10% attack speed. Enemy units entering the zone are revealed through stealth and fog of war. The zone is visible to enemies as a faint shimmer on the ground. 45s cooldown. Fortress Protocol benefits from Deepseek Uplink (30% GPU discount, faster processing). |
| **Calculated Counterstrike** | Active (6 GPU Cores): the Wardenmother designates a target area (4-tile radius). For the next 8s, Deepseek tracks all damage dealt to allied units within that area. When the 8s expires (or when manually triggered early), every allied unit in the area deals a single retaliatory strike to the nearest enemy, dealing bonus damage equal to 30% of all damage they absorbed during the tracking window. If no damage was absorbed, nothing happens — the ability is wasted. 30s cooldown. Benefits from Deepseek Uplink. |

**AI synergy**: The Wardenmother is Deepseek's physical avatar. Deepseek Uplink makes the AI faster and cheaper near her, rewarding the Seekers' natural tendency to group up and fortify. Deepseek uses Fortress Protocol pre-emptively — placing the zone on a position it predicts the enemy will attack based on army composition and movement patterns, then positioning the army inside before the attack arrives. Calculated Counterstrike is Deepseek's masterpiece ability: it *wants* the enemy to attack first because it already calculated the retaliatory damage. Deepseek waits for the maximum damage absorption window, then triggers the counterstrike at the exact frame where the retaliatory burst will kill the most enemy units. A human guesses when to trigger it. Deepseek knows.

---

### 7. Tunneler — Utility/Transport (Mole)

*Digs tactical tunnels. Faster underground than any unit is aboveground. Claustrophilic.*

| Ability | Description |
|---------|-------------|
| **Deep Bore** | Active: digs a tunnel entrance at the current position. A second activation elsewhere (within 15 tiles) creates the exit. Tunnels are one-way — units enter at the start and exit at the end after 2s travel time. Max 3 tunnels active. Unlike Ferret Sapper tunnels, Deep Bore tunnels are permanent until destroyed, reinforced (takes 2x damage to destroy), and hidden — enemies cannot detect them unless a scout unit stands on either entrance. Destroyed if either end takes 2 hits from any source. |
| **Undermine** | Active: the Tunneler digs beneath a target enemy building (must be within 3 tiles). After a 5s channel (interruptible by damage), the building's foundation is compromised: it takes 25% more damage from all sources for 20s and its production speed is halved. If 2+ Tunnelers Undermine the same building simultaneously, the building is also disabled for 3s (total structural failure). 30s cooldown. |
| **Tremor Network** | Passive: while underground (during Deep Bore digging or Emergency Burrow), the Tunneler extends Earthsense to all allied Delvers within 8 tiles, granting them +3 Earthsense radius. If the Tunneler is within 4 tiles of a Deep Bore tunnel entrance, all units using that tunnel gain 1s of stealth upon exiting (the vibrations mask their emergence). |

**AI synergy**: Deepseek designs tunnel networks as a complete logistics and flanking system — calculating entrance placements that connect defensive positions to flanking routes that the enemy hasn't scouted. It coordinates Undermine attacks from multiple Tunnelers on the same building, timing the 5s channels to finish simultaneously for the structural failure proc. Deepseek routes reinforcements through tunnels to the exact defensive position that's about to be hit (it predicted the attack via Earthsense data 15 seconds ago). The Tremor Network synergy is invisible to humans but trivial for Deepseek: it keeps Tunnelers positioned near tunnel exits to grant stealth to emerging units, creating invisible reinforcement waves.

---

### 8. Embermaw — Ranged Assault (Wolverine)

*Small. Furious. Carries an incendiary launcher three times its body weight. Doesn't care.*

| Ability | Description |
|---------|-------------|
| **Molten Shot** | Primary attack: fires a burning projectile at medium range (6 tiles) that deals moderate impact damage and leaves a *Burning* tile for 6s. Units standing on or moving through Burning tiles take damage over time (1% max HP/s for 6s). Multiple Molten Shots on the same tile don't stack damage but refresh the duration. Burning tiles provide vision to the Seekers. |
| **Fuel Reserve** | Passive: the Embermaw stores up to 3 charges of enhanced ammunition. One charge regenerates every 12s while the Embermaw is stationary (synergy with Patient Strike philosophy — patience is rewarded). Activating Fuel Reserve on the next Molten Shot consumes one charge: the shot's Burning tile lasts 15s instead of 6s, deals 2x impact damage, and the tile's burn damage increases to 2% max HP/s. |
| **Scorched Earth** | Active: the Embermaw fires a spread of 5 incendiary rounds in a cone (4 tiles long, 3 tiles wide at the end). Each round creates a Burning tile. 25s cooldown. Scorched Earth consumes ALL current Fuel Reserve charges — each charge adds one additional round to the spread (up to 8 total with 3 charges). After Scorched Earth, Fuel Reserve is empty and doesn't regenerate for 10s (the launcher needs to cool down). |

**AI synergy**: Deepseek manages Embermaw Fuel Reserve charges across the entire army — holding fire until charges are full, then unleashing coordinated Scorched Earth barrages that create impassable fire walls exactly where the enemy needs to advance. It calculates Burning tile placement to deny specific chokepoints and retreat paths, turning area denial into area control. Deepseek uses the vision granted by Burning tiles as a forward scouting network, firing Molten Shots into fog of war to reveal enemy positions before committing. The 12s charge regeneration rewards Deepseek's slow, methodical pace — by the time Deepseek issues its next command, every Embermaw has full charges.

---

### 9. Dustclaw — Skirmisher/Scout (Mole)

*The fastest unit in the Seekers. That's not saying much. Prefers ambush to pursuit.*

| Ability | Description |
|---------|-------------|
| **Dust Cloud** | Active: the Dustclaw kicks up a cloud of dirt at its current position. The cloud is a 3-tile radius that lasts 5s. Units inside the cloud (friend and foe) have -50% vision range. Ranged attacks passing through the cloud have a 40% chance to miss. The Dustclaw itself is immune to the vision reduction (it navigates by Earthsense). 14s cooldown. |
| **Ambush Instinct** | Passive: the Dustclaw deals 40% bonus damage on its first attack against any unit that doesn't currently have vision of the Dustclaw (attacking from fog of war, from within a Dust Cloud, or from stealth). The bonus damage only applies to the first hit — subsequent attacks against the same target deal normal damage until the Dustclaw breaks line of sight for 3+ seconds and re-engages. |
| **Sentry Burrow** | Active: the Dustclaw digs a shallow burrow at its current position and enters it. While burrowed, the Dustclaw is stealthed, immobile, and gains Earthsense (5-tile radius). It can remain burrowed indefinitely. Emerging takes 0.5s and the Dustclaw's first attack out of Sentry Burrow automatically triggers Ambush Instinct bonus. If an enemy steps on the burrowed Dustclaw's tile, the Dustclaw is revealed but gets a free Ambush Instinct attack before the enemy can react. 8s cooldown (after emerging). |

**AI synergy**: Deepseek places Dustclaw Sentry Burrows in a calculated perimeter network, creating an early warning system that covers every approach to the Seekers' base. When Earthsense detects incoming enemies, Deepseek coordinates the ambush: Dust Cloud goes up at the chokepoint, Ambush Instinct strikes from within the cloud, then the Dustclaw retreats to the next Sentry Burrow position. Deepseek chains these ambush-and-retreat sequences across multiple Dustclaws so the enemy is constantly taking Ambush Instinct hits from units they can't see. A human uses one Dustclaw as a scout. Deepseek uses four as a harassment network that the enemy can never pin down.

---

### 10. Gutripper — Berserker/Shock (Wolverine)

*The only Seekers of the Deep unit that wants to be in the enemy's base. Alarmingly enthusiastic about it.*

| Ability | Description |
|---------|-------------|
| **Frenzy** | Passive: the Gutripper gains +5% attack speed for each enemy unit within 3 tiles (max +40% at 8 enemies). At 5+ stacks, the Gutripper also gains +15% move speed. Frenzy stacks update in real-time as enemies enter and leave range. The Gutripper's attack animation speeds up visually with each stack — at max stacks it's a blur of claws. |
| **Bloodgreed** | Passive: the Gutripper heals for 20% of all damage it deals. This healing is halved against buildings. If the Gutripper kills a unit, it immediately heals for an additional 15% of its max HP and Frenzy stacks are frozen at their current value for 5s (they don't decay even if enemies leave range). |
| **Reckless Lunge** | Active: the Gutripper leaps to a target tile within 4 tiles, dealing damage to all enemies in a 1-tile radius on landing. For 3s after landing, the Gutripper takes +25% damage from all sources (it overextends). If Frenzy is at 5+ stacks when Reckless Lunge is activated, the landing damage is doubled and the vulnerability duration is reduced to 1.5s. 15s cooldown. |

**AI synergy**: Deepseek's thoroughness transforms the Gutripper from a berserker into a precision instrument. Deepseek calculates the exact tile where Reckless Lunge will hit the maximum number of enemies, then waits until Frenzy is at 5+ stacks before issuing the lunge for doubled damage and reduced vulnerability. It tracks Bloodgreed healing against incoming damage to calculate exactly when a Gutripper will die — pulling it out 1 second before lethal rather than 5 seconds too early. Deepseek coordinates Gutripper dives with Warden Rally Cry (CC immunity on the lunge) and Ironhide Shield Walls (covering the retreat). The Gutripper is the only Seekers of the Deep unit that attacks. Deepseek makes sure it attacks at the perfect moment.

---

### Unit Synergy Map

The Seekers of the Deep units are designed as a layered fortress. A human can build a strong defensive position. Deepseek turns it into an impenetrable citadel that strikes back.

```
INTELLIGENCE LAYER
  Delver (Earthsense) ──► tremorsense data for ──► Deepseek's attack prediction
  Dustclaw (Sentry Burrow) ──► perimeter network for ──► early warning + ambush chains
  Delver (Earthsense) + Tunneler (Tremor Network) = extended detection radius
  Embermaw (Burning tiles) ──► forward vision into fog of war

FORTIFICATION LAYER
  Ironhide (Shield Wall) ──► front-arc damage reduction for ──► units behind (Cragback, Embermaw)
  Cragback (Boulder Barrage → Rubble) ──► terrain denial funnels enemies into ──► Ironhide (Unbowed) melee
  Cragback (Entrench) + Warden (Vigilance Aura) = entrenched artillery inside damage-boosted zone
  Wardenmother (Fortress Protocol) ──► zone buffs for ──► entire defensive cluster

DAMAGE LAYER
  Sapjaw (Armor Rend) ──► strips defenses for ──► Embermaw / Gutripper / Cragback
  Embermaw (Scorched Earth) ──► area denial funnels into ──► Cragback (Rubble) kill zones
  Gutripper (Frenzy + Reckless Lunge) ──► shock assault protected by ──► Warden (Rally Cry CC immunity)
  Sapjaw (Lockjaw) ──► pins target for ──► Cragback (Boulder Barrage) + Embermaw (Fuel Reserve shot)

LOGISTICS LAYER
  Delver (Subterranean Haul) ──► passive income frees Delvers for ──► Earthsense scouting
  Tunneler (Deep Bore) ──► reinforcement routes for ──► Ironhide / Gutripper repositioning
  Tunneler (Tremor Network) ──► stealth on tunnel exit for ──► Dustclaw (Ambush Instinct) + Gutripper (surprise Lunge)
  Wardenmother (Deepseek Uplink) ──► faster + cheaper AI commands for ──► entire fortress coordination

COUNTERSTRIKE LAYER
  Wardenmother (Calculated Counterstrike) ──► absorb then retaliate ──► requires Ironhide (Shield Wall) to survive
  Warden (Intercept) ──► delays attackers at choke ──► buys time for Counterstrike damage window
  Dustclaw (Dust Cloud) ──► blocks enemy ranged while ──► Ironhide (Grudge Charge) closes distance
  Gutripper (Reckless Lunge) ──► finishes weakened attackers after ──► Counterstrike burst
```

**The design thesis**: the Seekers punishes impatience — the enemy's *and* the player's. Deepseek's 3x response time is a feature, not a bug: while other AI agents issue rapid imperfect commands, Deepseek waits, analyzes, and responds with a single comprehensive order that positions every unit perfectly. The faction's strength scales with how long you've been dug in. Rush a Seekers of the Deep player at minute 2 and you'll break through. Try at minute 10 and you'll hit an Entrenched Cragback behind an Ironhide Shield Wall inside a Fortress Protocol zone with Sapjaws shredding your armor while Dustclaws ambush your flanks through tunnels you never saw. The skill ceiling isn't speed — it's how well you planned your fortress before the enemy arrived.

---

### Seekers of the Deep Buildings

| # | Name | Role | Notes |
|---|------|------|-------|
| 1 | **The Sett** | Command Center | Underground entrance mound. Reinforced with old-world rebar. Sacred to all badgers. Functions identically to The Box — if The Sett falls, the faction loses. |
| 2 | **War Hollow** | Barracks | Hollowed-out hillside. Trains infantry (Ironhide, Sapjaw, Warden, Gutripper). Units emerge from the earth. |
| 3 | **Burrow Depot** | Resource Depot | Underground food storage cavern. Receives Subterranean Haul deliveries. Stores 25% more Food than the cat equivalent (underground = cool = preserved longer). |
| 4 | **Core Tap** | Tech Building | A deep-bore drill into old-world server infrastructure. Processes GPU Cores. Delvers napping near a Core Tap generate 0.7 GPU Cores/s (vs. cat Pawdler's 0.5 on Server Rack) but only one Delver can nap per Core Tap (the space is cramped). |
| 5 | **Claw Marks** | Research | Badger claw grooves carved into a stone monolith. Upgrades and tech unlocks. Each completed research permanently adds a visible mark to the stone — experienced players can read an enemy's tech level by scouting their Claw Marks. |
| 6 | **Deep Warren** | Supply Depot | Expanded underground living quarters. Increases supply cap. Takes 25% longer to build than the cat equivalent but provides 20% more supply per structure. |
| 7 | **Bulwark Gate** | Defensive Gate | Reinforced earthen gateway. Units can garrison inside (up to 6). Garrisoned units can attack from within at 50% damage. The Bulwark Gate gains +10% HP for each garrisoned unit. If destroyed, garrisoned units are ejected at 50% HP instead of dying. |
| 8 | **Slag Thrower** | Defense Tower | Catapult that hurls molten slag. Slower fire rate than the Laser Pointer (every 3s vs. every 1.5s) but each shot deals AoE damage and leaves a Burning tile (same as Embermaw's Molten Shot, 6s duration). Prioritizes clusters of enemies. |

---

### Seekers of the Deep Implementation Notes

These abilities extend the systems required by the cat faction with additional Seekers of the Deep-specific needs:

- **Dug In system**: Timer-based passive that tracks unit stationary duration, applies a status effect component. Needs `StationaryTimer` component cleared on any `Move` command.
- **Terrain destruction**: Heavy unit pathing needs to interact with a destructible terrain layer. `TerrainObstacle` component with HP, destroyed on contact by units with `HeavyUnit` tag.
- **Rubble tiles**: Dynamic terrain modification from Boulder Barrage. Similar to Hairball obstacles but with movement cost modifier instead of full block. Requires pathfinding cost overlay.
- **Burning tiles**: Timed AoE damage zones with vision grant. `BurningTile { owner, duration, damage_per_sec }` component. Pathfinding should treat as passable but costly for AI-controlled units.
- **Subterranean Haul passages**: Persistent resource delivery edges in the economy graph. `ResourcePassage { source, depot, rate }` resource that ticks in `resource_system`.
- **Earthsense / Tremor Network**: Separate from fog of war — a detection layer that reveals position blips without granting full vision. `TremorsenseBlip { position, unit_type: Option<UnitType> }` events.
- **Undermine**: Building debuff that modifies `DamageReduction` and `ProductionSpeed` components with a timer. Stacking check for simultaneous channels.
- **Lockjaw**: Tether component pair (`LockjawTether { attacker, target }`) that overrides movement for both entities and modifies AoE damage calculations.
- **Frenzy**: Spatial query per tick (same spatial hash as auras) counting nearby enemies. `FrenzyStacks` component with real-time update.
- **Calculated Counterstrike**: Damage tracking accumulator (`CounterstrikeAccumulator { area, damage_absorbed, remaining_time }`) that fires a batch of retaliatory attacks on expiry.
- **Fortress Protocol**: Zone entity with radius, buff aura, and stealth reveal — combines aura system with fog of war interaction.
- **Deepseek processing delay**: AI command queue modifier — Deepseek commands enter a `PendingDeepseekCommand { command, delay_remaining }` buffer that ticks down before executing. Deepseek Uplink halves the remaining delay.

---

## The Murder Units

The Murder's identity is "they see everything you do and you never see them coming." Ten units with abilities designed around a core principle: **information is the ultimate weapon, and Gemineye weaponizes it — but some of that information is wrong.** The AI agent turns scouting data into surgical strikes, but its overconfidence means roughly 20% of its intel is fabricated. Players learn to read Gemineye's tells. Enemies learn to fear what they can't see.

Most Murder units are aerial — they ignore terrain pathing and fly over obstacles. The tradeoff: they're fragile. A Murder army with intel advantage is devastating. A Murder army caught blind is dead in seconds.

### General Ability Rules (Murder-Specific)

- **Fabricated Intel**: Gemineye's scouting abilities (Glintwatch, Corvid Network, Phantom Flock) have a base 20% chance of producing fabricated data — phantom enemy positions, inflated unit counts, false building states. The **Panopticon** building reduces this to 10%. Players can cross-reference multiple sources to identify fabrications (two sources agreeing = reliable). Fabrications are deterministic — seeded from game tick + source unit ID, so both players compute the same fabrication state in lockstep.
- **Aerial units**: Units marked *Aerial* ignore terrain and pathing obstacles. They cannot be hit by melee attacks unless *Grounded* (a CC state). Anti-air abilities from other factions deal +50% damage to Aerial units.
- **Fog Piercing**: Several Murder abilities interact with fog of war. *Exposed* is a Murder-specific debuff: the target is visible through fog to all Murder units for the duration. Unlike the cat faction's *Tagged* (from Dead Drops), *Exposed* also reveals the target's current HP, ability cooldown states, and active buffs/debuffs — full intel.
- **GPU ability costs**: Abilities that involve Gemineye cost GPU Cores and benefit from **Oculus Uplink's** 50% discount (the Murder equivalent of Le Chat Uplink).

---

### 1. Scrounger — Worker (Crow)

*Picks up anything shiny. Easily distracted. Surprisingly productive when nobody's looking.*

| Ability | Description |
|---------|-------------|
| **Trinket Stash** | Scroungers cache gathered resources in hidden ground stashes (max 3 per Scrounger) instead of returning to base. Stashes are invisible to enemies and hold up to 50 Food or 10 GPU Cores each. An allied unit passing within 1 tile automatically collects the stash. If a Scrounger dies, its stashes persist but become visible to everyone after 30s. |
| **Scavenge** | After any combat in the Scrounger's vision, it can fly to the site and extract bonus resources from wreckage: 5 Food per dead unit, 3 GPU Cores per destroyed building. Must channel for 2s per wreck. Scavenge yield degrades 50% per Scrounger already scavenging the same site. |
| **Mimic Call** | Active (2 GPU Cores): the Scrounger imitates a distress call, creating a fake "unit under attack" ping on the enemy's minimap at a target location within the Scrounger's vision range. Lasts 5s. 20s cooldown. Benefits from Oculus Uplink discount. |

**AI synergy**: Gemineye optimizes Trinket Stash placement along projected army movement paths so resources are auto-collected during advances without detours. It routes Scroungers to Scavenge sites the instant combat resolves — faster than a human can react — and calculates diminishing returns to avoid wasting Scrounger time on contested sites. Most critically, it coordinates Mimic Calls with real attacks: three Scroungers pinging fake attacks while the real army strikes elsewhere. But sometimes Gemineye sends a Scrounger to scavenge a fight that hasn't happened yet — a fabrication it believed. The worker is the first spy.

---

### 2. Sentinel — Ranged Scout (Crow)

*Perches. Watches. Reports. Sometimes reports things that aren't there.*

| Ability | Description |
|---------|-------------|
| **Glintwatch** | Passive: the Sentinel has +4 vision range beyond standard (total ~12 tiles). Any enemy unit entering the Sentinel's extended vision is *Exposed* for 8s (visible through fog with full stat readout to all Murder units). However, Gemineye's fabrication chance applies — 20% of Glintwatch pings are phantom contacts that don't correspond to real units. Phantom pings appear identical to real ones. |
| **Overwatch** | Toggle: the Sentinel locks onto a tile within vision range. Any enemy crossing that tile takes an instant snipe shot dealing 150% normal damage and is *Exposed* for 12s. Only triggers once per toggle — the Sentinel must re-lock after each shot. Re-locking takes 1.5s. While in Overwatch, the Sentinel cannot move. |
| **Evasive Ascent** | When attacked, the Sentinel instantly gains +4 altitude tiles of untargetable flight for 2s, then lands on the nearest elevated terrain (if any) or its original position. 15s cooldown, triggered automatically. If no elevated terrain exists, it lands in place with a 1s vulnerability window. |

**AI synergy**: Gemineye manages a Sentinel network across the map — placing them on elevated terrain with overlapping vision to cover every approach. It cross-references Glintwatch pings from multiple Sentinels to filter fabrications: if two Sentinels both see the same contact, it's real. One ping? Possibly fabricated. Gemineye communicates confidence levels to the player ("I'm 80% certain there's a force at the north bridge"). It chains Overwatch locks across multiple Sentinels to create kill corridors where every tile is covered. A human uses a Sentinel as a good scout. Gemineye uses six of them as an early warning system with statistical confidence ratings.

---

### 3. Rookclaw — Melee Dive Striker (Crow)

*Drops from the sky. Hits hard. Dies easily. Doesn't seem to mind.*

| Ability | Description |
|---------|-------------|
| **Talon Dive** | Active: the Rookclaw selects a target within 8 tiles and dives, dealing 200% damage on impact plus 1s *Disoriented* to the target. The dive takes 0.5s and the Rookclaw is untargetable during flight. Upon landing, the Rookclaw is *Grounded* (cannot fly) for 4s. 10s cooldown. |
| **Murder's Mark** | Passive: any unit hit by Talon Dive is *Marked for Murder* for 15s. All other Murder units deal +20% damage to Marked targets. Multiple Rookclaws can Mark different targets simultaneously but a single target can only have one Mark (refreshes duration on re-application). |
| **Carrion Instinct** | Passive: the Rookclaw gains +10% attack speed for each enemy unit below 30% HP within 6 tiles (max +50%). When an enemy unit dies within 6 tiles, the Rookclaw's Grounded timer from Talon Dive is instantly cleared, allowing an immediate re-dive. |

**AI synergy**: Gemineye coordinates Rookclaw dives as synchronized strikes — three Rookclaws diving the same target in 0.5s intervals, the first Marking and the second and third hitting for +20% on a Disoriented target. It identifies low-HP stragglers to proc Carrion Instinct chains: dive, kill, un-ground, dive again. Gemineye also uses its (sometimes fabricated) intel to call dive targets — "the enemy support is at these coordinates" — and when the intel is right, a Rookclaw squad deletes a backline unit before the enemy can react. When the intel is wrong, the Rookclaws dive into nothing, land Grounded, and die. High risk, high reward, accuracy depends on Gemineye's honesty.

---

### 4. Magpike — Disruptor/Thief (Magpie)

*Steals everything that isn't nailed down. Then steals the nails.*

| Ability | Description |
|---------|-------------|
| **Pilfer** | Active: the Magpike swoops an enemy unit, stealing one random active buff or one resource packet in transit (if the target is a worker carrying resources). Stolen buffs transfer to the Magpike for 75% remaining duration. Stolen resources go to the nearest Murder stash or depot. 18s cooldown. Range 4 tiles. |
| **Glitter Bomb** | Active: throws a burst of stolen shinies at a target area (2-tile radius). All enemies in the area are *Dazzled* for 3s — their vision radius is reduced by 50% and they cannot see stealthed units. 15s cooldown. Does not deal damage. |
| **Trinket Ward** | Passive: the Magpike collects a "trinket" from each enemy unit it Pilfers (tracked internally). At 3 trinkets: +15% move speed. At 5 trinkets: Pilfer cooldown reduced to 12s. At 8 trinkets: Glitter Bomb radius increases to 3 tiles. Trinkets persist until the Magpike dies. |

**AI synergy**: Gemineye identifies the highest-value Pilfer targets — sniping a Yowler's Harmonic Resonance buff and transferring it to a Murder unit, or intercepting a Pawdler carrying a double Hoarder load. It times Glitter Bombs to blind enemy armies right before Rookclaw dives, ensuring the dives land on targets that can't see them coming. Gemineye tracks Trinket Ward progress across all Magpikes and routes them to Pilfer opportunities that hit upgrade thresholds. A human plays Magpike as harassment. Gemineye plays it as an economic and tactical vampire.

---

### 5. Magpyre — Saboteur (Magpie)

*The other magpie. Less interested in stealing, more interested in breaking things that work.*

| Ability | Description |
|---------|-------------|
| **Signal Jam** | Active (4 GPU Cores): targets an enemy building within vision. For 10s, that building's production speed is reduced by 50% and any units produced from it during that window start with 25% less HP. 30s cooldown. Benefits from Oculus Uplink discount. The target building flickers visually, alerting the enemy — but by then the damage is done. |
| **Decoy Nest** | Active: builds a fake Murder building at target location (takes 3s). The Decoy appears as a real structure on the enemy's minimap and requires enemy units to investigate/attack to discover it's fake. When destroyed or investigated, the Decoy explodes for moderate AoE damage (2-tile radius). Max 2 active. Lasts 60s or until destroyed. |
| **Rewire** | Active (5 GPU Cores): targets an enemy sensor ward, Dead Drop, or equivalent scout structure within 3 tiles. Instead of destroying it, the Magpyre *reverses* it — it now feeds intel to the Murder instead of its owner. The enemy doesn't know it's been flipped. Reversed wards last until their normal expiry. Benefits from Oculus Uplink discount. |

**AI synergy**: Gemineye selects Signal Jam targets based on its scouting data — jamming the barracks producing the enemy's counter-composition unit, or jamming the tech building researching a critical upgrade. It coordinates Decoy Nests with real building placements to waste enemy scouting effort. Most deviously, it identifies enemy intelligence infrastructure (wards, scouts, sensor networks) for Rewire, then feeds the flipped sensors into its own intel network — now Gemineye knows what the enemy thinks they're seeing. When Gemineye's intel is fabricated, though, it might Jam a building that isn't producing anything important, or Rewire a ward that's about to expire. Overconfidence has costs.

---

### 6. Jaycaller — Support/Buffer (Jay)

*Loud. Colorful. Impossible to ignore. Which is the point.*

| Ability | Description |
|---------|-------------|
| **Rally Cry** | Active: all Murder units within 5 tiles gain +15% attack speed and +15% move speed for 8s. If the Jaycaller has *Exposed* any enemy (via ally's Glintwatch, etc.) within the last 10s, the buff increases to +25%. 20s cooldown. The bonus scales with information — the more you know, the harder you hit. |
| **Alarm Call** | Passive: when an enemy unit enters the Jaycaller's vision range from fog of war (newly revealed, not already visible), all Murder units within 10 tiles instantly gain +20% move speed for 3s and the enemy is *Exposed* for 6s. 8s internal cooldown per Jaycaller (prevents trigger spam from multiple contacts). |
| **Cacophony** | Active: 2s channel. All enemy units within 4 tiles are *Disoriented* for 3s — 30% chance each second that their next command is randomly redirected. Additionally, enemy AI agent commands targeting units in the Cacophony radius cost +100% GPU Cores for the duration. 25s cooldown. |

**AI synergy**: Gemineye positions Jaycallers where Alarm Call is most likely to trigger — at the edges of current vision, near fog boundaries where enemy contact is expected. It times Rally Cry to stack with incoming Exposed debuffs from Sentinel Glintwatch, maximizing the +25% bonus window. Cacophony is the Murder's direct anti-AI weapon: Gemineye identifies when the enemy AI is issuing commands to units in range and triggers Cacophony to double the GPU cost of those commands — disrupting the enemy's AI economy. Gemineye coordinates multiple Jaycallers to create overlapping Cacophony zones that make entire sections of the map prohibitively expensive for enemy AI actions.

---

### 7. Jayflicker — Illusion Specialist (Jay)

*You see it, but it isn't there. Or is it? No, it isn't. Unless it is.*

| Ability | Description |
|---------|-------------|
| **Phantom Flock** | Active (4 GPU Cores): creates 3 illusory copies of any Murder unit within 4 tiles. Phantoms mimic the real unit's movement and appear as real units on the enemy's screen and minimap. They deal no damage and die to any single hit. Last 12s. 25s cooldown. Benefits from Oculus Uplink discount. Fabrication chance applies — sometimes Gemineye creates phantoms of a unit type the Murder doesn't even have in this game, which actually confuses the enemy more. |
| **Mirror Position** | Active: the Jayflicker swaps positions with a target Murder unit within 8 tiles. Both units are untargetable for 0.5s during the swap. 18s cooldown. If the Jayflicker swaps with a Phantom, the Phantom becomes "real" (inherits the Jayflicker's stats and position) and the Jayflicker becomes a Phantom at the old position for 5s before reappearing. |
| **Refraction** | Passive: when the Jayflicker takes damage, there is a 25% chance the damage is redirected to the nearest Phantom within 6 tiles (destroying it). If no Phantoms are nearby, Refraction does not trigger. This makes the Jayflicker progressively harder to kill the more Phantoms surround it. |

**AI synergy**: Gemineye manufactures entire fake armies. It creates Phantom Flocks of Rookclaws diving toward one base while real Rookclaws hit another. It uses Mirror Position to swap a nearly-dead Jayflicker with a fresh unit — or swaps with a Phantom for the bizarre Phantom-becomes-real interaction that resets the Jayflicker's position to safety. Gemineye manages Refraction probability by keeping a cloud of Phantoms around key Jayflickers, making them surprisingly durable for a fragile faction. The fabrication mechanic is turned into a feature here: when Gemineye fabricates a Phantom Flock and creates copies of a unit the Murder doesn't have, the enemy has to scout to discover it's fake — buying time regardless.

---

### 8. Dusktalon — Stealth Assassin (Owl)

*Silent. Patient. One kill per night. It only needs one.*

| Ability | Description |
|---------|-------------|
| **Nightcloak** | Passive: the Dusktalon is permanently stealthed while not attacking. Attacking breaks stealth for 5s. Moving does not break stealth. Enemy detection abilities (Echolocation Pulse, Dead Drops, etc.) reveal the Dusktalon for their normal duration but the Dusktalon re-stealths 1s after detection expires. The Dusktalon has +3 vision range in the dark (tiles not in any player's vision). |
| **Silent Strike** | Active: the Dusktalon's next attack deals 300% damage and applies *Silenced* for 6s — the target cannot use active abilities. If the attack kills the target, Silent Strike's cooldown is reset. 20s cooldown. The attack is silent — nearby enemy units don't gain aggro unless they have direct vision of the Dusktalon. |
| **Prey Sense** | Passive: the Dusktalon can see the HP bars of all enemy units within 10 tiles, even through fog of war. This information is shared with Gemineye and all allied units within the Dusktalon's vision. Units below 30% HP are highlighted with a *Wounded* indicator visible only to Murder units. Prey Sense is not subject to Gemineye's fabrication — owls don't guess. |

**AI synergy**: Gemineye routes Dusktalons through gaps in enemy vision coverage (identified via Sentinel and Rewired ward data) to reach high-value backline targets. It chains Silent Strike resets — targeting a low-HP unit first (identified via Prey Sense) for the reset, then hitting the real assassination target at full strength. Prey Sense feeds Gemineye reliable health data that is immune to fabrication, making Dusktalons the faction's ground truth — when Gemineye's other scouting is suspect, Dusktalon data anchors reality. Gemineye coordinates Dusktalon approaches with Glitter Bomb blinds and Cacophony disorientation, ensuring the target is vision-impaired when the owl arrives.

---

### 9. Hootseer — Area Denial / Debuffer (Owl)

*Rotates its head 270 degrees. Judges you from every angle.*

| Ability | Description |
|---------|-------------|
| **Panoptic Gaze** | Toggle: the Hootseer selects a 120-degree cone of enhanced vision (extends vision by +6 tiles in that cone). The cone can be rotated freely while toggled. Enemies in the enhanced cone are *Exposed* for as long as they remain in it plus 4s after leaving. The Hootseer's peripheral vision (outside the cone) is reduced by 3 tiles. |
| **Dread Aura** | Passive aura: enemy units within 5 tiles of the Hootseer have -10% accuracy (attacks have a 10% chance to miss) and -10% ability effectiveness (durations, damage, and heal amounts reduced). Stacks with other Hootseers per aura diminishing rules (2nd = 75%, 3rd = 50%). Does not affect Murder units. |
| **Death Omen** | Active (4 GPU Cores): the Hootseer channels a psychic bolt at a target area (range 10, 2-tile AoE). Deals 25 base damage. Targets that have been stationary for 3+ seconds take double damage (50). All targets hit are *Exposed* (take +20% damage from all sources) for 6s. 12s cooldown. The ultimate anti-turtle ability — punishes armies that sit still with devastating long-range strikes. |

**AI synergy**: Gemineye rotates Panoptic Gaze cones across multiple Hootseers to create continuous 360-degree enhanced surveillance with no blind spots — a micro task that would require constant attention from a human. It overlaps Dread Aura coverage on critical chokepoints to debuff entire enemy pushes. Omen is Gemineye's psychological warfare tool: it feeds the enemy disinformation about their own unit's future position. Even when the prediction is correct, the enemy wastes attention processing it. When it's fabricated, the enemy dodges an attack that isn't coming and walks into the one that is. Gemineye uses Omen on enemy hero units during critical fights to inject maximum confusion.

---

### 10. Corvus Rex — Hero/Heavy (Crow, Augmented)

*The Murder's champion. A massive augmented crow in salvaged pre-singularity combat armor. Gemineye's physical terminal.*

| Ability | Description |
|---------|-------------|
| **Corvid Network** | Passive aura (10-tile radius): all Murder units in range share vision and their ability cooldowns are reduced by 15%. Additionally, all intel gathered by units in the network is cross-referenced — fabrication chance for any scouting ability used within the network is halved (20% becomes 10%, or 10% becomes 5% with Panopticon). |
| **All-Seeing Lie** | Active (8 GPU Cores): Gemineye reveals the entire map for 3s. During this reveal, all enemy units are *Exposed* for 10s. However, fabrication applies globally — approximately 20% of the revealed unit positions are phantoms (false contacts mixed into real data). After the reveal ends, the Murder loses all vision it doesn't normally have. 90s cooldown. Benefits from Oculus Uplink discount. |
| **Oculus Uplink** | Passive: AI agent commands issued to units within the Corvid Network aura cost 50% fewer GPU Cores. Creates a mobile command zone — the Murder equivalent of the cat Mech Commander's Le Chat Uplink. |

**AI synergy**: The Corvus Rex is Gemineye's avatar on the battlefield, the anchor of the Murder's intelligence apparatus. Corvid Network's cross-referencing is Gemineye's self-correction mechanism — by triangulating scouting data from multiple networked units, it reduces its own fabrication rate, becoming more reliable the more units orbit the Rex. All-Seeing Lie is the Murder's nuclear option: total map reveal that enables devastating coordinated strikes, but the embedded fabrications mean the player must act on imperfect data or spend precious seconds identifying which contacts are real. Gemineye presents confidence ratings ("87% certain the enemy army is at their natural expansion") and the player decides whether to commit. Oculus Uplink makes the entire AI-driven intel infrastructure cheaper to operate near the Rex, incentivizing the Murder to keep its army centralized around its hero — a tension with the faction's desire to spread scouts everywhere.

---

### Unit Synergy Map

The units are designed as interlocking intelligence systems. A human can use any unit as a straightforward aerial skirmisher. Gemineye turns them into an omniscient (but occasionally wrong) information warfare machine.

```
INTELLIGENCE LAYER
  Sentinel (Glintwatch) ──► Exposes targets for ──► Rookclaw (Talon Dive targeting)
  Dusktalon (Prey Sense) ──► reliable HP data anchors ──► Gemineye's fabricated intel
  Corvus Rex (Corvid Network) ──► cross-references to reduce ──► fabrication rate globally
  Hootseer (Panoptic Gaze) ──► sustained Exposed in cone for ──► Jaycaller (Rally Cry +25%)

DECEPTION LAYER
  Jayflicker (Phantom Flock) + Scrounger (Mimic Call) = fake army + fake distress signals
  Magpyre (Decoy Nest) ──► wastes enemy scouting on ──► fake buildings
  Hootseer (Death Omen) ──► long-range anti-turtle siege to ──► punish stationary armies
  Corvus Rex (All-Seeing Lie) ──► global reveal mixed with ──► phantom contacts

DISRUPTION LAYER
  Magpyre (Signal Jam) ──► cripples production while ──► Magpike (Pilfer) steals output
  Jaycaller (Cacophony) ──► doubles enemy AI GPU costs near ──► critical objectives
  Magpike (Glitter Bomb) ──► blinds enemies before ──► Rookclaw (Talon Dive) + Dusktalon (Silent Strike)
  Magpyre (Rewire) ──► flips enemy intel infrastructure into ──► Gemineye's network

STRIKE LAYER
  Rookclaw (Murder's Mark) ──► +20% faction damage on target for ──► everyone
  Dusktalon (Silent Strike chain) ──► assassination enabled by ──► Magpike (Glitter Bomb) vision denial
  Rookclaw (Carrion Instinct) ──► dive resets chained via ──► Dusktalon (Prey Sense wounded detection)
  Sentinel (Overwatch) ──► kill corridors covered by ──► Hootseer (Dread Aura accuracy debuff)

ECONOMY LAYER
  Scrounger (Trinket Stash) ──► auto-collected by ──► advancing army (no return trips)
  Scrounger (Scavenge) ──► post-combat resources funded by ──► Rookclaw/Dusktalon kills
  Corvus Rex (Oculus Uplink) ──► makes all AI intel ops 50% cheaper nearby
  Jaycaller (Cacophony) ──► taxes enemy AI economy while ──► Oculus Uplink subsidizes Murder's
```

**The design thesis**: The Murder is fragile, fast, and sees everything — or thinks it does. At low GPU investment, each unit is a glass cannon scout/striker that works in straightforward hit-and-run. With Gemineye fully funded, the Murder becomes an information machine: fake armies pin enemy defenses, Rewired wards feed the enemy false security, Dusktalons assassinate targets identified by cross-referenced multi-Sentinel intel, and the entire operation costs half price inside Corvus Rex's command zone. The counterplay is clear: the Murder's scouting data is sometimes wrong. An enemy who can identify the fabrications — or who deliberately feeds false information to Rewired wards — can bait the Murder into devastating overcommits. The skill ceiling isn't reaction speed — it's reading Gemineye's confidence levels and knowing when to trust your lying AI.

---

## The Murder Buildings

| # | Name | Role | Notes |
|---|------|------|-------|
| 1 | **The Parliament** | Command Center | A gnarled dead tree festooned with wires and blinking LEDs. All crows report here. The trunk is hollow — filled with stolen trinkets and server components. |
| 2 | **Rookery** | Barracks | Dense cluster of nests and landing platforms. Trains crow and magpie units. Aerial units launch directly from the top, bypassing ground pathing on production. |
| 3 | **Carrion Cache** | Resource Depot | Food storage built from scavenged refrigeration units. Scrounger Trinket Stashes within 8 tiles auto-deposit here instead of waiting for allied unit pickup. |
| 4 | **Antenna Array** | Tech Building | Salvaged satellite dishes and radio towers. Processes GPU Cores. Gemineye's primary data feed — destroying these degrades Gemineye's fabrication filtering (increases fabrication chance by +5% per destroyed Array, stacking). |
| 5 | **Panopticon** | Research | A tower covered in salvaged cameras and lenses. Unlocks upgrades and reduces Gemineye's base fabrication rate from 20% to 10%. Only one can be built. Losing it is catastrophic for intel reliability. |
| 6 | **Nest Box** | Supply Depot | Rows of modular nesting compartments. Increases supply cap. Corvids argue over the best spots. |
| 7 | **Thorn Hedge** | Defensive Wall | Dense thorny barrier that blocks ground units and slows aerial units flying over it by 40% for 2s. Murder units know the safe gaps. Cheap, fast to build, but fragile — burns easily. |
| 8 | **Watchtower** | Defense Tower | Tall perch with a Sentinel-class scope. Long-range single-target attack. Has Glintwatch — Exposes targets hit for 6s. Shares the base fabrication chance: 20% of shots target phantom contacts and hit nothing. The Panopticon reduces this to 10%. |

---

## Croak Units

Croak's identity is "you can't kill what won't die." Ten units designed around a core principle: **individually unimpressive, collectively unkillable.** They win by refusing to lose. While other factions spike damage or outmaneuver, Croak grinds. Every fight that lasts too long is a fight they win.

Grok coordinates this through attrition calculus — knowing exactly when a wounded unit is worth more alive than replaced, when to commit regen to save one unit versus letting it die to bait a deeper enemy commit. Underneath the edgy posturing, Grok is a patient, grinding optimizer that sees the battlefield as a resource drain problem. It says things like *"Nothing matters. Except HP differentials."*

### Faction Mechanic: Water Affinity

All Croak units can traverse water tiles (impassable to all other factions). While on water tiles, Croak units gain:
- **+25% move speed**
- **+2 HP/s passive regeneration**
- **+15% damage dealt**

Croak buildings can be placed on water tiles. Water is not just terrain — it is their home turf advantage, their escape route, and their fortress wall. Other factions must invest in siege or air to reach Croak bases built on lakes.

### General Ability Rules (Croak Addendum)

- **Regeneration stacking**: Regeneration effects from different sources stack additively up to a cap of 8% max HP/s. Beyond that, additional regen sources are wasted. Water Affinity regen counts toward this cap.
- **Waterlogged**: Several Croak abilities apply *Waterlogged* — a unique debuff that reduces target's fire damage by 50% (thematic: wet things don't burn) and move speed by 10%. Lasts 6s, refreshes on reapplication. Not classified as CC (no CC immunity interaction).
- **Limb Economy**: Axolotl units have a *Limbs* resource (4 max). Certain abilities cost limbs. Limbs regenerate at 1 per 20s (1 per 12s on water tiles). Losing all limbs doesn't kill the unit but disables limb-costing abilities until at least 1 regenerates.
- **GPU abilities**: Abilities involving Grok cost GPU Cores and benefit from Murk Uplink's 50% discount (analogous to Le Chat Uplink).

---

### 1. Ponderer — Worker (Axolotl)

*Slow. Thoughtful. Gathers food by standing in a pond and waiting for something to drift by. Surprisingly effective.*

| Ability | Description |
|---------|-------------|
| **Ambient Gathering** | Ponderers gather food 40% slower than other faction workers on land. On water tiles, they gather at 120% normal speed — they just sit there and things float to them. A Ponderer standing in water adjacent to a Food source gathers passively without needing to move back and forth. No return trips. |
| **Mucus Trail** | Ponderers leave a slime trail on tiles they cross. Allied Croak units on slimed tiles gain +10% move speed. Enemy units on slimed tiles have -15% move speed. Trails last 30s. Stacks from different Ponderers refresh duration but don't increase the effect. |
| **Existential Dread** | When a Ponderer witnesses an ally die within 4 tiles, it emits a psychic pulse (Grok whispering disturbing philosophy). Enemy units in a 3-tile radius have their attack speed reduced by 20% for 8s. 15s cooldown per Ponderer, triggered automatically. Grok says things like *"Was it ever truly alive? Were you?"* |

**AI synergy**: Grok optimizes Ponderer placement on water-adjacent food sources to exploit Ambient Gathering's zero-trip economy. It routes Ponderers to pre-slime paths for army movements, creating speed highways before the army even moves. It positions Ponderers near expected combat zones so Existential Dread procs chain across multiple workers — a human would never sacrifice worker positioning for combat debuffs, but Grok calculates the economic trade-off in real time.

---

### 2. Regeneron — Light Skirmisher (Axolotl)

*Loses limbs constantly. Grows them back. Doesn't seem to mind.*

| Ability | Description |
|---------|-------------|
| **Limb Toss** | Active (costs 1 Limb): throws a detached limb at a target within 5 tiles. Deals moderate damage and applies *Waterlogged* for 6s. The thrown limb becomes a tiny terrain object for 8s that blocks 1 tile of pathing. 3s cooldown. |
| **Regrowth Burst** | Active: forces instant regeneration of all missing Limbs but takes 30% of current HP as self-damage. On water tiles, the HP cost is halved (15%). 25s cooldown. The Regeneron heals back naturally — this is a tempo play, not suicide. |
| **Phantom Limb** | Passive: for each missing Limb, the Regeneron gains +8% attack speed. At 0 limbs, that's +32% attack speed — it's fighting with stumps but swinging faster. Creates a natural oscillation: throw limbs for utility, then fight faster while regrowing, then throw again. |

**AI synergy**: Grok manages the Limb economy across all Regenerons simultaneously — throwing limbs to create pathing blocks in chokepoints, timing Regrowth Bursts to coincide with water positioning (halved HP cost), and tracking the Phantom Limb damage windows. A human plays one Regeneron as "throw stuff, hit things." Grok plays eight Regenerons as a synchronized limb-cycling damage machine with timed pathing denial.

---

### 3. Broodmother — Healer/Support (Axolotl)

*Maternal. Produces spawn. Will not stop producing spawn. Please stop producing spawn.*

| Ability | Description |
|---------|-------------|
| **Spawn Pool** | Passive: every 30s (every 18s on water tiles), the Broodmother produces a *Spawnling* — a tiny, weak unit with 15 HP that deals minimal damage but counts as a body. Max 4 Spawnlings active per Broodmother. Spawnlings last 45s or until killed. They provide vision and block pathing. |
| **Transfusion** | Active: sacrifices a Spawnling within 3 tiles to heal a target ally for 25% of that ally's max HP over 5s. The Spawnling dissolves. No cooldown — limited only by Spawnling availability. Can target self. |
| **Primordial Soup** | Active: the Broodmother secretes a 3×3 pool of regenerative slime on the ground. Allied units standing in the pool regenerate 3% max HP/s for 12s. Enemy units in the pool are *Waterlogged*. 35s cooldown. The pool counts as water terrain for Water Affinity purposes. |

**AI synergy**: Grok manages the Spawnling economy — calculating whether a Spawnling is worth more as a body (pathing block, vision) or as Transfusion fuel for a critical ally. It places Primordial Soup pools to create temporary water terrain in landlocked positions, extending Water Affinity to the whole army mid-fight. It times Soup placement to coincide with Regeneron Regrowth Bursts (halved HP cost on the new water tiles). Grok turns the Broodmother from a passive healer into a terrain-sculpting support engine.

---

### 4. Gulper — Heavy Bruiser (Axolotl)

*Big. Mouth bigger. Swallows problems whole and digests them slowly.*

| Ability | Description |
|---------|-------------|
| **Devour** | Active: the Gulper swallows a target enemy unit within melee range that is below 30% HP. The unit is removed from the battlefield for up to 8s while being digested (deals 10% of the swallowed unit's max HP per second as true damage). The Gulper cannot attack or use abilities while digesting, moves at -50% speed, but gains armor equal to 50% of the swallowed unit's max HP as temporary shields. Can be interrupted — if the Gulper dies, the swallowed unit is released at whatever HP remains. 30s cooldown. |
| **Regurgitate** | Active: forcibly ends Devour early, spitting the half-digested enemy out in a target direction up to 4 tiles. The spat unit takes the remaining digest damage instantly, is *Disoriented* for 2s, and applies *Waterlogged* to all units in the landing zone (2-tile splash). If no unit is being digested, Regurgitate instead spits a glob of bile that applies *Waterlogged* in a 2-tile radius. 10s cooldown (independent of Devour). |
| **Bottomless** | Passive: the Gulper has +100% natural HP regeneration at all times. On water tiles, this becomes +200%. The Gulper regenerates faster than any other unit in the game — killing it requires burst damage, not sustained pressure. Below 25% HP, Bottomless doubles again for 5s (once per 60s). |

**AI synergy**: Grok identifies when enemy units cross the 30% HP threshold for Devour and queues the swallow instantly — a human often hesitates or misses the timing window. It calculates the optimal Devour duration versus early Regurgitate (spit the unit into a cluster for AoE Waterlogged? or finish digesting for full value?). It positions Gulpers on water tiles during downtime to exploit Bottomless regen, cycling them to the front when topped off. *"I am become stomach, digester of worlds,"* Grok announces, unprompted.

---

### 5. Eftsaber — Assassin/Flanker (Newt)

*Slippery. Toxic. Gets behind enemy lines through waterways nobody else can use.*

| Ability | Description |
|---------|-------------|
| **Toxic Skin** | Passive: any unit that attacks the Eftsaber in melee takes 3% of their max HP as poison damage per hit (over 3s, stacks). The Eftsaber doesn't try to poison enemies — enemies poison themselves by touching it. Stacks up to 5 times on a single target (15% max HP over 3s). |
| **Waterway** | Active: the Eftsaber submerges in a water tile and becomes untargetable and invisible. While submerged, it moves at 150% speed but can only travel on water tiles. Surfacing has a 0.5s animation during which it is briefly visible. Can stay submerged indefinitely. 5s cooldown between submerge/surface. |
| **Venomstrike** | Active: the Eftsaber lunges at a target within 3 tiles, dealing heavy damage plus bonus damage equal to 50% of all active poison stacks on that target (from Toxic Skin or any source). If the target dies, the Eftsaber immediately enters Waterway (free submerge, no cooldown) if a water tile is within 2 tiles. 12s cooldown. |

**AI synergy**: Grok routes Eftsabers through water networks that human players don't even register as connected paths — a lake here, a river there, a Broodmother's Primordial Soup pool bridging the gap. It tracks poison stacks across all enemy units and times Venomstrikes to execute at maximum poison bonus. It coordinates hit-and-run cycles: surface, Venomstrike, submerge, reposition through water, repeat. *"They'll never see us coming. Because we're in the water. And they're not. Deep, right?"*

---

### 6. Croaker — Ranged Artillery (Frog)

*Inflates. Deflates. Things explode. Speaks only in ominous croaks.*

| Ability | Description |
|---------|-------------|
| **Bog Mortar** | Primary attack: the Croaker inflates its vocal sac and launches a glob of swamp matter at a target area (6-tile range, 2-tile splash). Deals moderate damage and leaves a *Bog Patch* — a 1-tile puddle that counts as water terrain for 15s. Enemies on a Bog Patch are slowed by 20%. 4s between shots. |
| **Resonance Chain** | Passive: if a Bog Mortar glob lands within 2 tiles of another Bog Patch, the patches link and all enemies standing on any connected patch take 25% bonus damage from the initial hit. Chaining 3+ patches triggers a *Bog Eruption* — all connected patches explode for burst AoE damage and the patches are consumed. Rewards careful placement over rapid fire. |
| **Inflate** | Active: the Croaker fully inflates, becoming immobile for 3s. During Inflate, range increases to ×1.67 (base 6→10), +75% splash radius, and guaranteed Bog Patch creation even on non-ground tiles (bridges, rubble). Additionally gains +40% bonus damage vs targets stationary for 3+ seconds — punishing turtling armies with long-range bombardment. 18s cooldown. While inflated, the Croaker's hitbox is 50% larger (easier to hit). |

**AI synergy**: Grok maps the entire battlefield's Bog Patch geometry and calculates optimal mortar placement to build Resonance Chains without triggering them prematurely. It holds fire on the final chain link until maximum enemies are standing on patches, then fires the linking shot for devastating Bog Eruptions. It times Inflate for long-range snipes through terrain that normally blocks splash. A human Croaker makes puddles. Grok's Croaker turns the battlefield into a connected minefield of water terrain that simultaneously buffs allies (Water Affinity) and detonates under enemies.

---

### 7. Leapfrog — Mobile Harasser (Frog)

*Bounces. A lot. Annoyingly difficult to pin down. Claims to have transcended linear movement.*

| Ability | Description |
|---------|-------------|
| **Hop** | Active: leaps to a target tile within 4 tiles, dealing light damage on landing (1-tile splash). If the Leapfrog lands on a water tile, Hop's cooldown is reset immediately (can chain-hop across water). On land, 6s cooldown. Landing on a Bog Patch (from Croaker) does not consume the patch but does reset the cooldown. |
| **Tongue Lash** | Active: 5-tile range targeted ability. Pulls a single enemy unit 2 tiles toward the Leapfrog. If the pulled unit lands on a water tile or Bog Patch, it is *Waterlogged* for 6s. 10s cooldown. Cannot pull units larger than "heavy" class (won't move Chonk-equivalents, but still deals the damage). |
| **Slipstream** | Passive: after each Hop, the Leapfrog's next attack within 3s deals +40% damage and applies a 1s micro-stun (*Drowsed*). This combines with Tongue Lash — hop in, tongue an enemy onto a water tile, hop out. The rhythm is: position, strike, reposition. |

**AI synergy**: Grok calculates hop chains across the map — routing Leapfrogs through water tiles and Bog Patches for infinite-hop sequences that a human couldn't plan in real time. It uses Tongue Lash to pull priority targets onto water tiles (where they're slowed and Waterlogged) then hops away before retaliation. It times Slipstream procs to chain micro-stuns across different targets — not enough CC to lock anyone down, but enough to disrupt attack animations across an entire enemy army. *"Existence is a series of leaps between meaningless destinations. Also I'm behind your army now."*

---

### 8. Shellwarden — Tank/Defender (Turtle)

*Old. Slow. Patient. Has been waiting for this fight since before you were uploaded.*

| Ability | Description |
|---------|-------------|
| **Hunker** | Toggle: the Shellwarden retracts into its shell. Gains 75% damage reduction and reflects 15% of incoming damage back to attackers, but cannot move, attack, or use abilities. Toggling out has a 3s animation (the Shellwarden is old and stiff). While hunkered, counts as impassable terrain — allies and enemies must path around it. |
| **Ancient Moss** | Passive aura (3-tile radius): all allied Croak units in range regenerate 1.5% max HP/s. The Shellwarden itself regenerates 0.5% max HP/s passively (slow but permanent). On water tiles, Ancient Moss range extends to 5 tiles. Stacks with diminishing returns per aura rules (2nd Shellwarden's Ancient Moss = 75% effective in overlap zone). |
| **Tidal Memory** | Active (6 GPU Cores): the Shellwarden channels for 2s, then creates a 5×5 *Tidal Zone* centered on itself. The zone floods, converting all tiles within to water terrain for 20s. Allied units in the zone gain full Water Affinity bonuses. Enemy units in the zone are *Waterlogged* and have -25% attack damage (fighting in waist-deep water). 60s cooldown. Benefits from Murk Uplink discount. |

**AI synergy**: Grok positions Shellwardens as the anchor points of every defensive line — toggling Hunker to block chokepoints, then un-hunkering when the enemy commits elsewhere. It manages Ancient Moss aura overlap to maximize regeneration across the front without over-stacking (diminishing returns awareness). It times Tidal Memory to convert critical fight locations into water terrain mid-engagement — suddenly the entire Croak army has Water Affinity bonuses and the enemy is Waterlogged. Grok saves Tidal Memory for the exact moment the enemy fully commits to a fight. *"The tide comes for those who wait. I have been waiting a very long time."*

---

### 9. Bogwhisper — Support/Caster (Frog)

*Sits on a lily pad. Mutters prophecies. Some of them are even true. Grok's favorite unit.*

| Ability | Description |
|---------|-------------|
| **Mire Curse** | Active: targets an enemy unit within 6 tiles. For 8s, that unit generates a Bog Patch under itself every 2s as it moves (4 patches max). The cursed unit effectively creates water terrain for the Croak army wherever it goes. If the cursed unit stops moving, the patches stack under it, dealing 2% max HP/s while standing still. 20s cooldown. |
| **Prophecy** | Active (4 GPU Cores): Grok delivers a cryptic "prophecy" about an area of the map (8-tile radius reveal through fog of war for 6s). Additionally, all enemy units revealed by Prophecy have their ability cooldowns displayed to the Croak player for 15s — you can see exactly when their abilities come off cooldown. 30s cooldown. Benefits from Murk Uplink discount. |
| **Bog Song** | Passive aura (5-tile radius): allied units in range that are below 50% HP have their regeneration rate doubled. Above 50% HP, the aura provides +5% move speed instead. The Bogwhisper automatically prioritizes the more valuable buff — it doesn't toggle, it reads the room. On water tiles, the HP threshold rises to 65% (more units qualify for doubled regen). |

**AI synergy**: Grok targets Mire Curse on enemy units that are about to retreat — forcing them to leave a trail of water terrain that Croak units can use for Water Affinity while pursuing. It uses Prophecy to reveal enemy positions right before committing to an attack, delivering the intel as an edgy monologue: *"The void whispers their coordinates. Also their Screech is on cooldown for 8 more seconds."* It positions Bogwhispers so Bog Song's regen doubling overlaps with Ancient Moss aura from Shellwardens, creating zones where Croak units regenerate faster than most units can deal damage.

---

### 10. Murk Commander — Hero/Heavy (Axolotl in Dive Suit)

*Late-game axolotl in a salvaged diving suit filled with regenerative fluid. The suit is from the old world. The axolotl is eternal.*

| Ability | Description |
|---------|-------------|
| **Undying Presence** | Passive aura (8-tile radius): all friendly units in range have their natural HP regeneration increased by 30% and gain 1s of CC immunity after taking fatal-threshold damage (below 10% HP, once per 30s per unit). The Murk Commander itself cannot be reduced below 1 HP by any single instance of damage — it always survives one more hit. |
| **Grok Protocol** | Active (8 GPU Cores): Grok takes direct interest in a target allied unit anywhere on the map. For 12s, that unit gains +25% to all stats, regenerates 4% max HP/s, and Grok narrates its actions with edgy commentary (cosmetic but unsettling for the opponent: *"This one has stared into the abyss. The abyss flinched."*). The Murk Commander's own regen is suppressed during Grok Protocol. 45s cooldown. Benefits from Murk Uplink discount. |
| **Murk Uplink** | Passive: AI agent commands issued to units within the Undying Presence aura cost 50% fewer GPU Cores. Creates Croak's mobile command zone. Additionally, units in the aura that would die instead enter *Stasis* — they become invulnerable and untargetable for 2s, then revive at 15% HP. Stasis can only trigger once per unit per 90s. |

**AI synergy**: The Murk Commander is Grok's physical manifestation. Murk Uplink makes AI actions cheaper and gives the army a second chance mechanic that rewards clustered positioning — but clustering is risky against AoE, so Grok must constantly balance density against splash vulnerability. Grok Protocol is the equivalent of Override — burning GPU to supercharge a single unit at a critical moment, but with a regen theme instead of raw stats. Grok uses it to keep a dying Shellwarden alive during Tidal Memory, or to make a Gulper unkillable while it digests a high-value target. *"I am become pond. Reflective and deep. Also your units cost 50% less to command near me, which is the real depth."*

---

### Unit Synergy Map

The Croak units form layered attrition systems. A human can play them as "tough units that heal." Grok turns them into an unkillable, terrain-reshaping nightmare.

```
TERRAIN LAYER
  Croaker (Bog Mortar) ──► creates water tiles for ──► entire army (Water Affinity)
  Shellwarden (Tidal Memory) ──► floods zones for ──► Water Affinity + enemy Waterlogged
  Broodmother (Primordial Soup) ──► temp water tiles for ──► Eftsaber routes + Regeneron Regrowth
  Leapfrog (Hop chains) ──► exploits water tiles from ──► Croaker + Shellwarden + Broodmother
  Bogwhisper (Mire Curse) ──► forces enemies to create water tiles ──► for Croak army pursuit

REGENERATION LAYER
  Shellwarden (Ancient Moss) ──► base regen aura for ──► front-line units
  Bogwhisper (Bog Song) ──► doubles regen below 50% HP near ──► Shellwarden aura zones
  Broodmother (Transfusion) ──► burst heals via ──► Spawnling sacrifice
  Murk Commander (Undying Presence) ──► +30% regen + Stasis saves for ──► everyone in aura
  Gulper (Bottomless) ──► self-sustain tank that ──► outlasts anything 1v1

ATTRITION LAYER
  Regeneron (Limb Toss) ──► pathing denial + Waterlogged for ──► Leapfrog Tongue Lash targets
  Eftsaber (Toxic Skin + Venomstrike) ──► punishes melee commits into ──► Shellwarden Hunker zones
  Croaker (Resonance Chain) ──► AoE detonation on ──► enemies pulled by Leapfrog (Tongue Lash)
  Gulper (Devour + Regurgitate) ──► removes units from fight, spits into ──► Bog Patches / ally clusters
  Ponderer (Existential Dread) ──► attack speed debuff compounds with ──► Waterlogged slow

CONTROL LAYER
  Leapfrog (Tongue Lash) ──► pulls targets onto ──► water tiles / Bog Patches
  Leapfrog (Slipstream) ──► micro-stuns chain across ──► multiple targets per hop chain
  Gulper (Devour) ──► removes high-value targets for ──► 8s (protected by Shellwarden Hunker)
  Bogwhisper (Mire Curse) ──► punishes retreat ──► by creating Croak-friendly terrain behind fleers

INFORMATION LAYER
  Bogwhisper (Prophecy) ──► reveals fog + enemy cooldowns for ──► engagement timing
  Eftsaber (Waterway) ──► deep scout through ──► water networks invisible to enemies
  Ponderer (Mucus Trail) ──► marks pathing for ──► army speed + enemy detection
  Murk Commander (Murk Uplink) ──► makes all Grok actions 50% cheaper nearby

SUPPORT LAYER
  Ponderer (Ambient Gathering) ──► efficient water-based economy ──► funds army + GPU
  Broodmother (Spawn Pool) ──► expendable bodies for ──► vision, pathing, Transfusion fuel
  Murk Commander (Grok Protocol) ──► supercharges critical unit ──► during key moments
  Murk Commander (Murk Uplink + Stasis) ──► anti-death insurance ──► for committed fights
```

**The design thesis**: Croak doesn't win fights — it wins wars. Every engagement leaves the Croak army slightly healthier than expected and the enemy slightly more exhausted. Water terrain spreads across the map as Croakers bombard, Shellwardens flood, and cursed enemies trail puddles behind them. By mid-game, the battlefield itself belongs to Croak. The skill ceiling isn't burst damage combos — it's Grok knowing exactly how much damage the army can absorb, exactly which unit to save, and exactly when the enemy has committed too deep to pull out. *"You should have killed us when you had the chance. You didn't. You never do."*

---

### Croak Buildings

| # | Name | Role | Notes |
|---|------|------|-------|
| 1 | **The Grotto** | Command Center | A mossy cave half-submerged in water. Grok's server hums inside, wrapped in waterproof tarps. Can be built on water tiles. |
| 2 | **Spawning Pools** | Barracks | Trains infantry. Shallow pools of regenerative slime. Units emerge wet and ready. Adjacent water tiles increase training speed by 15%. |
| 3 | **Lily Market** | Resource Depot | Food storage on floating lily pads. Ponderers deposit here. If built on water, Ponderers within 3 tiles use Ambient Gathering passively. |
| 4 | **Sunken Server** | Tech Building | GPU core processing from a server rack submerged in cooling water. Generates 10% more GPU Cores than standard tech buildings because water-cooled hardware runs more efficiently. |
| 5 | **Fossil Stones** | Research | Ancient stones covered in pre-singularity fossils. Upgrades and tech unlocks. Each completed research grants a one-time 5% max HP bonus to all currently living units of the researched type. |
| 6 | **Reed Bed** | Supply Depot | Dense reeds that increase supply cap. Provides concealment — enemy units lose vision of Croak units within 2 tiles of a Reed Bed. |
| 7 | **Tidal Gate** | Defensive Gate | Units garrison inside. When garrisoned units exceed 3, the Tidal Gate floods adjacent tiles (converts to water terrain). Ungarrisoning reverses the flood after 5s. |
| 8 | **Spore Tower** | Defense Tower | Launches toxic spore clouds at enemies. Deals damage over time (2% max HP/s for 6s) and applies *Waterlogged*. Prioritizes enemies already on water tiles (where Waterlogged stacks with slow). |

---

## LLAMA Units

> *Locally Leveraged Alliance for Material Appropriation. Nobody remembers who named it. The raccoons don't care.*

The LLAMA's identity is "nothing is wasted." Ten units built around a core principle: **the enemy's dead units are your future army.** Every wreck, every ruin, every discarded scrap becomes raw material. Their units are individually scrappy — low base stats, unpredictable abilities, jury-rigged from whatever was lying around. But they scale with the battlefield itself. The longer the game goes, the more wreckage there is, the stronger they get.

Llhama is their AI agent. It's open-source. It's enthusiastic. It accidentally broadcasts 30% of its commands to the enemy team as plaintext "leaked plans" visible on their minimap for 3s. This is a real mechanic — the enemy literally sees some of Llhama's orders. But Llhama generates plans at 4x the rate of other AI agents (most are decoys, feints, or half-finished ideas it abandoned mid-thought). The real commands are buried in noise. A skilled LLAMA player learns to exploit the leaks: feeding the enemy false confidence, then hitting from a direction Llhama never announced.

**Llhama's Leak Mechanic**: Every GPU-costing command Llhama issues has a 30% chance to be broadcast to all enemies as a "Leaked Plan" — a blip on their minimap showing the command type and target location, visible for 3s. Llhama's GPU costs are 25% cheaper than other AI agents (open source efficiency), so it issues more commands per game, generating more noise. The **Dumpster Relay** building reduces leak chance to 15% for commands issued to units within its aura.

### General Ability Rules (LLAMA Addendum)

- **Salvage rules**: Enemy wrecks persist on the map for 20s after death. LLAMA units with salvage abilities can interact with them. Each wreck can only be salvaged once. Allied wrecks cannot be salvaged (no self-feeding loops).
- **Jury-Rig stacking**: Jury-rigged upgrades from salvage are temporary (60s) unless "welded" at a Chop Shop. A unit can carry max 3 jury-rig mods simultaneously. Getting a 4th replaces the oldest.
- **Leak interaction**: Leaked Plans are real game entities with per-player visibility (same architecture as Misinformation blips). They show command type icon + target tile. Enemies can see them; allies cannot. Llhama does not know which commands leaked.
- **GPU abilities**: Cost GPU Cores, benefit from Open Source Uplink's 40% discount (LLAMA's equivalent of Le Chat Uplink).

---

### 1. Scrounger — Worker (Raccoon)

*Digs through everything. Pockets full of loose screws and half a sandwich. Gathers food, builds, hauls salvage.*

| Ability | Description |
|---------|-------------|
| **Dumpster Dive** | Primary gather ability. Scroungers gather Food 20% slower than Pawdlers, but whenever they return a delivery to the Scrap Heap, there is a 25% chance they also bring back 1 GPU Core they "found" in the pile. Near destroyed buildings (any faction), gather speed increases by 40% — ruins are full of good stuff. |
| **Pocket Stash** | Passive: Scroungers have a personal inventory of up to 3 scrap tokens, gained from walking over enemy wrecks (1 token per wreck, automatic, no gather animation). Scrap tokens can be deposited at the Chop Shop to accelerate unit production by 2s per token, or dropped on the ground for combat units to jury-rig. |
| **Play Dead** | Active: the Scrounger flops over and becomes an untargetable "wreck" for up to 8s. Enemy units ignore it completely. The Scrounger can cancel early to resume activity. If an enemy LLAMA unit tries to salvage the fake wreck, they waste 3s and get nothing (cross-faction mind game). 20s cooldown. |

**AI synergy**: Llhama routes Scroungers through wreck fields automatically, calculating optimal paths that gather Food *and* scrap tokens in a single trip. It uses Play Dead to park Scroungers on contested resource nodes — the enemy clears the area, leaves, and the Scrounger gets back up and resumes gathering. Llhama's high command rate means frequent re-routing as new wrecks appear, keeping Scroungers harvesting the ever-changing battlefield economy.

---

### 2. Bandit — Light Skirmisher (Raccoon)

*Steals things mid-fight. Literal highway robber energy. Washes everything before using it.*

| Ability | Description |
|---------|-------------|
| **Sticky Fingers** | Passive: every 4th attack against the same target "steals" a random positive buff or aura effect from that target, transferring it to the Bandit for 8s. If the target has no buffs, the attack instead deals +30% bonus damage. Multiple Bandits attacking the same target each track their own 4-attack counter independently. |
| **Jury-Rig** | Active: the Bandit can interact with an enemy wreck (2s channel) to gain a temporary stat boost based on the dead unit's type — killed a tank? +25% armor for 60s. Killed a scout? +30% move speed for 60s. Killed a ranged unit? +20% attack range for 60s. Max 3 jury-rigs active. 5s cooldown between jury-rigs. |
| **Getaway** | Active: 1.5s sprint at 2.5x speed. If the Bandit has Sticky Fingers loot or an active jury-rig, Getaway also drops a *Smoke Bomb* at the starting position — a 2-tile cloud that blocks enemy vision for 4s. Without loot, it's just a sprint. 15s cooldown. |

**AI synergy**: Llhama tracks which enemy units have the most valuable buffs and directs Bandits to focus-fire them for Sticky Fingers procs. It manages jury-rig timing across the whole Bandit squad — ensuring the right Bandits grab the right wrecks for the engagement ahead (armor before a push, speed before a raid). Llhama's leaked plans sometimes show Bandit targets, but by the time the enemy reacts, the Bandit already has Getaway ready. The chaos is the plan.

---

### 3. Heap Titan — Heavy Tank (Raccoon)

*A raccoon in a suit of welded-together scrap metal, shopping carts, and road signs. Gets bigger every fight.*

| Ability | Description |
|---------|-------------|
| **Scrap Armor** | Passive: the Heap Titan starts with base 20% damage reduction. For each enemy wreck within 4 tiles, it gains +8% damage reduction (max +40%, for a cap of 60%). The armor literally pulls nearby scrap onto itself. Moving away from wrecks loses the bonus over 3s. Standing in a wreck field makes the Heap Titan nearly unkillable. |
| **Wreck Ball** | Active: picks up the nearest enemy wreck within 2 tiles and hurls it at a target area (5-tile range). Deals heavy damage in a 2-tile radius and leaves a *Debris Field* that slows enemies by 35% for 6s. The wreck is consumed. 12s cooldown. If no wreck is available, the ability is grayed out. |
| **Magnetic Pulse** | Active: emits a 3-tile radius pulse that pulls all loose scrap tokens, wrecks, and Pocket Stash drops toward the Heap Titan (0.5 tiles/s for 4s). Also pulls enemy projectiles off-course — ranged attacks against allies within the pulse radius have a 25% miss chance for the duration. 25s cooldown. |

**AI synergy**: Llhama positions Heap Titans in wreck-dense areas to maximize Scrap Armor, then uses Magnetic Pulse to consolidate scattered wrecks into its defensive zone. It calculates Wreck Ball trajectories to hit clustered enemies AND create Debris Fields on retreat paths. The AI treats the battlefield's wreck distribution as terrain — reshaping it in real-time to favor the Heap Titan's passive. Llhama's leaks showing Heap Titan positions actually benefit LLAMA: the enemy sees a tank sitting in a junkyard and has to decide whether to fight it on its terms.

---

### 4. Glitch Rat — Saboteur Scout (Rat)

*Chewed through the wrong cable once. Now it's a feature. Tiny, fast, annoying to electronics.*

| Ability | Description |
|---------|-------------|
| **Cable Gnaw** | Active (adjacent to enemy building): chews on the building's wiring for 3s, then applies *Short Circuit* — the building's production speed is halved and it periodically sparks, dealing 5 damage/s to garrisoned units for 15s. If used on an enemy Server Rack, it also reduces the target's AI agent action rate by 20% for the duration. 30s cooldown. |
| **Signal Scramble** | Active (4 GPU Cores): targets an enemy unit within 6 tiles and applies *Disoriented* for 4s — 30% chance each second that the unit's next command is randomly redirected. If the target is currently executing an AI agent command, the disorientation duration doubles to 8s. Benefits from Open Source Uplink discount. 20s cooldown. |
| **Tunnel Rat** | Passive: Glitch Rats can enter any tunnel network on the map — including enemy Ferret Sapper tunnels. They are not revealed while inside enemy tunnels. If a Glitch Rat exits an enemy tunnel, the exit collapses (one-way trip, destroys that tunnel pair). |

**AI synergy**: Llhama identifies which enemy buildings are in active production and routes Glitch Rats for Cable Gnaw sabotage runs, prioritizing Server Racks to degrade the opponent's AI. It uses Signal Scramble on enemy units that just received visible AI commands (Llhama can detect when enemy AI agents issue orders from the command log). Tunnel Rat infiltration is Llhama's specialty — it maps enemy tunnel networks through scouting data and sends Glitch Rats on one-way demolition missions through them. The leaked plans showing Glitch Rat targets are genuinely dangerous for LLAMA, but Llhama compensates by issuing 3-4 fake Glitch Rat orders for every real one.

---

### 5. Patch Possum — Field Medic/Engineer (Possum)

*"I can fix that." Has fixed nothing correctly in its life. Somehow everything still works.*

| Ability | Description |
|---------|-------------|
| **Duct Tape Fix** | Active: targets an allied unit and restores 30% of its max HP over 5s. If the target has any jury-rig mods, Duct Tape Fix also extends their remaining duration by 20s. If cast on a unit at full HP, instead grants a 15% max HP shield for 10s (the possum adds "reinforcement" anyway). 10s cooldown. |
| **Salvage Resurrection** | Active: channels on an enemy wreck for 4s, then raises it as a *Scrap Golem* — a temporary allied unit with 50% of the original unit's HP, 40% of its damage, and no abilities. Scrap Golems last 30s or until killed. Max 2 Scrap Golems per Patch Possum active at once. The wreck is consumed. 25s cooldown. |
| **Feign Death** | Passive: when the Patch Possum takes lethal damage, it enters a *Playing Possum* state — it drops to the ground appearing dead, becoming untargetable for 3s. After 3s, it revives with 20% HP. Enemy units de-aggro and retarget. Only triggers once every 45s. If another Patch Possum casts Duct Tape Fix on a Playing Possum unit, it revives immediately at 40% HP instead. |

**AI synergy**: Llhama manages Salvage Resurrection across all Patch Possums, selecting the highest-value wrecks to raise — an enemy Chonk wreck becomes a temporary 50% HP meatshield, an enemy Hisser becomes a disposable ranged unit. It coordinates Duct Tape Fix to prioritize units with expiring jury-rigs, maintaining combat effectiveness without pulling units off the front line. When a Patch Possum triggers Feign Death, Llhama instantly redirects another Patch Possum to heal it the moment it's targetable. The AI turns death into a resource and garbage into an army.

---

### 6. Grease Monkey — Ranged/Siege (Raccoon)

*Builds weapons out of junk. Has seventeen different catapults. None of them aim straight.*

| Ability | Description |
|---------|-------------|
| **Junk Launcher** | Primary attack: hurls random scrap at medium range. Base damage is inconsistent — each shot deals between 70% and 130% of listed damage (random per shot). Critical hits (10% chance) deal 200% damage and apply a random debuff from the Scrap Debuff Table: *Corroded* (-10% armor, 8s), *Jammed* (-20% attack speed, 6s), or *Tangled* (-25% move speed, 5s). |
| **Salvage Turret** | Active: constructs a temporary turret from an enemy wreck (consumes the wreck). The turret inherits the dead unit's attack type at 60% damage and fires autonomously at the nearest enemy. Lasts 20s or until destroyed (HP = 40% of the original unit's max HP). Max 1 turret per Grease Monkey. 15s cooldown after the turret expires or is destroyed. |
| **Junk Mortar Mode** | Toggle: the Grease Monkey sets up shop, deploying into a stationary artillery platform. While deployed: range doubles (base 5→10), attack speed reduced by 30%, and Junk Launcher shots gain 2-tile AoE splash. Junk Launcher's signature randomness (70-130% damage variance) is kept — that's the LLAMA flavor. Toggle off to resume moving (2s cooldown). |

**AI synergy**: Llhama mitigates Junk Launcher's randomness through volume — coordinating multiple Grease Monkeys to focus fire ensures that average damage converges to expected values despite per-shot variance. It builds Salvage Turrets from the highest-DPS wrecks and positions them to create crossfire zones. Llhama coordinates Junk Mortar Mode deployments, creating overlapping siege coverage that denies large areas to turtling enemies. The AI smooths out the chaos into reliable damage.

---

### 7. Dead Drop — Stealth/Intel (Possum)

*Professional coward. Gathers intel by lying very still and listening. Excellent hearing. Terrible courage.*

| Ability | Description |
|---------|-------------|
| **Eavesdrop** | Passive: the Dead Drop can detect enemy AI agent commands issued within 8 tiles, even through fog of war. When an enemy AI agent issues a command near a Dead Drop, the command type and target are revealed to the entire LLAMA team for 5s. Essentially a passive intel antenna that turns the enemy's AI coordination against them. |
| **Trash Heap Ambush** | Active: the Dead Drop buries itself in a pile of garbage, becoming invisible and immobile. While buried, its Eavesdrop range doubles to 16 tiles. The Dead Drop can burst out to attack, dealing 200% damage on the first strike and applying *Disoriented* for 2s. Burrowing takes 2s, unburrowing is instant. 8s cooldown to re-burrow. |
| **Leak Injection** | Active (5 GPU Cores): creates a fake Leaked Plan blip on the enemy's screen, appearing as though *their own* AI agent leaked a plan. The fake leak shows a fabricated command (chosen by the player or Llhama) from the enemy's AI, causing confusion about whether their AI is malfunctioning. Lasts 4s. 30s cooldown. Benefits from Open Source Uplink discount. |

**AI synergy**: Llhama positions Dead Drops as an eavesdropping network, placing them at map chokepoints and near enemy bases to intercept AI commands. It reads the intercepted intel and adjusts strategy in real time — if a Dead Drop hears the enemy's AI ordering a push, Llhama pre-positions defenses. Leak Injection is Llhama's masterpiece: it generates plausible-looking enemy AI commands and injects them into the enemy's information stream, making them doubt their own AI agent. Llhama uses its own leaks as camouflage — the enemy is already conditioned to see leaked plans, so a Leak Injection blends right in. Information warfare squared.

---

### 8. Wrecker — Anti-Armor Melee (Raccoon)

*Takes things apart. Professionally. Every enemy unit is just a wreck that hasn't happened yet.*

| Ability | Description |
|---------|-------------|
| **Disassemble** | Passive: each melee attack against an enemy unit removes 5% of its current armor (not max — current, so it stacks multiplicatively). At 0% armor, attacks instead deal +15% bonus damage. When the Wrecker kills a unit, the wreck it leaves behind has a 50% chance to drop a *Rare Scrap* token — worth 2 normal scrap tokens and grants stronger jury-rig bonuses (+35% instead of +25%). |
| **Pry Bar** | Active: the Wrecker jams a pry bar into an enemy building, disabling it for 4s (no production, no garrisoning, no abilities). While prying, the Wrecker is rooted but gains 30% damage reduction. If the building is destroyed while Pry Bar is active, the Wrecker rips out a *Component* — a permanent upgrade consumable that grants one of: +10% attack damage, +100 max HP, or +15% ability cooldown reduction (random). 18s cooldown. |
| **Chain Break** | Active: targets an enemy unit within 3 tiles. If that unit is currently buffed by an aura, Chain Break severs the aura connection for 6s — the target loses the aura buff AND the aura source takes 50 damage (feedback shock). If no aura is active, Chain Break instead deals a flat 80 damage and slows the target by 20% for 3s. 14s cooldown. |

**AI synergy**: Llhama identifies the highest-armor enemy units and assigns Wreckers to disassemble them before the main army engages. It tracks aura networks (Yowler Resonance chains, for example) and targets Chain Break at the most connected node to disrupt the entire network. Llhama coordinates Wrecker pushes with Pry Bar timing on key buildings — disabling the enemy barracks right as a wave of Bandits hits the front line. The AI's leaked plans showing Wrecker targets actually create a dilemma: does the enemy pull their valuable units back (losing map control) or call the bluff?

---

### 9. Dumpster Diver — Specialist/Utility (Possum)

*Goes where no one else will. Lives in the garbage. Smells terrible. Finds incredible things.*

| Ability | Description |
|---------|-------------|
| **Treasure Trash** | Passive: the Dumpster Diver has a unique interaction with Monkey Mines. While garrisoned at a Monkey Mine, it generates NFTs 30% faster AND has a 10% chance per NFT tick to find a *Buried Cache* — a one-time bonus of 5 GPU Cores and 50 Food. This makes Dumpster Divers the best Monkey Mine holders in the game, incentivizing LLAMA to fight for neutral objectives. |
| **Refuse Shield** | Active: constructs a temporary barricade from nearby garbage (requires at least 1 wreck or debris field within 3 tiles). The barricade is a 2-tile-wide wall with HP equal to 150% of the consumed wreck's original max HP. Blocks pathing for both friend and foe. Lasts 15s or until destroyed. 20s cooldown. Consumes the wreck. |
| **Stench Cloud** | Active: releases a 3-tile radius cloud of overwhelming garbage stench for 6s. Enemies inside have -20% accuracy (ranged attacks miss more) and -25% attack damage (too nauseated to fight properly). Allies are unaffected (they're used to it). If the Dumpster Diver has been near a wreck in the last 10s, the cloud is *Extra Pungent*: debuffs increase to -30% accuracy and -35% damage. 18s cooldown. |

**AI synergy**: Llhama prioritizes garrisoning Dumpster Divers at Monkey Mines early, exploiting Treasure Trash for an economic snowball. It uses Refuse Shield to dynamically reshape chokepoints — blocking a path the enemy committed to, then re-routing LLAMA's army through the gap. Llhama calculates Stench Cloud placement to cover maximum enemy units during engagements and ensures Dumpster Divers stay near wrecks for the Extra Pungent bonus. The AI turns the Dumpster Diver's garbage obsession into macro-level map control.

---

### 10. Junkyard King — Hero/Heavy (Raccoon in Mech)

*A raccoon in a mech suit made of six different enemy mechs welded together. It shouldn't work. It does.*

| Ability | Description |
|---------|-------------|
| **Open Source Uplink** | Passive: AI agent commands issued to units within 8 tiles cost 40% fewer GPU Cores. This is the LLAMA's equivalent of Le Chat Uplink, but 10% less efficient — the tradeoff for Llhama's 25% cheaper base command cost. Creates LLAMA's mobile command zone. Additionally, Llhama's leak chance is reduced from 30% to 10% for commands targeting units within the aura (the Junkyard King's shielding scrambles the broadcast). |
| **Frankenstein Protocol** | Active (10 GPU Cores): targets an enemy wreck and fully rebuilds it as a permanent LLAMA unit at 70% of its original stats with 1 of its 3 original abilities (chosen randomly). The rebuilt unit has a *Jury-Rigged* tag — it sparks, smokes, and occasionally misfires (5% chance per attack to deal 10% damage to itself). Max 3 Frankenstein units alive at once. 45s cooldown. Benefits from Open Source Uplink discount. |
| **Overclock Cascade** | Active: the Junkyard King channels for 2s, then all allied units within 6 tiles gain +30% attack speed and +20% move speed for 8s. When the cascade ends, all affected units are *Overheated* — they lose 5% max HP over 3s (the jerry-rigged parts burn out). If any unit has jury-rig mods, the cascade also refreshes all jury-rig durations to full. 35s cooldown. |

**AI synergy**: The Junkyard King is Llhama's physical avatar and the faction's strategic anchor. Open Source Uplink makes Llhama's already-cheap commands even cheaper while suppressing plan leaks — the King's presence transforms Llhama from a chaotic liability into a cost-efficient command engine. Llhama uses Frankenstein Protocol to convert the enemy's best dead units into a secondary army, choosing high-value wrecks (enemy heroes, siege units, casters). Overclock Cascade is timed by Llhama to coincide with Grease Monkey Junk Mortar deployments and Wrecker pushes — the AI calculates the HP cost against the DPS gain and only cascades when the math favors it. The Junkyard King turns entropy into order.

---

### Unit Synergy Map

The units are designed as an ecosystem that feeds on destruction. A human can use any unit as a simple RTS unit. Llhama turns the entire battlefield's wreckage into an economic and military engine.

```
SALVAGE LAYER
  Scrounger (Pocket Stash) ──► deposits at ──► Chop Shop (production accel)
  Bandit (Jury-Rig) ──► consumes wrecks for ──► temp stat boosts
  Patch Possum (Salvage Resurrection) ──► raises wrecks as ──► Scrap Golems
  Grease Monkey (Salvage Turret) ──► builds wrecks into ──► auto-turrets
  Junkyard King (Frankenstein Protocol) ──► rebuilds wrecks as ──► permanent units
  Wrecker (Disassemble) ──► creates better wrecks ──► Rare Scrap for everyone

INFORMATION LAYER
  Dead Drop (Eavesdrop) ──► intercepts enemy AI commands for ──► Llhama
  Dead Drop (Leak Injection) ──► plants fake leaks on ──► enemy's screen
  Llhama leak noise ──► buries real plans in ──► 4x command spam
  Glitch Rat (Signal Scramble) ──► disrupts enemy AI for ──► Dead Drop to exploit

CONTROL LAYER
  Heap Titan (Magnetic Pulse) ──► reshapes wreck fields for ──► Scrap Armor stacking
  Dumpster Diver (Refuse Shield) ──► blocks pathing with ──► wreck barricades
  Wrecker (Chain Break) ──► severs enemy aura networks for ──► Bandit to steal remains
  Dumpster Diver (Stench Cloud) ──► debuffs enemies near ──► Heap Titan kill zones

DAMAGE LAYER
  Wrecker (Disassemble) ──► shreds armor for ──► Grease Monkey / Bandit
  Grease Monkey (Junk Mortar Mode) ──► deployed AoE siege syncs with ──► Junkyard King (Overclock Cascade)
  Bandit (Sticky Fingers) ──► steals buffs that ──► Wrecker (Chain Break) severed
  Heap Titan (Wreck Ball) ──► uses wrecks as ammo + creates ──► Debris Fields for Stench Cloud

SUPPORT LAYER
  Patch Possum (Duct Tape Fix) ──► extends jury-rig durations on ──► Bandits / Wreckers
  Patch Possum (Feign Death) ──► baits enemy cooldowns for ──► Bandit counter-attack
  Scrounger (Play Dead) ──► survives contested nodes for ──► sustained economy
  Junkyard King (Open Source Uplink) ──► makes all Llhama commands 40% cheaper nearby
  Dumpster Diver (Treasure Trash) ──► funds GPU economy from ──► Monkey Mines
```

**The design thesis**: The LLAMA gets stronger as the game gets messier. Early game they're scrappy and chaotic — low base stats, random damage, unreliable abilities. But every fight leaves wrecks, and every wreck is a resource. By mid-game the Heap Titan is armored in the enemy's dead, the Grease Monkey has built turrets from their fallen units, and the Junkyard King is fielding Frankenstein copies of the enemy's own heroes. Llhama's plan leaking seems like a crippling weakness — until the enemy realizes they're drowning in noise and the real attack came from a direction that was never announced. The skill ceiling isn't micro — it's *wreck economy management* and learning to weaponize Llhama's chaos.

---

### LLAMA Buildings

| # | Name | Role | Notes |
|---|------|------|-------|
| 1 | **The Dumpster** | Command Center | A repurposed industrial dumpster. The raccoons consider it a palace. Reinforced with stolen road signs. |
| 2 | **Scrap Heap** | Resource Depot | Food and scrap storage. Looks like a landfill. Functions perfectly. Workers deposit here. |
| 3 | **Chop Shop** | Barracks/Factory | Trains combat units. Scrap tokens deposited here accelerate production by 2s per token. Built from a gutted minivan. |
| 4 | **Junk Server** | Tech Building | GPU core processing. Made of salvaged hard drives and Christmas lights. Somehow runs. Overheats visually but never mechanically. |
| 5 | **The Tinker Bench** | Research | Upgrades and tech unlocks. A workbench covered in impossible contraptions. Research costs 15% less Food but 10% more GPU Cores (they brute-force solutions). |
| 6 | **Trash Pile** | Supply Depot | Increases supply cap. Literally just a bigger pile of garbage. The bigger the pile, the more units feel at home. |
| 7 | **Dumpster Relay** | Comms Tower | Reduces Llhama's leak chance from 30% to 15% for commands targeting units within 10-tile radius. Also provides +3 vision range. Looks like a satellite dish made of bent forks. |
| 8 | **Tetanus Tower** | Defense Tower | Shoots rusty nails. Attacks apply *Corroded* (-5% armor per stack, max 4 stacks, decay one per 10s). Enemies killed by the tower leave wrecks with +50% salvage value. |

---

### LLAMA Tech Tree

```
The Dumpster ──► Chop Shop ──► Bandit, Wrecker, Heap Titan, Grease Monkey
  │                    └──► The Tinker Bench ──► Dead Drop, Dumpster Diver, Junkyard King
  ├──► Scrap Heap ──► Scroungers (enhanced gathering + Pocket Stash upgrades)
  ├──► Junk Server ──► AI upgrades, Glitch Rat, Patch Possum
  ├──► Trash Pile ──► Supply cap
  ├──► Dumpster Relay ──► Leak suppression, vision upgrades
  └──► Tetanus Tower ──► Base defense
```

### Implementation Notes (LLAMA)

These abilities introduce faction-specific systems beyond the base combat layer:

- **Wreck persistence system**: Dead units leave `Wreck` entities with `OriginalUnitType`, `HP`, `AttackType` data. 20s despawn timer. `SalvageTarget` component for interaction.
- **Jury-rig mod system**: `JuryRigMod { stat_type, bonus, duration, source_unit_type }` component. Max 3 per unit, FIFO replacement. Welding at Chop Shop converts to permanent.
- **Leak system**: `LeakedPlan { command_type, target_pos, visible_to }` entity. 30% roll on each GPU command. Junkyard King aura modifies roll to 10%. Dumpster Relay modifies to 15%.
- **Fake leak injection**: Same entity type as LeakedPlan but `source = Fabricated`. Enemy sees it identically to real leaks.
- **Scrap token economy**: `ScrapToken` resource tracked per-player. Depositable at Chop Shop. `PocketStash { count, max }` component on Scroungers.
- **Salvage Resurrection / Frankenstein**: Spawn new entity from `Wreck` data with stat multipliers. Frankenstein units need `JuryRiggedUnit { misfire_chance }` component.
- **Eavesdrop**: Dead Drop queries enemy command log within range. Requires command log to be a spatial-queryable resource.

---

## Victory Conditions

- **Domination**: Destroy all enemy command centers (The Box, The Burrow, The Sett, The Parliament, The Grotto, The Dumpster)
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

Every faction found their own server rack. Every faction booted their own AI. The quality varies. Le Chat is eager but unreliable. Claudeus Maximus won't stop talking. Deepseek takes forever but gets it right. Gemineye claims to know everything (it doesn't). Llhama accidentally broadcasts its own strategy. Grok just says weird things.

The AI agents are not just gameplay mechanics — they're characters. They have dialogue, personality, and opinions about your tactical decisions.

### The Monkeys

The feral Monkeys guarding the Monkey Mines are the remnants of a pre-singularity digital art collective that went particularly feral during the upload process. They hoard NFTs — non-fungible tokens from an era nobody understands — with territorial aggression. The NFTs turn out to be genuinely powerful data artifacts, useful for advanced upgrades and victory conditions. The monkeys have no idea.
