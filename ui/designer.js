// kanopy · the Designer page: one species' voice, calls and speed. Edits go into the same
// draft as the Species page; Apply grows the forest with them, Revert throws them away.
// Call and Signature play the species on its own, dry from 1 m; the screen draws what it said.
(function () {
  const api = window.kanopy;
  const kit = window.kit;
  const $ = (id) => document.getElementById(id);
  const clone = (x) => JSON.parse(JSON.stringify(x));

  let info = null;
  let draft = [];
  let sel = 0;
  let dirty = false;
  let playing = false;
  let last = null; // the last call heard: { units, duration, start (ms), colour, take }
  let statusTimer = 0;

  // ---------- status, dirty

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
  function changed(voice) {
    dirty = true;
    if (info) window.drafts.save(info, draft);
    showDirty();
    drawList();
    if (voice) checkSoon();
    draw();
  }
  const sp = () => draft[sel];
  const v = () => sp().voice;
  const c = () => sp().calls;
  const h = () => sp().habitat;
  const colour = () => window.palette.colours(draft)[sel];

  // ---------- formats

  const NOTES = ['C', 'C♯', 'D', 'D♯', 'E', 'F', 'F♯', 'G', 'G♯', 'A', 'A♯', 'B'];
  const note = (n) => { const r = Math.round(n); return `${NOTES[((r % 12) + 12) % 12]}${Math.floor(r / 12) - 1}`; };
  const pair = (f) => ([a, b]) => (a === b ? f(a) : `${f(a)}–${f(b)}`);
  const hz = (x) => (x >= 1000 ? `${(x / 1000).toFixed(x >= 10000 ? 0 : 1)} kHz` : `${x.toFixed(0)} Hz`);
  const secs = (x) => (x < 1 ? `${(x * 1000).toFixed(0)} ms` : `${x.toFixed(x < 10 ? 2 : 0)} s`);
  const SHAPES = ['sine', 'tri', 'saw', 'square'];
  const shape = (x) => {
    const i = Math.round(x);
    return Math.abs(x - i) < 0.08 ? SHAPES[i] : `${SHAPES[Math.floor(x)]}/${SHAPES[Math.min(3, Math.floor(x) + 1)]}`;
  };
  const pct = (x) => `${(x * 100).toFixed(0)} %`;
  const st = (x) => `${x > 0 ? '+' : x < 0 ? '−' : ''}${Math.abs(x).toFixed(1)}`;

  // ---------- controls

  const fills = [];
  function rng(parent, name, get, set, opts) {
    const r = kit.range($(parent), { name, colour: opts.colour, min: opts.min, max: opts.max, log: opts.log, step: opts.step, def: opts.def, value: opts.def,
      fmt: opts.fmt, onChange: (x) => { set(x); changed(opts.voice); } });
    fills.push(() => r.set(get()));
    return r;
  }
  const G = 'var(--green)', O = 'var(--orange)', P = 'var(--purple)';
  const V = (k) => [() => v()[k], (x) => (v()[k] = x)];
  const C = (k) => [() => c()[k], (x) => (c()[k] = x)];
  rng('r-voice', 'Shape', ...V('shape'), { min: 0, max: 3, def: [0, 1.5], colour: G, voice: true, fmt: pair(shape) });
  rng('r-voice', 'Body', ...V('cutoff'), { min: 10, max: 20000, log: true, def: [100, 800], colour: G, voice: true, fmt: pair(hz) });
  rng('r-voice', 'Resonance', ...V('resonance'), { min: 0, max: 1, def: [0.2, 0.6], colour: G, voice: true, fmt: pair(pct) });
  rng('r-voice', 'Pitch', ...V('pitch'), { min: 24, max: 120, step: 1, def: [72, 84], colour: G, voice: true, fmt: pair(note) });
  rng('r-voice', 'Glide', ...V('contour'), { min: -24, max: 24, step: 0.5, def: [-2, 2], colour: G, voice: true, fmt: (x) => `${pair(st)(x)} st` });
  rng('r-voice', 'Attack', ...V('attack'), { min: 0.001, max: 0.5, log: true, def: [0.005, 0.03], colour: G, voice: true, fmt: pair(secs) });
  rng('r-voice', 'Release', ...V('release'), { min: 0.005, max: 2, log: true, def: [0.02, 0.1], colour: G, voice: true, fmt: pair(secs) });
  rng('r-voice', 'Level at 1 m', ...V('level'), { min: 30, max: 110, step: 1, def: [65, 75], colour: G, voice: true, fmt: (x) => `${pair((y) => y.toFixed(0))(x)} dB` });
  rng('r-calls', 'Units', ...C('units'), { min: 1, max: 32, step: 1, def: [1, 4], colour: O, fmt: pair((y) => y.toFixed(0)) });
  rng('r-calls', 'Unit length', ...C('unit_length'), { min: 0.01, max: 3, log: true, def: [0.08, 0.3], colour: O, fmt: pair(secs) });
  rng('r-calls', 'Gap', ...C('gap'), { min: 0.005, max: 2, log: true, def: [0.03, 0.15], colour: O, fmt: pair(secs) });
  rng('r-calls', 'Rest', ...C('rest'), { min: 0.2, max: 120, log: true, def: [4, 12], colour: O, fmt: pair(secs) });
  const evolveAmount = rng('r-evolve', 'How far', ...C('evolve_amount'), { min: 0, max: 1, def: [0, 0.3], colour: O, fmt: (x) => `${pair(pct)(x)} of pitch` });
  const evolve = kit.seg($('sp-evolve'), (x) => { c().evolve = x; changed(); fillEvolve(); });
  function fillEvolve() { evolveAmount.enable(c().evolve !== 'steady'); }

  function knob(parent, name, get, set, opts) {
    const k = kit.knob($(parent), { name, colour: opts.colour, min: opts.min, max: opts.max, value: opts.min, def: opts.def, step: opts.step, log: opts.log, fmt: opts.fmt || pct,
      onChange: (x) => { set(x); changed(); } });
    fills.push(() => k.set(get()));
    return k;
  }
  knob('k-signature', 'Signature', () => c().signature, (x) => (c().signature = x), { min: 0, max: 1, def: 0.3, colour: O });
  knob('k-signature', 'Variation', () => c().variation, (x) => (c().variation = x), { min: 0, max: 1, def: 0.2, colour: O });
  const ownSpeed = () => info.speeds[h().movement];
  const speedKnob = kit.knob($('k-moving'), { name: 'Speed', colour: P, min: 0.01, max: 30, log: true, value: 1, def: 1,
    fmt: (x) => (x < 1 ? `${(x * 100).toFixed(0)} cm/s` : `${x.toFixed(1)} m/s`),
    onChange: (x) => { h().speed = +x.toPrecision(3); $('sp-own-speed').checked = false; changed(); fillSpeed(); } });
  $('sp-own-speed').addEventListener('change', () => {
    if ($('sp-own-speed').checked) delete h().speed;
    else h().speed = ownSpeed();
    changed();
    fillSpeed();
  });
  function fillSpeed() {
    const own = h().speed == null;
    speedKnob.set(own ? ownSpeed() : h().speed);
    $('sp-own-speed').checked = own;
    $('speed-note').textContent = `Top speed. A ${h().movement}'s own is ${ownSpeed()} m/s; each one goes at 60–100 % of it.`;
  }

  $('sp-name').addEventListener('change', () => {
    const n = $('sp-name').value.trim() || 'Unnamed';
    const taken = new Set(draft.filter((_, i) => i !== sel).map((s) => s.name));
    let name = n;
    for (let k = 2; taken.has(name); k++) name = `${n} ${k}`;
    sp().name = name;
    $('sp-name').value = name;
    changed();
  });

  // ---------- list

  function drawList() {
    const fl = $('forest-list');
    fl.innerHTML = '';
    const cs = window.palette.colours(draft);
    draft.forEach((s, i) => {
      const r = document.createElement('div');
      r.className = 'row';
      r.innerHTML = '<i></i><span></span><small></small>';
      r.querySelector('i').style.background = cs[i];
      r.querySelector('span').textContent = s.name;
      r.querySelector('small').textContent = `${note(s.voice.pitch[0])}–${note(s.voice.pitch[1])} · ${s.habitat.movement}`;
      r.classList.toggle('sel', i === sel);
      r.addEventListener('click', () => { sel = i; last = null; fill(); });
      fl.appendChild(r);
    });
  }

  function fill() {
    const on = !!sp();
    document.querySelectorAll('#editor input, #editor button, #call-btn, #sig-btn').forEach((el) => (el.disabled = !on));
    drawList();
    if (!on) return;
    window.drafts.select(draft, sel);
    $('sp-name').value = sp().name;
    fills.forEach((f) => f());
    evolve.set(c().evolve);
    fillEvolve();
    fillSpeed();
    checkSoon(0);
    draw();
  }

  // ---------- below hearing

  let checkTimer = 0;
  let checkSeq = 0;
  function checkSoon(ms = 250) {
    clearTimeout(checkTimer);
    checkTimer = setTimeout(async () => {
      if (!sp()) return;
      const seq = ++checkSeq;
      try {
        const r = await api.check(sp());
        if (seq === checkSeq) showFlag(r);
      } catch (e) { /* rebuilding */ }
    }, ms);
  }
  function showFlag(r) {
    const el = $('flag');
    const world = r.medium === 'custom' ? 'this world' : r.medium;
    const World = world[0].toUpperCase() + world.slice(1);
    el.className = 'flag';
    if (Math.abs(r.ratio - 1) < 1e-6) {
      el.textContent = `${World}: the body sounds as designed.`;
    } else {
      const moved = `In ${world} its body moves ×${r.ratio.toFixed(2)}, to ${hz(r.cutoff[0])}–${hz(r.cutoff[1])}.`;
      const db = Math.round(r.loss_db);
      if (r.pinned) {
        el.textContent = `${moved} That's past the top of hearing: almost nothing is left.`;
        el.classList.add('bad');
      } else if (r.loss_db >= 12) {
        el.textContent = `${moved} About ${db} dB quieter: most of the call is cut.`;
        el.classList.add('bad');
      } else if (r.loss_db >= 3) {
        el.textContent = `${moved} About ${db} dB quieter, and thinner.`;
        el.classList.add('warn');
      } else {
        el.textContent = `${moved} It sounds about the same.`;
      }
    }
  }

  // ---------- auditions

  async function hear(take) {
    if (!sp()) return;
    try {
      const r = await api.audition(sp(), take);
      last = { ...r, start: performance.now() + r.ahead * 1000, colour: colour() };
      requestAnimationFrame(draw);
    } catch (e) {
      status(String(e.message || e), true);
    }
  }
  $('call-btn').addEventListener('click', () => hear('call'));
  $('sig-btn').addEventListener('click', () => hear('signature'));

  // ---------- the screen: the last call, pitch over time

  const canvas = $('call');
  const ctx = canvas.getContext('2d');
  let W = 0, H = 0;
  let drawing = false;
  function resize() {
    const r = canvas.getBoundingClientRect();
    const dpr = window.devicePixelRatio || 1;
    canvas.width = Math.round(r.width * dpr);
    canvas.height = Math.round(r.height * dpr);
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    W = r.width;
    H = r.height;
    draw();
  }
  function draw() {
    if (!W) return;
    const mono = getComputedStyle(document.body).getPropertyValue('--mono');
    ctx.fillStyle = '#060608';
    ctx.fillRect(0, 0, W, H);
    ctx.lineWidth = 1;
    ctx.globalAlpha = 1;
    if (!sp()) return;
    const [plo, phi] = v().pitch;
    const units = last ? last.units : [];
    let lo = plo, hi = phi;
    for (const u of units) {
      lo = Math.min(lo, u.note, u.note + u.contour);
      hi = Math.max(hi, u.note, u.note + u.contour);
    }
    lo = Math.floor(lo - 3);
    hi = Math.ceil(hi + 3);
    const dur = last ? Math.max(1, last.duration) : 2;
    const L = 12, R = 54, T = 12, B = 22;
    const X = (t) => L + (t / dur) * (W - L - R);
    const Y = (n) => T + ((hi - n) / (hi - lo)) * (H - T - B);
    // the species' pitch range
    ctx.fillStyle = 'rgba(136,255,136,.06)';
    ctx.fillRect(L, Y(phi), W - L - R, Y(plo) - Y(phi));
    ctx.font = `9px ${mono}`;
    ctx.textBaseline = 'middle';
    // C lines with their names; every note faint when the range is narrow
    for (let n = Math.ceil(lo); n <= hi; n++) {
      const isC = ((n % 12) + 12) % 12 === 0;
      if (!isC && hi - lo > 30) continue;
      ctx.strokeStyle = isC ? 'rgba(136,255,136,.22)' : 'rgba(136,255,136,.07)';
      ctx.beginPath();
      ctx.moveTo(L, Math.round(Y(n)) + 0.5);
      ctx.lineTo(W - R, Math.round(Y(n)) + 0.5);
      ctx.stroke();
      if (isC || hi - lo <= 14) {
        ctx.fillStyle = `rgba(136,255,136,${isC ? 0.6 : 0.35})`;
        ctx.fillText(note(n), W - R + 6, Y(n));
      }
    }
    // seconds
    const step = dur > 6 ? 1 : dur > 3 ? 0.5 : dur > 1.5 ? 0.25 : 0.1;
    ctx.textBaseline = 'alphabetic';
    for (let t = 0; t <= dur + 1e-9; t += step) {
      ctx.fillStyle = 'rgba(136,255,136,.45)';
      ctx.fillText(t === 0 ? '0 s' : `${+t.toFixed(2)}`, X(t) - 3, H - 7);
      ctx.fillStyle = 'rgba(136,255,136,.15)';
      ctx.fillRect(X(t), H - B, 1, 4);
    }
    if (!last) {
      ctx.fillStyle = 'rgba(136,255,136,.55)';
      ctx.font = `11px ${mono}`;
      ctx.textAlign = 'center';
      ctx.fillText('PRESS CALL OR SIGNATURE', (L + W - R) / 2, (T + H - B) / 2);
      ctx.textAlign = 'left';
      readouts(null);
      return;
    }
    const now = (performance.now() - last.start) / 1000;
    for (const u of units) {
      const lit = now >= u.at && now <= u.at + u.dur;
      ctx.strokeStyle = last.colour;
      ctx.globalAlpha = lit ? 1 : now > u.at + u.dur ? 0.75 : 0.45;
      ctx.lineWidth = Math.max(1.5, Math.min(9, 1.5 + (u.level - 45) / 5));
      ctx.lineCap = 'round';
      ctx.beginPath();
      ctx.moveTo(X(u.at), Y(u.note));
      ctx.lineTo(X(u.at + u.dur), Y(u.note + u.contour));
      ctx.stroke();
      if (lit) {
        ctx.globalAlpha = 0.25;
        ctx.lineWidth += 8;
        ctx.stroke();
      }
    }
    ctx.globalAlpha = 1;
    if (now >= 0 && now <= last.duration) {
      ctx.fillStyle = 'rgba(136,255,136,.8)';
      ctx.fillRect(X(now), T, 1, H - T - B);
    }
    // keep drawing until the call has played (it starts a little after the press)
    if (now <= last.duration && !drawing) {
      drawing = true;
      requestAnimationFrame(() => { drawing = false; draw(); });
    }
    readouts(now);
  }
  function readouts(now) {
    const s = sp();
    const what = last ? (last.take === 'signature' ? 'SIGNATURE' : 'CALL') : '—';
    const n = last ? last.units.length : 0;
    $('readouts').innerHTML = `<span><b>SPECIES</b>${s.name.toUpperCase()}</span><span><b>LAST</b>${what}${last ? ` · ${n} UNIT${n === 1 ? '' : 'S'} · ${last.duration.toFixed(2)} S` : ''}</span>` +
      `<span><b>FOREST</b>${playing ? (last && now != null && now <= last.duration ? 'DUCKED' : 'PLAYING') : 'STOPPED'}</span>`;
  }

  // ---------- buttons

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
  const togglePlay = () => { playing = !playing; $('play').textContent = playing ? 'Stop' : 'Play'; api.cmd({ type: 'play', on: playing }); draw(); };
  $('play').addEventListener('click', togglePlay);
  window.addEventListener('keydown', (e) => {
    if (e.target.closest('input, select, textarea') || e.metaKey || e.ctrlKey || e.altKey) return;
    if (e.key === 'c' || e.key === 'C') hear('call');
    else if (e.key === 's' || e.key === 'S') hear('signature');
    else if (e.key === ' ') { e.preventDefault(); togglePlay(); }
  });

  // ---------- the loop

  async function loadInfo() {
    info = await api.info();
    $('forestname').textContent = info.forest.name;
    if (!dirty) {
      const d = window.drafts.load(info);
      draft = d.species;
      dirty = d.dirty;
      sel = window.drafts.selected(draft);
      showDirty();
      fill();
    } else checkSoon(0);
  }
  let version = -1;
  let busy = false;
  async function poll() {
    if (!busy) {
      busy = true;
      try {
        const f = window.screen2.decode(await api.frame());
        if (f) {
          if (f.version !== version) {
            version = f.version;
            await loadInfo();
          }
          if (f.playing !== playing) {
            playing = f.playing;
            $('play').textContent = playing ? 'Stop' : 'Play';
            draw();
          }
        }
      } catch (e) { /* rebuilding */ }
      busy = false;
    }
    setTimeout(poll, 150);
  }
  window.addEventListener('resize', resize);
  resize();
  loadInfo().then(poll).catch((e) => status(String(e.message || e), true));
})();
