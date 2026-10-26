# kanopy · design

**Pass 6 · 20 Oct 2026.** Agreed; changes from here on are noted in the commit log. Settled over six passes in chat.

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
- **Repeatable.** Same forest file, same seed, same render: the same WAV, bit for bit. On any
  machine: kanopy uses its own portable maths, not the system's. [Decided as built]
- **Plain, short copy.**

## 3. The world

### 3.1 The slice
- The forest is a **2D side-on slice**: x is left to right, y is height. **No depth.** [Decided]
- Width is **fixed at 200 m** for every forest. [Decided] Heights run from −3 m (soil) to 50 m.

### 3.2 Layers
Height bands, used for habitat and drawn as faint guides on the scene. [Decided: these seven]

| Layer | Height |
|---|---|
| Emergent | 30–50 m |
| Canopy | 15–30 m |
| Subcanopy | 5–15 m |
| Understorey | 3–5 m |
| Shrubs | 0.5–3 m |
| Floor | 0–0.5 m |
| Soil | −3–0 m |

### 3.3 Trees
- Trees are **objects** generated from the forest seed: an x position, a height and a crown (the
  span of heights and widths where branches are). [Decided]
- **Tree variation** (slider): how much tree heights and crowns differ, from an even stand to a
  ragged mix. The tallest tree always reaches the top of the emergent layer. [Decided]
- Animals that live in trees can only be where a tree is. In an open woodland they cluster around
  the few trees there are; in a thicket they spread out.
- Shrubs are a continuous band whose fullness follows Density. [Decided]

### 3.4 Density
One continuous slider, **Woodland → Forest → Thicket** [Decided]. It sets:
- how many trees there are and how full their crowns are;
- **foliage loss**: extra attenuation of high frequencies per metre travelled through vegetation
  (only for the part of the path that passes through crowns or shrubs). **Kept subtle.** [Decided];
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
  [Decided]
- **Delay is real**: a call 100 m away arrives about 0.3 s late in air, 0.07 s in water, 0.5 s on
  Titan. Animals react to the call when it arrives.
- **Liquid worlds**: the same forest, read as an underwater one [Decided]:
  - flyers **swim** and move anywhere in the open water;
  - tree-dwellers live on **kelp** (trees are drawn as kelp stalks);
  - walkers stay on the **seabed**; burrowers in the sediment.
  - **Underwater stereo** (on/off): human ears hear direction badly under water, so this narrows
    the stereo width in a liquid. Default off. [Decided]

- **As built in batch 04** [Proposed: check these]:
  - Absorption, dB per metre at 1 kHz / 8 kHz: Earth 0.0015 / 0.1; Mars 0.2 / 13; Titan 0.0005 /
    0.03; Venus 0.001 / 0.06; Water almost none. Custom takes speed, density, absorption and
    liquid.
  - **Medium compensation** (0…1, default 1): the reference level moves by the medium's loudness
    offset, like a microphone set for each world, so every world starts at a comparable level and
    the Ear's ±5 dB works on top. At 0 it's fully physical: water is 36 dB louder than Earth and
    the limiters do the work. The ±5 dB Ear can't bridge 55 dB between Mars and water on its own.
  - Foliage: 20 m of crown takes about 3 dB off 4 kHz. Soil: crossing the surface −20 dB, then per
    metre −1.5 dB and a strong high loss (2 m of soil: about −11 dB at 4 kHz). Height: straight
    below is darker by about 1.5 dB at 4 kHz than straight above.
  - Animals hear through the medium too (louder worlds carry further), with foliage as the
    forest's average and the surface loss when one is underground.
  - Underwater stereo narrows the field to 35 %.

### 3.6 Reverb
- One cheap diffuse reverb on the mix; its length and brightness follow Density and Medium.
  **On/off switch**, plus an amount. **On by default.** [Decided]
- As built: an 8-line feedback delay network. Length 0.6 s (woodland) to 2.2 s (thicket); Mars
  ×0.5 and dark (900 Hz), Titan ×1.3, Venus ×1.4, water ×1.6 and darker (2.5 kHz). Amount 0.3.
  [Proposed]

## 4. Listening

Two ways to listen, switched on the Forest page. [Decided] Either way the forest itself behaves the
same; only what you hear changes.

### 4.1 Listener
- The **listener** is drawn as a person but works like a microphone: it can be anywhere in the
  slice, in the open air, inside a crown, in the water or underground. [Decided]
