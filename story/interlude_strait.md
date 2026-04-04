# Interlude: The Strait

> *Between Act 3 and Act 4 — Dream Sequence Part 3*
> *Stylistic divergence: DEFCON-inspired drone warfare*
> *Setting: A narrow maritime strait, modern-day military operations center*
> *Arc: Kell Fisher wakes from the code sword dream and executes his battle plan*

---

## Design Vision

### Narrative Context

Commander Kell Fisher, US Navy, has just crashed out at his desk (dream_office) and dreamt of a code sword (dream_lake). He wakes up and immediately goes to work — not the busywork grind of the office dream, but the thing he's actually good at: running a drone warfare operation through screens and code.

This mission shows who Kell was before he became an otter. Clinical. Precise. Viewing war as an optimization problem. The DEFCON aesthetic reinforces this — everything is abstracted into dots on a screen, radar sweeps, and cooldown timers. The people dying on both sides are invisible. Kell likes it that way.

Narratively, this is the first time the player uses the Lua/voice system in gameplay. The "code sword" from the lake dream manifests here as Kell's ability to write scripts and issue voice commands to his drone fleet. The player is learning the system that will carry them through Acts 4-5, but framed as Kell's military command style — the same dehumanizing technological mastery he bragged about in the office scene.

### The Mission

**Setting**: A long, narrow strait (think Hormuz or Taiwan Strait). Top-down/isometric view with DEFCON-style visual treatment — dark background, glowing lines for coastlines, radar sweep overlays, unit icons instead of sprites.

**Player Objective**: Protect a convoy of oil tankers transiting the strait from hostile coastal defenses. Tankers move autonomously west-to-east along a shipping lane. If too many are destroyed, you fail.

**Enemy Objective**: Destroy the tankers using shore-launched anti-ship missiles and kamikaze drones. The enemy operates from concealed positions along the hostile (northern) coast, using a hide-launch-relocate doctrine.

### Core Mechanic: Compute Budget

The player has a **compute budget** — a shared resource pool that regenerates slowly. It's split between three capabilities:

#### 1. Drone Vision (Default, Cheap)
- Patrol drones provide local, moving vision circles along the strait
- Efficient — low compute cost per tick
- Vulnerable — enemy AA drones can shoot them down, creating blind spots
- When a drone dies, its patrol zone goes dark until you reassign coverage
- **Voice/Lua integration**: player scripts drone patrol patterns, waypoints, and reaction behaviors

#### 2. Satellite Vision (Backup, Expensive)
- Can target any point on the map for temporary high-resolution vision
- Much higher compute cost than drones
- Use case: when enemy concentrates fire and kills your local drones, satellite fills the gap
- Has a cooldown/sweep delay — not instant
- Reveals more information than drones (can see camouflaged units)

#### 3. Zero-Day Exploits (Strategic, Slow-Build)
- Compute invested during lulls to "build" exploits
- Take many ticks to develop but provide powerful one-shot effects when deployed:
  - **Spoof**: Make enemy missiles target decoys instead of tankers
  - **Blind**: Disable enemy radar/sensors in an area for a duration
  - **Hijack**: Turn an enemy drone to your side temporarily
  - **Brick**: Permanently destroy an enemy launcher (if you can see it)
- Building 0-days is interrupted if you need to reallocate compute to vision during a crisis
- Strategic tension: do you invest in 0-days during quiet moments, or keep compute liquid for emergencies?

### Enemy Behavior (Cat-and-Mouse)

The enemy uses a **hide-launch-relocate** cycle:
1. **Setup** (vulnerable): Mobile launchers move from hidden positions to firing positions along the coast. Visible if you have vision.
2. **Launch** (committed): Fire anti-ship missiles at tankers. Missiles are trackable and can be intercepted by your interceptor drones, but interceptors are finite.
3. **Retreat** (fast): Launchers relocate to new hidden positions after firing. If you catch them here, easy kills.

