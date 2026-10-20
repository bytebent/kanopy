// kanopy · the Species page. Edits a copy of the forest's species (the draft, shared with the
// Designer); Apply grows the forest with them and saves them to the library, Revert throws the
// edits away.
(function () {
  const api = window.kanopy;
  const kit = window.kit;
  const $ = (id) => document.getElementById(id);
  const clone = (x) => JSON.parse(JSON.stringify(x));

  let info = null;
  let frame = null;
  let draft = []; // the forest's species, being edited
  let sel = 0;
  let dirty = false;
  let library = [];
  let statusTimer = 0;

  function status(text, err) {
    const el = $('status');
    el.textContent = text || '';
    el.classList.toggle('err', !!err);
    clearTimeout(statusTimer);
    if (text && !err) statusTimer = setTimeout(() => showDirty(), 5000);
  }
  function showDirty() {
    $('apply').disabled = !dirty;
    $('revert').disabled = !dirty;
    if (dirty) {
      $('status').innerHTML = '<span class="dirty">Changes not applied yet</span>';
      $('status').classList.remove('err');
    } else if (!$('status').classList.contains('err')) $('status').textContent = '';
  }
  function changed() {
    dirty = true;
    if (info) window.drafts.save(info, draft);
    showDirty();
    drawLists();
  }
  const sp = () => draft[sel];
  const colours = () => window.palette.colours(draft);
  const unique = (name) => {
    const names = new Set(draft.map((s) => s.name));
    if (!names.has(name)) return name;
    for (let k = 2; ; k++) if (!names.has(`${name} ${k}`)) return `${name} ${k}`;
  };

  // ---------- the screen: everyone faint, the selected species bright, its area editable

  const canvas = $('scene');
  const scr = window.screen2.create(canvas);
  let drag = null;
  const areaRect = (s) => [s.habitat.area.x0, s.habitat.area.x1, s.habitat.area.y0, s.habitat.area.y1];
  function draw() {
    scr.clear();
    if (frame && info && frame.version === info.version) scr.drawAnimals(frame, window.palette.colours(info.forest.species), sel < info.forest.species.length ? sel : -1);
    if (sp()) {
      const c = colours()[sel];
      scr.drawRect(areaRect(sp()), c, 'rgba(255,255,255,.04)');
      if (sp().habitat.movement === 'flyer') {
        // the flight band, inside the area
        const a = sp().habitat.area, [lo, hi] = sp().habitat.flight.band;
        const ctx = scr.ctx;
        ctx.strokeStyle = 'rgba(92,200,245,.7)';
        ctx.setLineDash([2, 4]);
        for (const y of [lo, hi]) {
          ctx.beginPath();
          ctx.moveTo(scr.X(a.x0), scr.Y(y));
          ctx.lineTo(scr.X(a.x1), scr.Y(y));
          ctx.stroke();
        }
        ctx.setLineDash([]);
      }
    }
    if (frame) {
      const f = frame;
      $('readouts').innerHTML = `<span><b>TIME</b>${Math.floor(f.time / 60).toString().padStart(2, '0')}:${Math.floor(f.time % 60).toString().padStart(2, '0')}</span>` +
        `<span><b>CALLING</b>${f.calling}</span><span><b>INDIVIDUALS</b>${draft.reduce((n, s) => n + (s.habitat.individuals || 0), 0)} / ${info ? info.max_individuals : ''}</span>`;
    }
  }
  canvas.addEventListener('pointerdown', (e) => {
    if (!sp()) return;
    canvas.setPointerCapture(e.pointerId);
    drag = scr.rectDrag(scr.pointer(e), areaRect(sp()));
  });
  canvas.addEventListener('pointermove', (e) => {
    if (!drag) return;
    const [x0, x1, y0, y1] = drag.move(scr.pointer(e));
    if (x1 - x0 < 0.5 || y1 - y0 < 0.1) return;
    sp().habitat.area = { x0: +x0.toFixed(1), x1: +x1.toFixed(1), y0: +y0.toFixed(1), y1: +y1.toFixed(1) };
    fillArea();
  });
  const endDrag = () => { if (drag) { drag = null; changed(); } };
  canvas.addEventListener('pointerup', endDrag);
  canvas.addEventListener('pointercancel', endDrag);

  // ---------- lists

  function row(colour, name, small, extra) {
    const r = document.createElement('div');
    r.className = 'row';
    r.innerHTML = `<i style="background:${colour}"></i><span></span><small></small>`;
    r.querySelector('span').textContent = name;
    r.querySelector('small').textContent = small;
    if (extra) r.appendChild(extra);
    return r;
  }
  function drawLists() {
    const fl = $('forest-list');
    fl.innerHTML = '';
    const cs = colours();
    draft.forEach((s, i) => {
      const r = row(cs[i], s.name, `${s.habitat.individuals} · ${s.habitat.movement}`);
      r.classList.toggle('sel', i === sel);
      r.addEventListener('click', () => { sel = i; fill(); });
      fl.appendChild(r);
    });
    const ll = $('library-list');
    ll.innerHTML = '';
    const lc = window.palette.colours(library);
    library.forEach((s, i) => {
      const add = document.createElement('button');
      add.className = 'add';
      add.textContent = 'Add';
      add.title = 'Add to this forest';
      add.addEventListener('click', (e) => {
        e.stopPropagation();
        const c = clone(s);
        c.name = unique(c.name);
        draft.push(c);
        sel = draft.length - 1;
        changed();
        fill();
      });
      const del = document.createElement('button');
      del.className = 'x';
      del.textContent = '×';
      del.title = 'Delete from the library';
      del.addEventListener('click', async (e) => {
        e.stopPropagation();
        if (!confirm(`Delete ${s.name} from the library? Forests that use it keep their copy.`)) return;
        try { await api.libraryDelete(s.name); await loadLibrary(); status(`Deleted ${s.name} from the library`); } catch (err) { status(String(err.message || err), true); }
      });
      const wrap = document.createElement('span');
      wrap.style.display = 'contents';
      wrap.append(add, del);
      ll.appendChild(row(lc[i], s.name, s.habitat.movement, wrap));
    });
  }
  async function loadLibrary() {
    library = await api.library();
    drawLists();
  }

  // ---------- the editor

  const sw = $('swatches');
  const swatchList = [...window.palette.columns.flat(), ...window.palette.neutrals];
  // columns of three (dark, normal, light), then the neutrals as a last column
  for (const c of swatchList) {
    const b = document.createElement('button');
    b.className = 'swatch';
    b.style.background = c;
    b.dataset.c = c;
    b.title = c;
    b.addEventListener('click', () => { sp().colour = c; $('sp-auto').checked = false; changed(); fillSwatches(); });
    sw.appendChild(b);
  }
  function fillSwatches() {
    const c = colours()[sel];
    sw.querySelectorAll('.swatch').forEach((b) => b.classList.toggle('on', b.dataset.c === c));
    $('sp-auto').checked = !sp().colour;
  }
  $('sp-auto').addEventListener('change', () => {
    if ($('sp-auto').checked) delete sp().colour;
    else sp().colour = colours()[sel];
    changed();
    fillSwatches();
  });
  $('sp-name').addEventListener('change', () => {
    const n = $('sp-name').value.trim() || 'Unnamed';
    sp().name = draft.some((s, i) => i !== sel && s.name === n) ? unique(n) : n;
    $('sp-name').value = sp().name;
    changed();
  });

  const h = () => sp().habitat;
  const b = () => sp().behaviour;
  const move = kit.seg($('sp-move'), (v) => { h().movement = v; changed(); fill(); });
  const spread = kit.seg($('sp-spread'), (v) => { h().spread = v; changed(); });
  $('sp-count').addEventListener('change', () => { h().individuals = Math.max(0, Math.min(20000, Math.round(+$('sp-count').value || 0))); $('sp-count').value = h().individuals; changed(); });
  const groupInputs = () => {
    let lo = Math.max(1, Math.round(+$('sp-gmin').value || 1));
    let hi = Math.max(lo, Math.round(+$('sp-gmax').value || lo));
    h().group_size = [lo, hi];
    $('sp-gmin').value = lo;
    $('sp-gmax').value = hi;
    changed();
  };
  $('sp-gmin').addEventListener('change', groupInputs);
  $('sp-gmax').addEventListener('change', groupInputs);
  $('sp-flock').addEventListener('change', () => { h().flock = $('sp-flock').checked; changed(); });
  $('sp-perch').addEventListener('change', () => { h().flight.perch = $('sp-perch').value; changed(); });

  const pct = (v) => `${(v * 100).toFixed(0)} %`;
  const knobs = [];
  function k(parent, name, get, set, opts) {
    const kn = kit.knob($(parent), { name, live: true, colour: opts.colour, min: opts.min, max: opts.max, value: opts.min, def: opts.def, step: opts.step, log: opts.log, fmt: opts.fmt || pct,
      onChange: (v) => { set(v); changed(); draw(); } });
    knobs.push(() => kn.set(get()));
    return kn;
  }
  k('k-habitat', 'Spacing', () => h().spacing, (v) => (h().spacing = v), { min: 0, max: 60, def: 5, colour: 'var(--green)', fmt: (v) => `${v.toFixed(1)} m` });
  k('k-groups', 'Cohesion', () => h().cohesion, (v) => (h().cohesion = v), { min: 0, max: 1, def: 0.5, colour: 'var(--orange)' });
  k('k-groups', 'Moves on', () => h().migration_per_hour, (v) => (h().migration_per_hour = v), { min: 0, max: 60, def: 0, step: 0.1, colour: 'var(--orange)', fmt: (v) => (v === 0 ? 'never' : `${v.toFixed(1)} / h`) });
  k('k-groups', 'How far', () => h().migration_distance, (v) => (h().migration_distance = v), { min: 0, max: 200, def: 30, colour: 'var(--orange)', fmt: (v) => `${v.toFixed(0)} m` });
  const fLo = k('k-flight', 'Flies from', () => h().flight.band[0], (v) => { h().flight.band[0] = Math.min(v, h().flight.band[1]); }, { min: 0, max: 50, def: 5, colour: 'var(--blue)', fmt: (v) => `${v.toFixed(1)} m` });
  const fHi = k('k-flight', 'up to', () => h().flight.band[1], (v) => { h().flight.band[1] = Math.max(v, h().flight.band[0]); }, { min: 0, max: 50, def: 30, colour: 'var(--blue)', fmt: (v) => `${v.toFixed(1)} m` });
  const fShare = k('k-flight', 'In the air', () => h().flight.share, (v) => (h().flight.share = v), { min: 0, max: 1, def: 0.5, colour: 'var(--blue)' });
  const P = 'var(--purple)';
  k('k-behaviour', 'Talkative', () => b().talkative, (v) => (b().talkative = v), { min: 0, max: 1, def: 0.6, colour: P });
  k('k-behaviour', 'Answers', () => b().response, (v) => (b().response = v), { min: 0, max: 1, def: 0.3, colour: P });
  k('k-behaviour', 'Chorus', () => b().chorus, (v) => (b().chorus = v), { min: -1, max: 1, def: 0, colour: P,
    fmt: (v) => (v < -0.05 ? `Alternate ${(-v * 100).toFixed(0)}` : v > 0.05 ? `Sync ${(v * 100).toFixed(0)}` : 'Free') });
  k('k-behaviour', 'Harmonise', () => b().harmonise, (v) => (b().harmonise = v), { min: 0, max: 1, def: 0, colour: P });
  k('k-behaviour', 'Mimicry', () => b().mimicry_chance, (v) => (b().mimicry_chance = v), { min: 0, max: 1, def: 0, colour: P });
  k('k-behaviour', 'Accuracy', () => b().mimicry_accuracy, (v) => (b().mimicry_accuracy = v), { min: 0, max: 1, def: 0.7, colour: P });
  k('k-behaviour', 'Timbre match', () => b().timbre_match, (v) => (b().timbre_match = v), { min: 0, max: 1, def: 0.5, colour: P });
  k('k-behaviour', 'Pitch match', () => b().pitch_match, (v) => (b().pitch_match = v), { min: 0, max: 1, def: 0.5, colour: P });
  k('k-behaviour', 'Hears from', () => b().hearing, (v) => (b().hearing = v), { min: 0, max: 80, def: 30, colour: P, fmt: (v) => `${v.toFixed(0)} dB` });

  function fillArea() {
    const a = h().area;
    $('sp-area').textContent = `${a.x0.toFixed(0)}–${a.x1.toFixed(0)} m across · ${a.y0.toFixed(1)}–${a.y1.toFixed(1)} m high`;
    draw();
  }
  function fill() {
    const on = !!sp();
    window.drafts.select(draft, sel);
    document.querySelectorAll('#editor input, #editor select, #editor button').forEach((el) => (el.disabled = !on));
    drawLists();
    if (!on) return;
    $('sp-name').value = sp().name;
    fillSwatches();
    move.set(h().movement);
    spread.set(h().spread);
    $('sp-count').value = h().individuals;
    $('sp-gmin').value = h().group_size[0];
    $('sp-gmax').value = h().group_size[1];
    $('sp-flock').checked = !!h().flock;
    $('sp-perch').value = h().flight.perch;
    knobs.forEach((f) => f());
    const flyer = h().movement === 'flyer';
    $('flight').disabled = !flyer;
    [fLo, fHi, fShare].forEach((kn) => kn.enable(flyer));
    fillArea();
  }

  // ---------- buttons

  $('new').addEventListener('click', () => {
    const s = clone(info.default_species);
    s.name = unique('New species');
    draft.push(s);
    sel = draft.length - 1;
    changed();
    fill();
  });
  $('dup').addEventListener('click', () => {
    if (!sp()) return;
    const s = clone(sp());
    s.name = unique(s.name);
    draft.splice(sel + 1, 0, s);
    sel += 1;
    changed();
    fill();
  });
  $('remove').addEventListener('click', () => {
    if (!sp()) return;
    if (draft.length === 1) return status('A forest needs at least one species', true);
    draft.splice(sel, 1);
    sel = Math.min(sel, draft.length - 1);
    changed();
    fill();
  });
  $('import').addEventListener('click', async () => {
    try { const s = await api.libraryImport(); if (s) { await loadLibrary(); status(`Added ${s.name} to the library`); } } catch (e) { status(String(e.message || e), true); }
  });
  $('export-sp').addEventListener('click', async () => {
    try { const p = await api.speciesExport(sp()); if (p) status(`Saved ${p}`); } catch (e) { status(String(e.message || e), true); }
  });
  $('apply').addEventListener('click', async () => {
    const total = draft.reduce((n, s) => n + s.habitat.individuals, 0);
    if (total > info.max_individuals) return status(`${total} individuals: this build plays up to ${info.max_individuals}`, true);
    try {
      const e = await api.cmd({ type: 'species', species: draft });
      if (e) return status(e, true);
      await api.librarySave(draft);
      window.drafts.clear();
      dirty = false;
      showDirty();
      await loadLibrary();
      status('Applied: a new forest is growing');
    } catch (e) {
      status(String(e.message || e), true);
    }
  });
  $('revert').addEventListener('click', () => {
    draft = clone(info.forest.species);
    sel = Math.min(sel, draft.length - 1);
    window.drafts.clear();
    dirty = false;
    showDirty();
    fill();
  });
  let playing = false;
  $('play').addEventListener('click', () => { playing = !playing; api.cmd({ type: 'play', on: playing }); });

  // ---------- the loop

  async function loadInfo() {
    info = await api.info();
    scr.info = info;
    scr.drawBackground();
    $('forestname').textContent = info.forest.name;
    if (!dirty) {
      const d = window.drafts.load(info);
      draft = d.species;
      dirty = d.dirty;
      sel = window.drafts.selected(draft);
      showDirty();
      fill();
    }
  }
  let busy = false;
  async function tick() {
    if (!busy) {
      busy = true;
      try {
        const f = window.screen2.decode(await api.frame());
        if (f) {
          frame = f;
          if (!info || f.version !== info.version) await loadInfo();
          if (f.playing !== playing) {
            playing = f.playing;
            $('play').textContent = playing ? 'Stop' : 'Play';
          }
        }
      } catch (e) { /* rebuilding */ }
      busy = false;
    }
    draw();
    requestAnimationFrame(tick);
  }
  window.addEventListener('resize', () => scr.resize());
  scr.resize();
  Promise.all([loadInfo(), loadLibrary()]).then(() => requestAnimationFrame(tick)).catch((e) => status(String(e.message || e), true));
})();