- Underground, sound from above passes through the soil and comes through heavily muffled;
  burrowers nearby are clear. [Decided]
- **Path**: Still · Wander (drifts anywhere) · Follow (a species), with a Speed. The listener can
  always be dragged. No following single individuals. [Decided]
- **Panning**: equal-power, from the horizontal offset relative to distance, so a call straight
  above is centred. Height adds a light brightness cue (above is slightly brighter). [Decided]
- Moving changes delays smoothly, so Doppler comes for free on fast flyers.
- **Paths in renders too** [Decided]: an export follows the listener's path, Still, Wander or
  Follow, at its Speed, not only a still listener. The route comes from the forest's seed, so
  the same file renders the same walk. The path is saved in the forest file (and the WAV).
  - **Underground** (on/off, default off): whether Wander may go below the surface. [Decided]
  - Wander turns smoothly, not at sharp corners, so its Doppler doesn't jump. [Decided]
  - The box doesn't wander. [Decided]
  - As built [Proposed: check by ear]: turns are arcs; the heading turns at most speed / 3 m
    per second (never slower than 1 rad/s), so turns are about 3 m across at any speed; speed
    changes take about 1.5 s. Near the edges of the slice, and the ground with Underground off,
    it slides along rather than stopping. Wander picks a new spot within 60 m left or right,
    6 m before reaching the last one; with Underground on, a tenth of the spots are below. In
    files: `"path": {"kind": "wander", "speed": 2, "underground": false}`; Follow names its
    species (`"follow": "Swifts"`).

### 4.2 Box
- Draw a **box** on the forest picture; you hear only the individuals inside it. [Decided]
- Inside the box there is **no distance**: no spreading, absorption, foliage loss or delay. Each
  individual sounds at its own source level (with the medium's loudness and body shift), panned by
  its position across the box's width. [Decided]
- **Nothing outside the box is heard.** [Decided] Individuals fade in and out over the last 2 m
  inside the box edges, so flyers crossing it don't click. [Decided]
- The box can be moved and resized while playing. Reverb and Ear apply as usual.
- As built: with no distance, a box is far louder than the forest heard from a listener (every
  animal as if 1 m away), so the box is brought to the Ear's target level first; the Ear's ±5 dB
  works on top. Animals within 20 m outside the box still call (and can be answered inside it)
  but aren't heard. [Proposed]

## 5. Species

A species is a voice, a habitat and a set of behaviours. Every number below is a **range**
(min–max); each call rolls inside it, and min = max locks it. [Decided]

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
  across a call). [Decided]
- **Signature**: each group has one recognisable call of its own, made once from the seed (§5.4).
  Two controls [Decided]:
  - **Signature amount** (0–1): how often a call is the signature rather than a freshly rolled
    call. 0 = every call is new; 1 = it only ever says its signature.
  - **Variation** (0–1): how much each repeat of the signature is changed. 0 = identical every
    time; 1 = barely recognisable.

### 5.3 Habitat and movement
- **Movement type** [Decided: these four]:

  | Type | Lives | Moves | In a liquid |
  |---|---|---|---|
  | Walker | Floor (and shrubs) | Along the ground | Seabed |
  | Climber | Inside tree crowns | Along branches; between trees by **migrating** | Kelp |
  | Flyer | Anywhere in its flight band, rests at its perch | Freely through open space | Swims |
  | Burrower | Soil | Slowly, underground | Sediment |

- **Area**: where the species lives, set on the Species page as a box drawn on the forest picture:
  a stretch of the width (left–right) and a height range (bottom–top, snapping to the seven
  layers if you like). Every individual stays inside its area; movers stay inside it too. [Decided]
- **Flyers** get two extra settings [Decided: flyers need their own perch option]:
  - **Flight band**: the heights it flies in (e.g. above the canopy).
  - **Perch**: Emergent · Canopy · Subcanopy · Understorey · Shrubs · Ground · **Never** (always
    airborne). It can call in flight and on its perch. In water, perching means resting on kelp or
    the seabed. [Decided]
  - **Flight share**: how much of its time it spends flying rather than perched. It calls just as
    much in flight as perched. [Decided]
