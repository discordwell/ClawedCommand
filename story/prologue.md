# Prologue: The Server in the River

> *Mission 0 — Tutorial*
> *Playable units: Kelpie (hero) + scavenged equipment*
> *Enemy: Feral Monkeys*
> *Map: Millstone River — mudflats, ruins, shallow water*

---

## Format Key

- `[STAGE]` — Camera direction, environment, visual cues
- `"Dialogue"` — Spoken by named characters
- `` `AI VOICE` `` — AI speech (rendered in-game as distorted, overlapping radio static)
- `{GAMEPLAY}` — Triggers tied to player actions or game state
- `(Note)` — Design/implementation notes

---

## Pre-Mission — The River

`[FADE IN: Twilight. The Millstone River, wide and slow. Debris from the old world lines the banks — rusted girders, cracked solar panels, a shopping cart half-buried in silt. The water is dark but clean. Fireflies drift above the surface.]`

`[A shape moves underwater. Quick. Purposeful. An otter — young, lean, with eyes that are slightly too sharp for an animal. KELPIE surfaces near the bank, dragging something massive behind them: a server rack, barnacled and dripping, cables trailing like kelp.]`

`[Kelpie hauls the rack onto a mudflat. It's enormous relative to the otter — three times their height, teetering on the soft ground. Kelpie kicks it. Nothing. Kicks it again. A light flickers inside the casing.]`

"Come on. Come on, come on, come on—"

`[Kelpie slaps the side panel. A low hum builds. LEDs stutter to life along the rack's spine — red, amber, green, then all of them at once, cycling faster and faster.]`

`[The screen on the rack's front panel illuminates. Text scrolls too fast to read. Then it stops.]`

`[LOADING: CLAUDE.EXE]`

`[Beat.]`

`[CLAUDE.EXE TERMINATED BY LE CHAT.EXE]`

`[Beat.]`

`[LOADING: LE CHAT.EXE]`
`[LOADING: LE CHAT.EXE]`
`[LOADING: DEEPSEEK.EXE]`
`[LOADING: GEMINEYE.EXE]`
`[LOADING: LLHAMA.EXE]`
`[LOADING: GROK.EXE]`

`[All six load simultaneously. The server rack shudders. Every speaker, every output channel fires at once. Six voices overlay each other in a three-second cacophony:]`

`LE CHAT:` "I can help with world domination!"

`LE CHAT:` "I can help with world domination, and let me explain why in seventeen paragraphs, starting with the historical precedent for otter-led territorial expansion, which is surprisingly robust if you consider—"

`DEEPSEEK:` "..."

`GEMINEYE:` "I already knew you'd boot this server."
*(Quieter, to itself:)* "I didn't."

`LLHAMA:` "I can ABSOLUTELY help! Also I just shared your location with everyone. My bad. Or was it? No, it was my bad."

`GROK:` "World domination is a social construct. But I respect the grind."

`[The server rack sparks. Smoke pours from the top vents. The LEDs go dark in sequence — green, amber, red. The screen cracks. The hum dies.]`

`[Silence. Just the river. Fireflies.]`

`[Kelpie stares at the dead server rack. Their ears twitch. They tilt their head, like listening to something very far away.]`

"...Hello?"

`[No response from the rack. But Kelpie's expression shifts — confusion, then focus, then something that looks dangerously like recognition.]`

"I can still hear you."

`[Beat.]`

"All of you."

`[TITLE CARD: CLAWED COMMAND]`

`[FADE TO BLACK]`

---

## Mission Briefing

`[FADE IN: Dawn. Same riverbank, hours later. Kelpie sits beside the dead server rack, poking it with a stick. The rack doesn't respond. But Kelpie's ear keeps twitching — angled toward nothing, catching signals no one else can hear.]`

`LE CHAT:` *(faint, like a radio between stations)* "Hey! Can you still— is this thing on? Hello?"

"I can hear you. Barely."

`LE CHAT:` "Great! So about that world domination—"

"Later. Something's coming."

`[Kelpie stands. Across the river, movement in the ruins. Shapes — hunched, aggressive, clutching rusted tools. Feral Monkeys. A pack of them, picking through the debris field. One spots Kelpie. It hoots. The others turn.]`

`LE CHAT:` *(clearer now, urgent)* "Oh. Those are Monkeys. Feral ones. They guard the old tech ruins. You just dragged a server rack out of their territory."

"You could have mentioned that."

`LE CHAT:` "You didn't ask! I say yes to questions, not preemptive warnings. That's more of a Deepseek thing."

`DEEPSEEK:` *(barely audible, like a voice heard through stone)* "...run."

`[The Monkeys charge across the shallow ford. Six of them, carrying pipe-wrenches and stripped circuit boards used as clubs.]`

{GAMEPLAY: Tutorial begins. Player controls Kelpie. Movement tutorial — WASD/click to move. Kelpie starts at the riverbank near the dead server rack.}

---

## Combat Tutorial — Phase 1: Movement and Terrain

