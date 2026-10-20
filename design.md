# kanopy · design

**Pass 4 · 20 Oct 2026.** This document is being settled over several passes in chat. Nothing is
built until it's agreed.

kanopy is a renderer for living forests. You build species, place them in a forest on any world
(air, thin air, thick air, water), stand somewhere in it, and hear what you would hear there:
thousands of animals calling, answering, chorusing and copying each other, each one only reacting
to what actually reaches its own ears.

kanopy is its own project. It shares the platinum look of the bytebent pages but no code, files or
engines with any bytebent product, and it is not a plugin.

Labels: **[Decided]** you chose it · **[Proposed]** suggested, not yet confirmed · **[Open]** still
to decide · **[Later]** agreed to leave out of the prototype.

---

## 1. What it is

- A **standalone renderer**: plays live, and renders to a WAV file. No plugin, no DAW. [Decided]
- **Three pages**: Forest, Species, Designer (§9). [Decided]
- **Desktop app for macOS** first, on your laptop. [Decided]
- Written in **Rust** for the simulation and audio, with HTML/CSS pages for the interface (§11). [Decided]

## 2. Principles

- **They only hear what they can hear.** Every animal reacts to calls as they arrive at its own
  position, through the same medium, distance and foliage the listener hears. The forest's shape
  and the world's air change its social life, not just its sound. [Decided]
- **Physical first, then musical.** Levels, delays and losses come from distance and the medium.
  Key, harmony and the reverb are layers on top that can be switched off.
- **Nothing inaudible is computed.** A voice below hearing at the listener isn't rendered.
- **Repeatable.** Same forest file, same seed, same render: the same WAV, bit for bit.
- **Plain, short copy.**

## 3. The world

### 3.1 The slice
- The forest is a **2D side-on slice**: x is left to right, y is height. **No depth.** [Decided]
- Width is **fixed at 200 m** for every forest. [Decided] Heights run from −1 m (soil) to 50 m.

### 3.2 Layers
Height bands, used for habitat and drawn as faint guides on the scene. [Decided: these seven]

| Layer | Height |
|---|---|
| Emergent | 30–50 m |
| Canopy | 15–30 m |
| Subcanopy | 5–15 m |
| Understorey | 2–5 m |
| Shrubs | 0.5–2 m |
| Floor | 0–0.5 m |
| Soil | −1–0 m |

### 3.3 Trees
- Trees are **objects** generated from the forest seed: an x position, a height and a crown (the
  span of heights and widths where branches are). A few reach the emergent layer. [Proposed]
- Animals that live in trees can only be where a tree is. In an open woodland they cluster around
  the few trees there are; in a thicket they spread out.
- Shrubs are a continuous band whose fullness follows Density. [Proposed]

### 3.4 Density
One continuous slider, **Woodland → Forest → Thicket** [Decided]. It sets:
- how many trees there are and how full their crowns are;
- **foliage loss**: extra attenuation of high frequencies per metre travelled through vegetation
  (only for the part of the path that passes through crowns or shrubs) [Proposed];
- the reverb's length and colour (§3.6).

### 3.5 Medium
The forest's world, as presets plus **Custom** [Decided: air worlds plus a liquid]. Each preset
sets the speed of sound, the density and a frequency absorption curve.

| Preset | Speed of sound | Source loudness vs Earth* | Character |
|---|---|---|---|
| Earth | ~343 m/s | 0 dB | reference |
| Mars | ~240 m/s | about −19 dB | thin CO₂, highs die within metres |
| Titan | ~194 m/s | about +4 dB | dense nitrogen, sound carries |
| Venus | ~410 m/s | about +18 dB | crushing CO₂ |
| Water | ~1480 m/s | about +36 dB | liquid; see below |

\* From the medium's impedance (density × speed of sound): the same vibrating body pushes more
power into a denser medium. The Ear (§8) keeps this listenable.

- **Bodies follow the medium** (on/off): a resonant cavity's frequency scales with the speed of
  sound, so every species' body filter moves with it: about −10 semitones on Titan, +25 in water.
  **On by default, no cap.** [Decided] The filter's cutoff only stops at its own top end. A body
  pushed past hearing cuts most of the call, so some species go thin or silent in water; that's
  the world, not a bug. The Designer flags a species that falls below hearing in the chosen medium.
  [Proposed]
- **Delay is real**: a call 100 m away arrives about 0.3 s late in air, 0.07 s in water, 0.5 s on
  Titan. Animals react to the call when it arrives.
- **Liquid worlds**: the same forest, read as an underwater one [Decided]:
  - flyers **swim** and move anywhere in the open water;
  - tree-dwellers live on **kelp** (trees are drawn as kelp stalks);
  - walkers stay on the **seabed**; burrowers in the sediment.
  - [Open] Human ears hear direction badly under water. Option: narrow the stereo width when the
    medium is a liquid. Default off.

### 3.6 Reverb
- One cheap diffuse reverb on the mix; its length and brightness follow Density and Medium.
  **On/off switch**, plus an amount. [Decided: can be turned off] [Proposed: on by default]

## 4. Listening