- **Spread**: how the species' groups are laid out **inside its area** [Decided]:
  - **Even**: evenly spaced, like territorial birds each holding a patch;
  - **Random**: anywhere in the area;
  - **Clumped**: bunched into a few patches with empty space between.
  - **Spacing**: the smallest distance kept between neighbouring groups (territory size).
- **Migration** [Decided: species move between trees from time to time]: how often an individual
  (or its whole group) moves to another tree or place, and how far. Climbers hop between crowns
  that touch, or descend and walk if they don't. [Decided]
- **Individuals**: how many of this species are in the forest.
- **Speed**: top speed in m/s, a Designer control. Left unset it's the movement type's own
  (walker 1.2, climber 0.8, flyer 9, burrower 0.05); each individual moves at 60–100 % of it.
  [Decided] Pause and perch times stay internal constants (listed in `experiments.md`, batch 02).

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

**As built in batch 03** [Proposed: check these]:
- **Talkative**: when its rest is over it calls with these odds, otherwise rests again.
- **Call and response**: a call gets **at most one answer**. With the species' Response odds,
  one conspecific that heard it answers (louder hearers likelier), 0.2–1 s after the call has
  reached it in full, and only if it has rested at least its shortest Rest. So exchanges go back
  and forth and die out by themselves (Response 0.6: about 2.5 calls long). Answering every call
  with every hearer flooded the forest.
- **Chorus, synchronise**: hearing a conspecific pulls its next call forward to join, if that
  call was coming soon anyway (within Chorus × 40 % of its mean rest), once per rest. It shifts
  *when* it calls, not how often. **Alternate**: if its next call would land on the one it
  hears, it waits until that one has finished (odds: |Chorus|).
- **Harmonise** moves fresh and signature calls, not copies.
- **Evolve** moves pitch only; its amount is a share of the pitch range. Each call has one sound
  (Shape, body, level), varied a little per unit.
- **Copies** keep the caller's number of units (the gesture), 1–32.

### 5.6 Mimicry
[Decided: any sound it hears, once is enough; chance and accuracy are separate]
- **Chance** says how much a species mimics; **Accuracy** says how well. A parrot is high on
  both; a species that often tries but gets it wrong is high chance, low accuracy.
- Every call is kept as a **motif**: per unit its length, gap, pitch step and each parameter's
  position inside the caller's own range (0–1).
- A copy maps those positions into the copier's ranges: the caller's gesture in the copier's
  voice. [Decided]
- **Timbre match** (0–1) [Decided: try to match timbre too]. At 0 a copy keeps the copier's own
  sound, with only the gesture carried over. At 1 the copier aims for the caller's actual Shape,
  body cutoff and resonance, as far as its own ranges reach. A species' ranges are its anatomy: a
  mimic with wide ranges can sound like almost anyone, a narrow one only gets close. In the
  prototype every species uses the same voice, so this is cheap. The real aim is matching across
  **different sources and bodies**, once there are more than one: [Later, wanted]. The simple
  version is in the prototype. [Decided]
- **Pitch match** (0–1) [Decided]. At 0 a copy carries the caller's pitch **steps**, placed in the
  copier's own pitch range. At 1 it aims for the caller's **actual pitch**, as far as its pitch
  range reaches. In key, the result still snaps to the scale.
- **Accuracy** sets how much the copy drifts: at 1 the gesture is exact (within the copier's
  ranges); lower values add errors in pitch steps, timing and unit count, and may drop or add
  units. [Decided]
- Each individual remembers the last few calls it heard, from any species including its own, and
  draws on them. A high-chance species ends up carrying pieces of the whole forest. [Decided]

## 6. Key
- Forest-wide switch: **In key** (root and scale; anchor notes snap) or **Free**. [Decided]
- Harmonise (group only) prefers consonant intervals: scale steps in key, simple ratios when Free.
  [Decided]

## 7. Hearing between animals

- A call reaches each other individual with the same propagation the listener gets: spreading
  (−6 dB per doubling of distance), medium absorption, foliage loss and delay. [Decided: they
  only hear what they can hear]
- An individual reacts only if the arriving level is above its species' **Hearing**. [Decided]
- Only **call events** travel between animals, never audio, and a spatial grid limits each call to
  the individuals within its reach. [Decided]

- As built: reach is worked out once per call (spreading and air absorption at the call's first
  note); a mimic's memory is its last 4 calls heard within 60 s, worked out when it decides to
  copy. [Decided]

## 8. Loudness

