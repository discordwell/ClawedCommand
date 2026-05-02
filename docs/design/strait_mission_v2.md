---
name: Strait Mission V2 Design
description: Complete tactical design for Kell's strait clearing mission — enemy composition, drone mechanics, player strategy space, and intended solutions
type: project
---

## Strait Mission V2 — Full Tactical Design

### Core Insight: Drones Are General-Purpose
Our drones are the universal tool. They patrol, bomb, intercept shaheeds, and escort. The mission is about scripting them well.

### Player Unit: Drones
- **Flare ability**: auto-use, on recharge. Counters enemy AA — one AA can suppress one drone, but 3 drones overwhelm 1 AA's flare capacity and close in for bombing.
- **Bomb**: drones can attack ground targets when close enough.
- **Intercept**: a drone held at base can intercept incoming shaheeds, saving Patriot missiles for actual missiles.
- **Vision**: radius scales with drone_vision compute slice (flow economy, instant switching).

### Allocation Exploit (Intended Secret)
Since allocation changes are instant with no delay, savvy players can pulse: allocate 100% drone_vision for a few ticks → full 4-tile radius on all drones → switch to 100% zero_day → drones go blind but build progresses fast. Effectively gets benefits of both by cycling. Satellite is less efficient — reserved for when enemy AA balls up and drones can't safely scout.

### Enemy Forces
1. **AA (Anti-Air)**: Dangerous to drones, but extremely limited in number. Can suppress/kill lone drones. Countered by 3-drone swarms (overwhelm flares). If clustered tight → airstrike them. If 3 AA spread-but-near → use zero-day Blind to disable all 3, then swarm simultaneously.
2. **Soldiers**: Only dangerous at extremely close range. Basically speedbumps with good scripting — drones bomb from standoff distance. Only a threat if scripts get drones too close.
3. **Suicide drones (Shaheed)**: Fly toward ships/base. Can be intercepted by our drones (scripted intercept) or by Patriots (wasteful). Key scripting gap: holding a drone at base for shaheed intercept saves Patriots for missiles.
4. **Missile launchers**: Require setup and teardown time. High priority targets — visible during setup phase, vulnerable. Hide-launch-relocate cycle. Must be found and killed.

### Enemy Tactics
- Waves triggered by ship launches.
- If no ships are running, enemy attacks the home base with missiles + shaheeds.
- Enemy gets reinforcements over time but finite total force.
- Mix of: probing with shaheeds, suppressing with AA, firing missiles at ships, sending soldiers forward.

### Intended Player Strategies

**"Slow but sure" (easy mode):**
1. Hold all boats at base.
2. Tank incoming missiles/shaheeds at base using Patriots + drone intercept.
3. Systematically clear enemy positions with drone swarms + airstrikes + zero-days.
4. Enemy gets reinforcements but once their stuff is blown up, sweep the stragglers.
5. Launch all boats at once through a cleared strait.

**"Aggressive" (hard/fast):**
1. Launch boats early, escort with drones.
2. Manage combat and escort simultaneously.
3. Higher risk, faster completion.

**"Pulse exploit" (clever):**
1. Rapidly cycle compute allocation for max vision + max zero-day build.
2. Build powerful exploits fast while maintaining situational awareness.
3. Use zero-days surgically, then sweep.

### Scripting Gaps (designed for discovery)
- Drone at base for shaheed intercept (vs. wasting Patriots)
- Pulse allocation exploit
- Coordinated 3-drone swarm tactics against AA
- Prioritizing launchers during their setup phase
- Holding boats until safe

### Patriot Interceptors
- Finite pool at base.
- Baseline behavior: shoot down anything that gets close (missiles AND shaheeds).
- Smart scripting: reserve Patriots for missiles only, use drones to intercept shaheeds.
- If Patriots run out, missiles hit base/ships unopposed.

**Why:** This defines the complete tactical layer for implementation. The mission should be beatable by iterative Lua scripting without human input.
**How to apply:** All systems (drone flares, shaheed intercept, enemy AI waves, boat holding) must be implemented. The Lua script API needs bindings for all of these.