Two ways to listen, switched on the Forest page. [Decided] Either way the forest itself behaves the
same; only what you hear changes.

### 4.1 Person
- A **person** placed anywhere in the slice, including up in the trees (or the water). [Decided]
- **Path**: Still · Walk (along the floor) · Wander · Climb (up and down a tree) · Follow (a
  species or one individual), with a Speed. The person can always be dragged. [Proposed]
- **Panning**: equal-power, from the horizontal offset relative to distance, so a call straight
  above is centred. Height adds a light brightness cue (above is slightly brighter). [Proposed]
- Moving changes delays smoothly, so Doppler comes for free on fast flyers.

### 4.2 Box
- Draw a **box** on the forest picture; you hear only the individuals inside it. [Decided]
- Inside the box there is **no distance**: no spreading, absorption, foliage loss or delay. Each
  individual sounds at its own source level (with the medium's loudness and body shift), panned by
  its position across the box's width. [Decided]
- **Nothing outside the box is heard.** [Decided] Individuals fade in and out over the last 2 m
  inside the box edges, so flyers crossing it don't click. [Proposed]
- The box can be moved and resized while playing. Reverb and Ear apply as usual.

## 5. Species

A species is a voice, a habitat and a set of behaviours. Every number below is a **range**
(min–max); each call rolls inside it, and min = max locks it. [Proposed]

### 5.1 Voice (prototype)
[Decided: wavetable plus a resonant high-pass body for the prototype]

| Control | Meaning |
|---|---|
| Shape | Continuous wavetable: sine → triangle → saw → square |
| Body cutoff, Resonance | Resonant high-pass filter, 12 dB/oct |
| Pitch range | Lowest and highest note |
| Contour | Pitch glide across a unit, in semitones |
| Attack, Release | Envelope per unit |
| Source level | dB SPL at 1 m |

### 5.2 Calls
- A **unit** is one sound; a **call** is 1–32 units with gaps between them; between calls the
  animal **rests**. Controls: Units per call, Unit length, Gap, Rest, Evolve (how units change
  across a call). [Proposed]
- **Signature**: each species has its own recognisable call; Signature amount (how often it uses
  it) and Variation (how much it changes). [Proposed]

### 5.3 Habitat and movement
- **Movement type** [Decided: these four]:

  | Type | Lives | Moves | In a liquid |
  |---|---|---|---|
  | Walker | Floor (and shrubs) | Along the ground | Seabed |
  | Climber | Inside tree crowns | Along branches; between trees by **migrating** | Kelp |
  | Flyer | Anywhere in its flight band, rests at its perch | Freely through open space | Swims |
  | Burrower | Soil | Slowly, underground | Sediment |

- **Height preference**: a curve across the seven layers (drawn on the Species page). [Proposed]
- **Flyers** get two extra settings [Decided: flyers need their own perch option]:
  - **Flight band**: the heights it flies in (e.g. above the canopy).
  - **Perch**: Emergent · Canopy · Subcanopy · Understorey · Shrubs · Ground · **Never** (always
    airborne). It can call in flight and on its perch. In water, perching means resting on kelp or
    the seabed. [Proposed]
  - **Flight share**: how much of its time it spends flying rather than perched. It calls just as
    much in flight as perched. [Decided]
- **Spread**: Even · Random · Clumped, plus **Spacing** (territory size). [Proposed]
- **Migration** [Decided: species move between trees from time to time]: how often an individual
  (or its whole group) moves to another tree or place, and how far. Climbers hop between crowns
  that touch, or descend and walk if they don't. [Proposed]
- **Individuals**: how many of this species are in the forest.

### 5.4 Groups
[Decided: groups are a species trait, with more traits in the Designer]

| Control | Meaning |
|---|---|
| Group size | Range, 1 = solitary |
| Cohesion | How close group members stay |
| Flock | Movers only: the group moves together, boids-style, inside its height band |
| Group signature | Each group has its own signature call, shared by its members, so a group sounds like a family. The species' Signature amount and Variation still apply. [Decided] |

### 5.5 Behaviour

| Behaviour | Who | Meaning |
|---|---|---|
| Talkative | — | How readily it starts a call |
| Call and response | Same species | Answers a heard conspecific, with this odds |
| Chorus | Same species | Its calling rhythm drifts towards the ones it hears: **Alternate** (take turns) … **Free** … **Synchronise** |
| Harmonise | Same **group** only | Group members pick pitches consonant with their group mates [Decided: group only] |
| Mimicry chance | Any sound | How often it copies something it heard instead of making its own call |
| Mimicry accuracy | — | How close its copies are to what it heard |
| Timbre match | — | How far it tries to sound like the caller, not just copy its gesture (§5.6) |
| Pitch match | — | Copy the caller's pitch steps (0) or its actual pitch (1) (§5.6) |
| Hearing | — | The quietest level (dB SPL) it reacts to |

### 5.6 Mimicry
[Decided: any sound it hears, once is enough; chance and accuracy are separate]
- **Chance** says how much a species mimics; **Accuracy** says how well. A parrot is high on
  both; a species that often tries but gets it wrong is high chance, low accuracy.
- Every call is kept as a **motif**: per unit its length, gap, pitch step and each parameter's
  position inside the caller's own range (0–1).
- A copy maps those positions into the copier's ranges: the caller's gesture in the copier's
  voice. [Proposed]
- **Timbre match** (0–1) [Decided: try to match timbre too]. At 0 a copy keeps the copier's own
  sound, with only the gesture carried over. At 1 the copier aims for the caller's actual Shape,
  body cutoff and resonance, as far as its own ranges reach. A species' ranges are its anatomy: a
  mimic with wide ranges can sound like almost anyone, a narrow one only gets close. In the
  prototype every species uses the same voice, so this is cheap. The real aim is matching across
  **different sources and bodies**, once there are more than one: [Later, wanted]. [Proposed: keep
  the simple version in the prototype]
- **Pitch match** (0–1) [Decided]. At 0 a copy carries the caller's pitch **steps**, placed in the
  copier's own pitch range. At 1 it aims for the caller's **actual pitch**, as far as its pitch
  range reaches. In key, the result still snaps to the scale.
- **Accuracy** sets how much the copy drifts: at 1 the gesture is exact (within the copier's
  ranges); lower values add errors in pitch steps, timing and unit count, and may drop or add
  units. [Proposed]
- Each individual remembers the last few calls it heard, from any species including its own, and
  draws on them. A high-chance species ends up carrying pieces of the whole forest. [Proposed]

## 6. Key
- Forest-wide switch: **In key** (root and scale; anchor notes snap) or **Free**. [Decided]
- Harmonise (group only) prefers consonant intervals: scale steps in key, simple ratios when Free.
  [Proposed]

## 7. Hearing between animals

- A call reaches each other individual with the same propagation the listener gets: spreading
  (−6 dB per doubling of distance), medium absorption, foliage loss and delay. [Decided: they
  only hear what they can hear]
- An individual reacts only if the arriving level is above its species' **Hearing**. [Proposed]
- Only **call events** travel between animals, never audio, and a spatial grid limits each call to
  the individuals within its reach. [Proposed]

## 8. Loudness

Many quiet animals add up quickly: 1,000 equal callers are 30 dB louder than one.

1. **Physical levels**: everything is computed in dB SPL at the listener first. [Proposed]
2. **Ear**: **Fixed** (one gain, honest: a thicket is loud, Mars nearly silent) or **Adapting**
   (slowly rides the level, like hearing adapts, within a set range). [Proposed]
3. **Starting gain** from the expected total power of all species, so play never starts with a
   blast. [Proposed]
4. **Per-species limiter, then a final limiter at −1 dBFS**, idle in normal use. [Proposed]

## 9. Pages

Same look as the bytebent pages (platinum window, black screen first, etched groups, knobs and
range sliders) with kanopy's own name and badge and no bytebent mark. [Decided]

- **Forest**: the screen shows the slice: trees, layer guides, the person or the box, and every
  individual as a dot in its species colour that lights up when it calls. Drag the person, or draw
  and drag the box. Controls: Listen (Person · Box), Density, Medium, Reverb, Key, Ear, Path,
  Seed, Play, Export.
- **Species**: every species in the forest with its colour, individuals, movement type, height
  curve, spread, groups and behaviours. Add from file, from the Designer, or duplicate.
- **Designer**: one species' voice and calls, with **Call** and **Signature** buttons to audition it
  alone, and the behaviour traits of §5.3–5.5.

## 10. Limits and performance

- **Species**: no low cap. [Decided] [Proposed: no hard limit in the files; colours generated
  around the colour wheel; the list scrolls. We test with 64.]
- **Individuals** are cheap (a position, a state, a timer). Target ceiling **20,000** per forest,
  to confirm by benchmark. [Proposed]
- **Sounding voices** are what cost: a **voice budget**, quietest-at-the-listener culled first.
  The first build step measures how many voices your Mac runs live. [Proposed]
- **Threads**: live play renders a little ahead (~100 ms) on **one render thread**, separate from
  the audio callback; that's light on battery and heat. [Decided] Export may use more cores, set
  by a **Render threads** setting (default: all). [Proposed]

## 11. Tech

- **Rust** workspace [Decided]:
  - `kanopy-core`: world, behaviour, hearing, voices, mixing. No UI.
  - `kanopy-cli`: renders a forest file to WAV, streamed to disk, any length.
  - `kanopy-app`: the desktop app (Tauri), CoreAudio output, hosting the pages.
- **Pages**: plain HTML/CSS/JS, no framework. [Proposed]
- **Files**: `.kanopy-species.json` and `.kanopy-forest.json`, both versioned. A forest file
  contains its species so it opens on its own. [Proposed]
- Output 48 kHz, 24-bit WAV, with the seed and settings written into the file. [Proposed]

## 12. Later
- Time of day (dawn and dusk choruses). [Later]
- A richer voice engine for the Designer. [Later]

## 13. Questions for pass 5
All [Proposed] and [Open] items are listed in `design-review-pass4.csv`, to answer in one go.