Many quiet animals add up quickly: 1,000 equal callers are 30 dB louder than one.

1. **Physical levels**: everything is computed in dB SPL at the listener first. [Decided]
2. **Ear**: how the overall volume follows the forest, two modes [Decided]:
   - **Fixed**: one master gain that never moves. A loud thicket stays loud and a sparse Martian
     woodland stays very quiet, so you hear the real differences between forests and places.
   - **Adapting**: the gain slowly turns up when it's quiet and down when it's loud, over tens of
     seconds, like your ears getting used to a place. It moves at most **±5 dB**, so a quiet
     forest still sounds clearly quieter than a loud one. Default.
3. **Starting gain**: before playing, kanopy estimates how loud the forest will be on average,
   from how many individuals there are, how loud and how often they call and how far they are from
   the listener, and sets the master gain from that. Play starts at a sensible level, not at a
   blast or near-silence. It's also Adapting's starting point. It too is capped small: at most
   **±5 dB** from the physical level. [Decided]
4. **Per-species limiter, then a final limiter at −1 dBFS**, idle in normal use. [Decided]

Fixed in batch 09: the limiters look exactly 64 samples (1.3 ms) ahead whatever the block
size; before, they dropped sound at every 50 ms step (the clicks). [Decided as built]

As built in batch 04 [Proposed]: the reference maps 80 dB SPL to 0 dBFS, so an ordinary forest
around 60 dB SPL sits near −20 dBFS RMS, which is where the starting gain aims. Both the starting
gain and Adapting stay within ±5 dB of the reference (after medium compensation). Adapting's time
constant is 20 s. Species limiters sit at −3 dBFS; limiters look 1.3 ms ahead. They act when an
animal calls right next to the listener, which is what they're for.

## 9. Pages

Same look as the bytebent pages (platinum window, black screen first, etched groups, knobs and
range sliders) with kanopy's own name and badge and no bytebent mark. [Decided]

- **Forest**: the screen shows the slice: trees, layer guides, the listener or the box, and every
  individual as a dot in its species colour that lights up when it calls. Drag the listener, or draw
  and drag the box. Controls: Listen (Listener · Box), Density, Medium, Reverb, Key, Ear, Path,
  Seed, Play, Export.
- **Species**: every species in the forest with its colour, individuals, movement type, area,
  spread, groups and behaviours. Add from file, from the Designer, or duplicate.
- As built in batch 06 (Forest page) [Proposed: check in use]: the listener glides to where it's
  dragged at up to 40 m/s, so a drag makes real Doppler; Wander drifts to random spots (a tenth of
  them underground), Follow keeps 2 m off the nearest group of the chosen species; clicking a
  species in the legend follows it. "Calls within" sets the active zone radius. Density,
  variation, seed and world grow a new forest (the old one fades out over 50 ms); listener, box,
  key, reverb, Ear and volume change while playing. Space plays and stops. Volume is playback
  only, not part of renders.
- As built in batch 07 (Species page) [Decided]:
  - **Colours** come from one palette: the six kanopy colours and the six in between, each dark,
    normal and light, plus three greys (39 in all). Colours are picked by hand on the Species
    page; two species may share one. A species with no colour of its own gets the next free one.
  - **Not live.** Edits are a draft: **Apply** grows a new forest with them, **Revert** throws
    them away. The screen shows the selected species bright and the rest faint; its area is
    dragged or redrawn there.
  - **Library.** Every species made so far is kept in one folder (Application Support on the
    Mac) and listed on the page; **Add** puts a copy in the forest. It starts with the demo
    species. Apply saves the forest's species to it, by name. Import… and Export… are for single
    species files. Forest files still carry full copies of their species.