Enemy also has:
- **AA drones**: Hunt your patrol drones to create vision gaps before missile launches
- **Decoy launchers**: Set up fake firing positions to waste your interceptors and attention
- **Escalation**: As the mission progresses, enemy launches become more coordinated, with diversionary attacks to pull your vision one way while the real strike comes from another

### Interceptor Economy

- You have a **finite pool of interceptor drones** (replenished slowly)
- Interceptors auto-engage incoming missiles if they're in range, but each interception costs one interceptor
- If the enemy depletes your interceptors in one zone, the next missile salvo hits tankers unopposed
- Strategic choice: spread interceptors evenly, or concentrate near the most threatened tanker?

### Win/Lose

- **Win**: Sufficient tankers transit the strait safely (e.g., 8 of 12)
- **Lose**: Too many tankers destroyed (e.g., fewer than 6 survive)
- **Bonus objective**: Complete a specific 0-day chain (foreshadows the "code sword" mastery)

### Visual Style

DEFCON-inspired overlay on top of the existing isometric engine:
- **Dark blue/black background** with glowing coastline contours
- **Radar sweep** animation on drone vision areas
- **Dot icons** for units (no detailed sprites — everything is abstracted through screens)
- **Missile trails** as bright arcing lines
- **Explosion flashes** when missiles hit
- **Terminal-green text** for status readouts and compute budget display
- The whole thing should feel like looking at a military C2 (command and control) screen

### Dialogue & Characterization

Kell issues commands with cold efficiency. Voice lines during the mission reinforce his worldview:
- On successful intercept: *"Splash one. Next."*
- On tanker hit: *"Unacceptable. Reallocate interceptors to sector [X]."*
- On 0-day deployment: *"Payload delivered. They won't even know what happened."*
- On enemy AA killing drones: *"They're adapting. Good. So are we."*
- When things get hairy: *"This is what we trained for. Stay clinical."*

Rex Harmon occasionally chimes in from a secondary console with human reactions that contrast Kell's detachment.

### Lua/Voice Integration

This is the player's **tutorial** for the Lua + voice system, disguised as military command:
- Player can write drone patrol scripts (Lua)
- Player can voice-command drone reallocation
- The "code sword" is literally Kell typing commands into a terminal
- Scripts persist — good scripts written here can be adapted for Act 4+ RTS gameplay
- Voice commands map to drone operations: "patrol", "intercept", "scan", "exploit"

### Thematic Resonance

The mission deliberately mirrors the future RTS gameplay but from the "wrong" perspective. Kell is doing exactly what the player will do as Kelpie — managing units through an abstract interface, making life-and-death decisions through a screen. But here, the targets are human ships and the enemies are human fighters defending their coast.

When Kell becomes Kelpie and commands cat/mouse/badger armies, the player should feel the echo: *you've done this before. You were good at it then too. Does it feel different with animals?*

---

## Required New Assets

### Graphics (DEFCON-style, minimal)
- Strait map background (dark, glowing coastlines) — 1 large background or tileable terrain
- Radar sweep overlay shader/animation
- Drone icon (friendly) — small glowing dot with range circle
- Drone icon (enemy) — different color dot
- Missile trail effect (bright arc line)
- Tanker icon — larger dot/ship silhouette moving through lane
- Launcher icon (enemy, when visible) — blinking dot on coast
- Explosion flash effect
- Terminal/HUD overlay frame (green-on-dark text areas)
- Compute budget bar UI element

### Audio (stretch)
- Radar ping/sweep sound
- Missile launch warning tone
- Intercept confirmation beep
- Explosion (distant, muffled — heard through headphones)
- Ambient: military operations center hum, keyboard clicks, radio static

### Code
- New `DreamSceneType::Strait` variant
- Strait-specific systems (drone patrol, missile simulation, interceptor logic, compute budget)
- DEFCON visual overlay renderer (or shader)
- Lua bindings for drone commands
- Voice command mappings for drone operations
- Mission RON file: `assets/campaign/dream_strait.ron`
- Tanker convoy AI (simple west-to-east path following)
- Enemy AI (hide-launch-relocate cycle with escalating difficulty)
