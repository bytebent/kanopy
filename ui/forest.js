// kanopy · the Forest page.
(function () {
  const api = window.kanopy;
  const kit = window.kit;
  const W = 200, TOP = 50, BOTTOM = -3, SHRUB_TOP = 3;
  const LAYERS = [['Soil', -3, 0], ['Floor', 0, 0.5], ['Shrubs', 0.5, 3], ['Understorey', 3, 5], ['Subcanopy', 5, 15], ['Canopy', 15, 30], ['Emergent', 30, 50]];
  const colour = (i) => speciesColours[i] || '#888';
  const NOTES = ['C', 'C#', 'D', 'D#', 'E', 'F', 'F#', 'G', 'G#', 'A', 'A#', 'B'];
  const SCALES = [['major', 'Major'], ['minor', 'Minor'], ['dorian', 'Dorian'], ['phrygian', 'Phrygian'], ['lydian', 'Lydian'], ['mixolydian', 'Mixolydian'],
    ['pentatonic-major', 'Pentatonic major'], ['pentatonic-minor', 'Pentatonic minor'], ['whole-tone', 'Whole tone'], ['chromatic', 'Chromatic']];
  const $ = (id) => document.getElementById(id);

  let info = null;
  let frame = null;
  let boxRect = null; // [x0, x1, y0, y1] in box mode
  let following = null;
  let statusTimer = 0;

  function status(text, err) {
    const el = $('status');
    el.textContent = text || '';
    el.classList.toggle('err', !!err);
    clearTimeout(statusTimer);
    if (text && !err) statusTimer = setTimeout(() => (el.textContent = ''), 6000);
  }
  async function send(cmd) {
    try {
      const e = await api.cmd(cmd);
      if (e) status(e, true);
    } catch (e) {
      status(String(e.message || e), true);
    }
  }

  // ---------- the screen (screen.js)

  const canvas = $('scene');
  const scr = window.screen2.create(canvas);
  const ctx = scr.ctx;
  const { X, Y } = scr;
  const decode = window.screen2.decode;
  const layerAt = scr.layerAt;
  const resize = () => scr.resize();
  const drawBackground = () => { scr.info = info; scr.drawBackground(); };
  const drawListener = scr.drawListener;
  let speciesColours = [];

  function drawFrame() {
    scr.clear();
    if (!frame) return;
    scr.drawAnimals(frame, speciesColours);
    // listener, or the box
    if (frame.boxed || mode.value === 'box') {
      const b = drag && drag.kind !== 'listener' && boxRect ? boxRect : frame.boxed ? frame.box : boxRect;
      if (b) scr.drawRect(b, '#fdb827', 'rgba(253,184,39,.06)');
    }
else {
      const radius = info && info.forest.listen.mode === 'listener' ? info.forest.listen.radius : 150;
      ctx.strokeStyle = 'rgba(92,200,245,.18)';
      ctx.setLineDash([2, 5]);
      ctx.beginPath();
      ctx.arc(X(frame.lx), Y(frame.ly), radius * scr.G.s, 0, Math.PI * 2);
      ctx.stroke();
      ctx.setLineDash([]);
      if (drag && drag.kind === 'listener') drawListener(drag.x, drag.y, true);
      drawListener(frame.lx, frame.ly, false);
    }
    readouts();
  }

  function mmss(t) {
    const m = Math.floor(t / 60), s = Math.floor(t % 60);
    return `${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
  }
  function meter(db) {
    const w = Math.max(0, Math.min(1, (db + 48) / 48));
    return `<span class="meter"><i class="${db > -3 ? 'hot' : ''}" style="width:${(w * 100).toFixed(0)}%"></i></span>`;
  }
  let cursor = null;
  function readouts() {
    const f = frame;
    const cap = info ? info.forest.cap : 0;
    const parts = [
      `<span><b>TIME</b>${mmss(f.time)}</span>`,
      `<span><b>CALLING</b>${f.calling} / ${cap}</span>`,
      `<span><b>ACTIVE</b>${f.active}</span>`,
      `<span><b>CPU</b>${(f.cpu * 100).toFixed(0)} %</span>`,
      `<span><b>EAR</b>${f.ear >= 0 ? '+' : ''}${f.ear.toFixed(1)} dB</span>`,
      `<span><b>L</b>${meter(f.peak[0])} <b>R</b>${meter(f.peak[1])}</span>`,
    ];
    if (cursor) parts.push(`<span><b>AT</b>${cursor.x.toFixed(0)} m · ${cursor.y.toFixed(1)} m · ${layerAt(cursor.y).toUpperCase()}</span>`);
    $('readouts').innerHTML = parts.join('');
  }

  // ---------- dragging the listener or the box

  let drag = null;
  let lastSend = 0;
  function sendBox(force) {
    const now = performance.now();
    if (!force && now - lastSend < 150) return;
    lastSend = now;
    const [x0, x1, y0, y1] = boxRect;
    if (x1 - x0 > 1 && y1 - y0 > 1) send({ type: 'hear', rect: [x0, x1, y0, y1] });
  }
  canvas.addEventListener('pointerdown', (e) => {
    const p = scr.pointer(e);
    canvas.setPointerCapture(e.pointerId);
    if (mode.value === 'listener') {
      drag = { kind: 'listener', ...p };
      send({ type: 'listener', x: p.x, y: p.y });
      return;
    }
    drag = { kind: 'box', rd: scr.rectDrag(p, boxRect) };
    if (!boxRect) boxRect = [p.x, p.x, p.y, p.y];
  });
  canvas.addEventListener('pointermove', (e) => {
    const p = scr.pointer(e);
    cursor = p;
    if (!drag) return;
    if (drag.kind === 'listener') {
      drag.x = p.x;
      drag.y = p.y;
      const now = performance.now();
      if (now - lastSend > 40) {
        lastSend = now;
        send({ type: 'listener', x: p.x, y: p.y });
      }
    } else {
      boxRect = drag.rd.move(p);
      sendBox(false);
    }
  });
  const endDrag = () => {
    if (!drag) return;
    if (drag.kind === 'listener') send({ type: 'listener', x: drag.x, y: drag.y });
    else if (boxRect) sendBox(true);
    drag = null;
  };
  canvas.addEventListener('pointerup', endDrag);
  canvas.addEventListener('pointercancel', endDrag);
  canvas.addEventListener('pointerleave', () => (cursor = null));

  // ---------- controls

  const mode = kit.seg($('mode'), (v) => {
    if (v === 'box') {
      if (!boxRect && frame) {
        const x = frame.lx, y = Math.max(frame.ly, 1);
        boxRect = [Math.max(0, x - 20), Math.min(W, x + 20), Math.max(BOTTOM, y - 6), Math.min(TOP, y + 14)];
      }
      sendBox(true);
    } else {
      send({ type: 'hear', rect: null });
    }
    hint();
  });
  function hint() {
    $('hint').innerHTML = mode.value === 'box'
      ? 'DRAG TO DRAW A BOX<br>DRAG INSIDE IT TO MOVE · CORNERS RESIZE'
      : 'DRAG TO MOVE THE LISTENER';
  }
  const sendPath = () => {
    const kind = path.value;
    const sp = $('follow').value === '' ? null : Number($('follow').value);
    following = kind === 'follow' ? sp : null;
    send({ type: 'path', kind, speed: speed.value, species: sp });
    legendMarks();
  };
  const path = kit.seg($('path'), sendPath);
  $('follow').addEventListener('change', () => {
    path.set('follow');
    sendPath();
  });
  const kl = $('k-listen'), kf = $('k-forest'), kw = $('k-world'), ks = $('k-sound');
  const speed = kit.knob(kl, { name: 'Speed', min: 0.1, max: 20, value: 1.5, def: 1.5, log: true, colour: 'var(--blue)', fmt: (v) => `${v.toFixed(1)} m/s`, onChange: sendPath });
  const radius = kit.knob(kl, { name: 'Calls within', min: 10, max: 300, value: 150, def: 150, log: true, colour: 'var(--blue)', fmt: (v) => `${v.toFixed(0)} m`, onChange: (v) => send({ type: 'radius', radius: v }) });
  const density = kit.knob(kf, { name: 'Density', min: 0, max: 1, value: 0.5, def: 0.5, colour: 'var(--green)',
    fmt: (v) => (v < 0.25 ? 'Woodland' : v < 0.75 ? 'Forest' : 'Thicket') + ` ${(v * 100).toFixed(0)}`, onChange: (v) => send({ type: 'density', density: v }) });
  const variation = kit.knob(kf, { name: 'Variation', min: 0, max: 1, value: 0.5, def: 0.5, colour: 'var(--green)', fmt: (v) => `${(v * 100).toFixed(0)} %`, onChange: (v) => send({ type: 'variation', variation: v }) });
  const comp = kit.knob(kw, { name: 'Compensate', min: 0, max: 1, value: 1, def: 1, colour: 'var(--orange)', fmt: (v) => `${(v * 100).toFixed(0)} %`, onChange: (v) => send({ type: 'compensation', amount: v }) });
  const reverb = kit.knob(ks, { name: 'Reverb', min: 0, max: 1, value: 0.3, def: 0.3, colour: 'var(--purple)', live: true, fmt: (v) => `${(v * 100).toFixed(0)} %`, onChange: () => sendReverb() });
  const volume = kit.knob(ks, { name: 'Volume', min: -40, max: 6, value: 0, def: 0, colour: 'var(--purple)', live: true, fmt: (v) => `${v >= 0 ? '+' : ''}${v.toFixed(1)} dB`, onChange: (v) => send({ type: 'volume', db: v }) });
  const sendReverb = () => send({ type: 'reverb', amount: $('reverb-on').checked ? reverb.value : null });
  $('reverb-on').addEventListener('change', () => { reverb.enable($('reverb-on').checked); sendReverb(); });
  $('seed').addEventListener('change', () => send({ type: 'seed', seed: Math.max(0, Math.floor(Number($('seed').value) || 0)) }));
  $('dice').addEventListener('click', () => { const s = Math.floor(Math.random() * 1e6); $('seed').value = s; send({ type: 'seed', seed: s }); });
  $('medium').addEventListener('change', () => { if ($('medium').value !== 'custom') send({ type: 'medium', medium: $('medium').value }); });
  $('uwstereo').addEventListener('change', () => send({ type: 'underwater_stereo', on: $('uwstereo').checked }));
  for (let i = 0; i < 12; i++) $('root').add(new Option(NOTES[i], NOTES[i]));
  for (const [v, t] of SCALES) $('scale').add(new Option(t, v));
  const sendKey = () => send({ type: 'key', key: keymode.value === 'free' ? null : { root: $('root').value, scale: $('scale').value } });
  const keymode = kit.seg($('keymode'), sendKey);
  $('root').addEventListener('change', () => { keymode.set('key'); sendKey(); });
  $('scale').addEventListener('change', () => { keymode.set('key'); sendKey(); });
  const ear = kit.seg($('ear'), (v) => send({ type: 'ear', ear: v }));
  $('name').addEventListener('change', () => send({ type: 'name', name: $('name').value }));

  let playing = false;
  $('play').addEventListener('click', () => {
    playing = !playing;
    $('play').textContent = playing ? 'Stop' : 'Play';
    $('play').classList.toggle('on', playing);
    send({ type: 'play', on: playing });
  });
  document.addEventListener('keydown', (e) => {
    if (e.code === 'Space' && !['INPUT', 'SELECT'].includes(document.activeElement.tagName)) {
      e.preventDefault();
      $('play').click();
    }
  });

  api.examples().then((list) => list.forEach((n) => $('examples').add(new Option(n, n)))).catch(() => {});
  $('examples').addEventListener('change', async () => {
    const n = $('examples').value;
    $('examples').value = '';
    if (!n) return;
    try { await api.openExample(n); status(`Opened ${n}`); } catch (e) { status(String(e.message || e), true); }
  });
  $('open').addEventListener('click', async () => {
    try { const n = await api.open(); if (n) status(`Opened ${n}`); } catch (e) { status(String(e.message || e), true); }
  });
  $('save').addEventListener('click', async () => {
    try { const p = await api.save(); if (p) status(`Saved ${p}`); } catch (e) { status(String(e.message || e), true); }
  });
  $('export').addEventListener('click', async () => {
    const minutes = Math.max(0.1, Number($('export-min').value) || 10);
    try {
      const p = await api.exportWav(minutes);
      if (!p) return;
      $('export').disabled = true;
      const poll = async () => {
        const s = await api.exportState();
        if (s.running) {
          status(`Rendering… ${((100 * s.done_s) / Math.max(1, s.total_s)).toFixed(0)} %`);
          setTimeout(poll, 500);
        } else {
          $('export').disabled = false;
          if (s.result && s.result.Ok) status(`Rendered ${s.result.Ok}`);
          else if (s.result && s.result.Err) status(s.result.Err, true);
        }
      };
      poll();
    } catch (e) {
      status(String(e.message || e), true);
    }
  });

  // ---------- the forest's settings into the controls

  function legendMarks() {
    document.querySelectorAll('.chip').forEach((c) => c.classList.toggle('following', Number(c.dataset.i) === following && path.value === 'follow'));
  }

  async function loadInfo() {
    info = await api.info();
    const f = info.forest;
    $('name').value = f.name;
    density.set(f.density);
    variation.set(f.tree_variation);
    $('seed').value = f.seed;
    const preset = typeof f.medium === 'string' ? f.medium.toLowerCase() : 'custom';
    if (preset === 'custom' && !$('medium').querySelector('option[value=custom]')) $('medium').add(new Option('Custom', 'custom'));
    $('medium').value = preset;
    $('uwstereo').checked = !!f.underwater_stereo;
    $('uwstereo').disabled = !info.liquid;
    comp.set(f.medium_compensation);
    keymode.set(f.key ? 'key' : 'free');
    if (f.key) {
      $('root').value = NOTES[(NOTES.indexOf(f.key.root.replace('b', '')) + (f.key.root.includes('b') ? 11 : 0)) % 12] || f.key.root;
      $('scale').value = f.key.scale;
    }
    ear.set(f.ear);
    $('reverb-on').checked = f.reverb !== null;
    reverb.enable(f.reverb !== null);
    if (f.reverb !== null) reverb.set(f.reverb);
    volume.set(info.volume);
    if (f.listen.mode === 'box') {
      mode.set('box');
      boxRect = [f.listen.x0, f.listen.x1, f.listen.y0, f.listen.y1];
    } else {
      mode.set('listener');
      radius.set(f.listen.radius);
    }
    path.set(info.path.kind);
    speed.set(info.path.speed);
    following = info.path.kind === 'follow' ? info.path.species : null;
    speciesColours = window.palette.colours(f.species);
    // species: legend and follow list
    const follow = $('follow');
    follow.innerHTML = '';
    follow.add(new Option('—', ''));
    const legend = $('legend');
    legend.innerHTML = '';
    info.species.forEach((s, i) => {
      follow.add(new Option(s.name, String(i)));
      const chip = document.createElement('span');
      chip.className = 'chip';
      chip.dataset.i = i;
      chip.title = `Follow the ${s.name}`;
      chip.innerHTML = `<i style="background:${colour(i)}"></i>${s.name} <small>${s.count} · ${s.movement}</small>`;
      chip.addEventListener('click', () => { follow.value = String(i); path.set('follow'); sendPath(); });
      legend.appendChild(chip);
    });
    follow.value = info.path.species === null ? '' : String(info.path.species);
    legendMarks();
    hint();
    drawBackground();
  }

  // ---------- the loop

  let busy = false;
  async function tick() {
    if (!busy) {
      busy = true;
      try {
        const f = decode(await api.frame());
        if (f) {
          frame = f;
          if (!info || f.version !== info.version) await loadInfo();
          if (f.playing !== playing) {
            playing = f.playing;
            $('play').textContent = playing ? 'Stop' : 'Play';
            $('play').classList.toggle('on', playing);
          }
        }
      } catch (e) {
        /* the engine may be busy rebuilding */
      }
      busy = false;
    }
    drawFrame();
    requestAnimationFrame(tick);
  }
  setInterval(async () => {
    try { const e = await api.error(); if (e) status(e, true); } catch (e) { /* ignore */ }
  }, 2000);

  window.addEventListener('resize', resize);
  resize();
  hint();
  loadInfo().then(() => requestAnimationFrame(tick)).catch((e) => status(String(e.message || e), true));
})();