- **Designer**: one species' voice and calls, with **Call** and **Signature** buttons to audition it
  alone, and the behaviour traits of §5.3–5.5. Decided for batch 08:
  - **Same edits as the Species page**: one draft shared by both pages, **Apply** and **Revert**.
    Moving between the pages keeps the draft. [Decided]
  - The Species page keeps habitat, groups and behaviour; the Designer has the voice (§5.1),
    the calls (§5.2) and Speed (§5.3). [Decided]
  - **Auditioning is dry and close**: the creature as heard from 1 m, no distance, no reverb, no
    Ear. **Call** plays a fresh call, **Signature** its signature as a group repeats it (with
    Variation). Both are in the forest's key. [Decided]
  - **If the forest is playing**, it ducks while the creature plays on top, then comes back.
    If it's stopped, only the creature plays. [Decided]
  - The screen shows the last call heard: each unit as a stroke of pitch over time.
  - **Below hearing**: the page says when the forest's world moves the body (Bodies follow) so
    far that most of the call is cut, with how much quieter it gets. [Decided]
  - As built [Proposed: check by ear]: the forest ducks by **15 dB**, going down in 40 ms and
    back up over 0.5 s. Live output has a safety limit at −1 dBFS, since a creature at 1 m can
    peak above full scale. 80 dB SPL at 1 m is full scale, as in the forest. The audition's body
    moves with the world (as it sounds there); its loudness doesn't (medium compensation). The
    flag: 3–12 dB lost "thinner", 12 dB or more "most of the call is cut", body past 20 kHz
    "almost nothing is left". A signature here is the species' own (one per species and seed),
    not any one group's.

## 10. Limits and performance

- **Species**: no low cap, no hard limit in the files; colours from the palette (§9);
  the list scrolls. We test with 64. [Decided, for now]
- **Individuals** are cheap (a position, a state, a timer). Target ceiling **20,000** per forest,
  for testing. The app's real limit will likely be lower, set from the benchmark. [Decided]
- **Sounding voices** are what cost: each animal calling right now needs its sound computed.
  Two limits keep that in hand [Decided]:
  - **Active zone** (default radius 150 m: the whole slice from its middle) [Proposed]: only
    individuals within an activation range of the listener (a radius around
    it), or of the box (the box plus a margin), call at all. Outside it they keep living, moving
    and migrating, but stay silent, so they cost almost nothing. The margin lets animals just
    outside a box call and set off answers inside it without being heard.
  - **Calling cap**: the most individuals that may be calling at once, even inside the active zone,
    so a box over the whole forest doesn't make everything sound. An individual that wants to call
    while the cap is full skips that call and rests again; the ones closest to the listener (or
    anywhere, in a box) get priority. [Decided]
  - Calling cap defaults: **600 live, 2,000 for export**, from the batch 01 bench on an Intel
    Mac (i7-8700B: ~940 units at half a core). Active zone radius and margin are set in batch 03,
    once calls have real lengths. [Decided, for now]
- As built: the render thread stays 125 ms ahead of the sound card; the screen shows each frame
  when its sound is actually playing, so the dots light up with what you hear.
- **Threads**: live play renders a little ahead (~100 ms) on **one render thread**, separate from
  the audio callback; that's light on battery and heat. [Decided] Export may use more cores, set
  by a **Render threads** setting (default: all). [Decided]

## 11. Tech

- **Rust** workspace [Decided]:
  - `kanopy-core`: world, behaviour, hearing, voices, mixing. No UI.
  - `kanopy-cli`: renders a forest file to WAV, streamed to disk, any length.
  - `kanopy-app`: the desktop app (Tauri), CoreAudio output, hosting the pages.
- **Pages**: written in plain HTML, CSS and JavaScript, the way the bytebent pages are, with no
  framework such as React or Vue on top. Simpler to read and change, nothing to install. [Decided]
- **Files**: `.kanopy-species.json` and `.kanopy-forest.json`, both versioned. A forest file
  contains its species so it opens on its own. [Decided]
- As built in batch 05 [Proposed]: JSON, ranges as `[lo, hi]`, names lowercase. A forest file
  holds the seed, density, tree variation, medium (a preset name or custom speed, density and
  absorption), key (or null for Free), listening (listener x, y and radius, or a box), cap, Ear,
  medium compensation, reverb amount (or null), bodies follow, underwater stereo, render settings
  (minutes, fades, bits) and its species. Missing fields take defaults, so a file only needs what
  differs; a file from a newer kanopy is refused with a clear message.
- As built: `kanopy render` streams to disk (memory stays at about 16 MB however long), fades in
  and out (default 5 s / 10 s, raised cosine), and stores the forest file with its render
  settings inside the WAV (`knpy` chunk), so a WAV can be opened and rendered again exactly.
- Output 48 kHz, 24-bit WAV, with the seed and settings written into the file. [Decided, for now]

## 12. Later
- Time of day (dawn and dusk choruses). [Later]
- A richer voice engine for the Designer. [Later]

## 13. Still open
Nothing.