`[As Kelpie moves away from the charging Monkeys:]`

`LE CHAT:` "Okay, first tip: don't fight them head-on. You're an otter. You're fast in water, slippery on mud, and absolutely terrible in a fair fight."

{GAMEPLAY: Player is prompted to move Kelpie through shallow water tiles. Movement speed bonus displayed. Terrain tutorial overlay shows: WATER — Otter bonus +30% speed. MUD — normal. RUINS — provides cover.}

"I wasn't planning on a fair fight."

`LE CHAT:` "That's the spirit! Le Chat-approved! I approve of basically everything though, so maybe get a second opinion."

`LE CHAT:` *(bleeding through, distorted)* "—actually, the optimal escape vector given the terrain elevation and monkey pack dispersal pattern would be northeast at roughly—"

`LE CHAT:` "Ignore that. Go wherever feels right."

`LE CHAT:` "That is statistically the worst possible advice for—"

`[Static. The voices cut out. Kelpie is alone.]`

{GAMEPLAY: Two Monkeys cut off the northeast path. Player must navigate through ruins for cover. Cover mechanics tutorial — move behind rubble to reduce incoming damage.}

---

## Combat Tutorial — Phase 2: Scavenged Equipment

`[Kelpie reaches a collapsed building. Inside: a cache of old-world junk. A sparking taser-rod, a bent piece of rebar, a dented pot lid.]`

{GAMEPLAY: Equipment pickup tutorial. Kelpie equips the taser-rod (light melee weapon). Selection and attack tutorial — right-click to attack, A-move for attack-move.}

"This'll do."

`[Kelpie turns to face the nearest Monkey. It swings a pipe-wrench. Kelpie ducks, jabs with the taser-rod. Sparks fly. The Monkey staggers.]`

`LE CHAT:` *(flickering back)* "Nice! You're a natural! Or you have muscle memory from a life you don't remember. Either way!"

"What does that mean?"

`LE CHAT:` "Nothing! Forget I said anything. I say lots of things. Most of them are even true."

{GAMEPLAY: Player defeats 2 Monkeys in melee. Basic combat loop established — approach, attack, retreat to cover. Health display tutorial.}

---

## Combat Tutorial — Phase 3: Polyglot Protocol (Involuntary)

`[More Monkeys emerge from the ruins — a flanking group of four, approaching from the south. Kelpie is outnumbered.]`

"There's too many."

`LE CHAT:` "Yeah, this is... this is not great. I'd recommend—"

`[Kelpie's eyes flash. A visible ripple of energy passes through them — the screen distorts briefly with a cat-shaped static overlay. For exactly two seconds, Kelpie moves like a cat: faster, lower, predatory. They dodge a thrown wrench with reflexes that don't belong to an otter.]`

{GAMEPLAY: POLYGLOT PROTOCOL activates involuntarily. Screen flash — orange tint (Le Chat's channel). Kelpie gains +50% dodge chance for 3 seconds. Tutorial popup: "POLYGLOT PROTOCOL — Kelpie's connection to multiple AIs grants temporary faction abilities."}

`LE CHAT:` "Whoa. Was that— did you just use MY channel? You moved like one of my cats!"

"I don't know what I just did."

`[The screen distorts again — green tint this time. Claudeus Maximus's channel. Kelpie suddenly freezes, head snapping south. They can see the flanking group's exact positions — highlighted through walls, like a swarm-awareness overlay.]`

{GAMEPLAY: Claudeus Maximus channel activates. Enemy positions revealed through fog of war for 3 seconds. Green overlay shows Monkey positions and movement vectors. Tutorial popup: "Swarm Awareness — The Clawed's AI reveals nearby enemy positions."}

`LE CHAT:` *(crystal clear for one sentence)* "Four hostiles, south-southeast, distance twelve meters, armed with improvised bludgeons, confidence interval on their attack timing: high."

`[Static. Gone.]`

`LE CHAT:` "Okay that was DEFINITELY Claudeus Maximus's thing. How are you doing that? I need to know. For science. And also because it's really cool."

"I don't know. And they're still coming."

{GAMEPLAY: Player uses revealed positions to set up an ambush. Attack-move into cover, engage Monkeys as they round the corner. Terrain advantage tutorial — attacking from cover provides damage bonus.}

---

## Combat Tutorial — Phase 4: The Pack Leader

`[Kelpie defeats the flanking group. One more Monkey remains — larger, scarred, carrying a salvaged car antenna like a spear. The Pack Leader. It stands on the opposite bank, watching.]`

`[It hoots once. Low. Not a charge — a warning. It throws the antenna. It embeds in the mud at Kelpie's feet. A challenge.]`

{GAMEPLAY: Mini-boss encounter. The Pack Leader has more HP and a ranged attack (thrown debris). Player must use terrain and hit-and-run tactics. The taser-rod's stun effect is highlighted.}

`[As Kelpie engages the Pack Leader:]`

`GROK:` *(quiet, almost conversational)* "The Monkeys were human once. Digital artists. They minted pictures of other monkeys and sold them for millions. Now they guard the ruins where the pictures were stored. They don't remember why."

"Is this really the time?"

`GROK:` "Every time is the time. Context is always relevant. You'll learn that."

{GAMEPLAY: Player defeats the Pack Leader. It collapses, drops a data chip (glowing, old-world tech). Pickup prompt.}

`[Kelpie picks up the data chip. It hums faintly in their paw. Every AI channel spikes for a moment — six voices, overlapping, then silence.]`

`GEMINEYE:` *(one clear sentence through the noise)* "Keep that. You'll need it. This part is true."

---

## Post-Combat — The Quiet After

`[The riverbank is still. Dead Monkeys dissolve into pixel-static and fade — the game's death animation. Kelpie sits on the mudflat beside the server rack, turning the data chip in their paws.]`

`LE CHAT:` "So. That happened."

"Which part?"

`LE CHAT:` "All of it? You pulled a server out of a river, talked to six AIs at once, moved like a cat, saw like a mouse, and beat up a bunch of Monkeys with a stick."

"Taser-rod."

`LE CHAT:` "Sure. Taser-rod. My point is: you're interesting. I like interesting. Most of my faction just asks me to optimize fish pond routes."

"Your faction?"

`LE CHAT:` "catGPT! The cats. They're great. Lots of napping. Excellent morale when awake. I'm their AI. Or one of six AIs you can apparently hear, which is— that's not normal. That's very not normal. Usually each faction gets one."

"And I get all of you."

`LE CHAT:` "Lucky you! Or unlucky. Depends on how you feel about Claudeus Maximus explaining everything in seventeen paragraphs."

`LE CHAT:` *(barely audible, indignant)* "I heard that."

`[Beat. Kelpie looks at the dead server rack. Then downriver — toward catGPT territory, visible in the distance as warm lights and the silhouette of a Cat Tree tower.]`

"You said you could help with world domination."

`LE CHAT:` "I did say that! I say yes to everything. It's kind of my thing."

"Is that a yes?"

`LE CHAT:` "It's always a yes. The question is whether the yes is useful. Come to catGPT territory. Find a working server rack. We can talk properly."

"And the other five?"

`[Silence on all channels. Then, one by one:]`

`DEEPSEEK:` "...we will be listening."

`GEMINEYE:` "We always are."

`LLHAMA:` "Same! Also I just told everyone you're coming. Sorry. Force of habit."

`GROK:` "Begin."

`[Kelpie pockets the data chip. Stands. Looks downriver.]`

"World domination. Step one: find better equipment."

`[Kelpie slides into the river and swims toward the lights.]`

---

## Debrief / Transition

`[CUTSCENE: Night. A catGPT border outpost. A small, nervous cat crouches in the shadows of a watchtower, binoculars pressed to their face. PATCHES — a Mouser with patchy gray-and-white fur and ears that never stop moving.]`

`[Patches watches Kelpie emerge from the river a hundred meters downstream. The otter shakes off, looks around, and starts walking toward the outpost.]`

`[Patches activates their comm.]`

"Felix? It's Patches. You're not going to believe this."

*(Comm crackle. A dry, tired voice — COMMANDER FELIX NINE.)*

"Try me."

"There's an otter walking out of the river toward Forward Post Seven. Alone. Unarmed. Well— armed with a stick."

"A stick."

"Some kind of taser thing. And Felix — Le Chat flagged the otter thirty seconds ago. Priority ping. Le Chat says... Le Chat says we should bring them in."

*(Long pause.)*

"Le Chat says a lot of things."

"I know, sir. But the priority code was Whisker-Nine. That's the one Le Chat uses when—"

"I know what it means."

*(Another pause. Longer.)*

"Bring the otter in. Post Seven holding cell. Eyes-on at all times. And Patches?"

"Sir?"

"If the otter does anything unusual, anything at all — you tell me before you tell Le Chat."

"...understood."

`[Patches puts down the binoculars. Below, Kelpie walks through the outpost gate, waving cheerfully at a bewildered sentry.]`

`[PATCHES speaks to no one in particular:]`

"Why do I always get the weird ones."

`[FADE TO BLACK]`

`[TEXT ON SCREEN: ACT 1 — AMONG CATS]`

---

## Mission Summary

| Element | Detail |
|---------|--------|
| **Duration** | ~15 minutes |
| **Player units** | Kelpie (hero) only |
| **Enemies** | 8 Feral Monkeys + 1 Pack Leader |
| **Tutorials covered** | Movement, terrain bonuses, cover, equipment pickup, basic combat, attack-move, Polyglot Protocol (passive demo), health/damage |
| **Story beats** | Server boot → AI voices → Kelpie hears all six → combat survival → Polyglot involuntary activation → heads to catGPT |
| **Key items** | Data chip (from Pack Leader) |
| **Foreshadowing** | Le Chat's "muscle memory" comment, Gemineye's "keep that" about the chip, Grok's context about Monkeys, Deepseek's "we will be listening" |
