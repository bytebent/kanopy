// kanopy · the Forest page.
(function () {
  const api = window.kanopy;
  const kit = window.kit;
  const W = 200, TOP = 50, BOTTOM = -3, SHRUB_TOP = 3;
  const LAYERS = [['Soil', -3, 0], ['Floor', 0, 0.5], ['Shrubs', 0.5, 3], ['Understorey', 3, 5], ['Subcanopy', 5, 15], ['Canopy', 15, 30], ['Emergent', 30, 50]];
  const BASE = ['#61bb46', '#fdb827', '#f5821f', '#e03a3e', '#c86ad0', '#009ddc', '#d9d9d9', '#9be36b'];
  const colour = (i) => (i < BASE.length ? BASE[i] : `hsl(${(i * 137.508) % 360} 70% 60%)`);
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

  // ---------- screen geometry

  const canvas = $('scene');
  const ctx = canvas.getContext('2d');
  const bg = document.createElement('canvas');
  let G = { s: 5, x0: 10, y0: 10, w: 1000, h: 330, dpr: 1 };
  const X = (x) => G.x0 + x * G.s;
  const Y = (y) => G.y0 + (TOP - y) * G.s;
  const toWorld = (px, py) => ({ x: (px - G.x0) / G.s, y: TOP - (py - G.y0) / G.s });
  const layerAt = (y) => (LAYERS.find(([, lo, hi]) => y < hi) || LAYERS[LAYERS.length - 1])[0];

  function resize() {
    const r = canvas.getBoundingClientRect();
    const dpr = window.devicePixelRatio || 1;
    canvas.width = Math.round(r.width * dpr);
    canvas.height = Math.round(r.height * dpr);
    const padL = 10, padR = 92, padT = 8, padB = 8;
    const s = Math.min((r.width - padL - padR) / W, (r.height - padT - padB) / (TOP - BOTTOM));
    G = { s, x0: padL, y0: padT + (r.height - padT - padB - s * (TOP - BOTTOM)) / 2, w: r.width, h: r.height, dpr };
    drawBackground();
  }

  function drawBackground() {
    bg.width = canvas.width;
    bg.height = canvas.height;
    const b = bg.getContext('2d');
    b.setTransform(G.dpr, 0, 0, G.dpr, 0, 0);
    const liquid = info && info.liquid;
    b.fillStyle = liquid ? '#03101a' : '#060608';
    b.fillRect(0, 0, G.w, G.h);
    if (liquid) {
      const g = b.createLinearGradient(0, Y(TOP), 0, Y(0));
      g.addColorStop(0, 'rgba(40,120,170,.22)');
      g.addColorStop(1, 'rgba(10,40,70,.10)');
      b.fillStyle = g;
      b.fillRect(X(0), Y(TOP), W * G.s, (TOP) * G.s);
    }
    // soil and shrubs
    b.fillStyle = liquid ? 'rgba(90,80,60,.55)' : 'rgba(92,60,30,.6)';
    b.fillRect(X(0), Y(0), W * G.s, -BOTTOM * G.s);
    const full = info ? info.shrubs : 0.5;
    b.fillStyle = `rgba(60,130,60,${0.1 + 0.35 * full})`;
    b.fillRect(X(0), Y(SHRUB_TOP), W * G.s, (SHRUB_TOP - 0.5) * G.s);
    // layer guides
    b.font = '9px ' + getComputedStyle(document.body).getPropertyValue('--mono');
    b.textBaseline = 'middle';
    // labels at each layer's middle, pushed apart where layers are thin
    let last = -Infinity;
    for (const [name, lo, hi] of [...LAYERS].reverse()) {
      b.strokeStyle = 'rgba(136,255,136,.13)';
      b.setLineDash([3, 4]);
      b.beginPath();
      b.moveTo(X(0), Y(hi) + 0.5);
      b.lineTo(X(W), Y(hi) + 0.5);
      b.stroke();
      b.setLineDash([]);
      b.fillStyle = 'rgba(136,255,136,.55)';
      const y = Math.max((Y(lo) + Y(hi)) / 2, last + 10);
      last = y;
      b.fillText(name.toUpperCase(), X(W) + 8, y);
    }
    if (!info) return;
    // trees (kelp in water)
    for (const [x, h, cb, hw] of info.trees) {
      if (liquid) {
        b.strokeStyle = 'rgba(120,170,90,.55)';
        b.lineWidth = 1.5;
        b.beginPath();
        for (let y = 0; y <= h; y += 0.5) {
          const sx = X(x + Math.sin(y * 0.35 + x) * 0.8);
          y === 0 ? b.moveTo(sx, Y(y)) : b.lineTo(sx, Y(y));
        }
        b.stroke();
        b.fillStyle = 'rgba(90,150,80,.18)';
        b.beginPath();
        b.ellipse(X(x), Y((cb + h) / 2), hw * 0.55 * G.s, ((h - cb) / 2) * G.s, 0, 0, Math.PI * 2);
        b.fill();
      } else {
        b.strokeStyle = 'rgba(120,85,50,.85)';
        b.lineWidth = Math.max(1.2, 0.35 * G.s);
        b.beginPath();
        b.moveTo(X(x), Y(0));
        b.lineTo(X(x), Y(cb + (h - cb) * 0.35));
        b.stroke();
        b.fillStyle = 'rgba(46,94,42,.42)';
        b.strokeStyle = 'rgba(110,190,100,.28)';
        b.lineWidth = 1;
        b.beginPath();
        b.ellipse(X(x), Y((cb + h) / 2), hw * G.s, ((h - cb) / 2) * G.s, 0, 0, Math.PI * 2);
        b.fill();
        b.stroke();
      }
    }
  }

  // ---------- frames

  function decode(buf) {
    const dv = new DataView(buf);
    if (dv.byteLength < 68 || dv.getUint8(0) !== 0x4b) return null;
    const f = {
      time: dv.getFloat64(4, true), version: dv.getUint32(12, true), playing: dv.getUint8(16) === 1, boxed: dv.getUint8(17) === 1,
      calling: dv.getUint32(20, true), active: dv.getUint32(24, true), cpu: dv.getFloat32(28, true),
      peak: [dv.getFloat32(32, true), dv.getFloat32(36, true)], lx: dv.getFloat32(40, true), ly: dv.getFloat32(44, true),
      box: [dv.getFloat32(48, true), dv.getFloat32(52, true), dv.getFloat32(56, true), dv.getFloat32(60, true)], ear: dv.getFloat32(64, true),
      n: (dv.byteLength - 68) / 12, dv,
    };
    return f;
  }

  function drawListener(x, y, ghost) {
    const px = X(x), py = Y(y), s = Math.max(1, G.s / 5);
    ctx.globalAlpha = ghost ? 0.4 : 1;
    ctx.fillStyle = '#5cc8f5';
    ctx.beginPath();
    ctx.arc(px, py - 9 * s, 2.6 * s, 0, Math.PI * 2);
    ctx.fill();
    ctx.fillRect(px - 2.4 * s, py - 6 * s, 4.8 * s, 6 * s);
    ctx.fillRect(px - 2.4 * s, py, 1.8 * s, 4 * s);
    ctx.fillRect(px + 0.6 * s, py, 1.8 * s, 4 * s);
    ctx.globalAlpha = 1;
  }

  function drawFrame() {
    ctx.setTransform(G.dpr, 0, 0, G.dpr, 0, 0);
    ctx.clearRect(0, 0, G.w, G.h);
    ctx.drawImage(bg, 0, 0, G.w, G.h);
    if (!frame) return;
    const dv = frame.dv;
    const n = frame.n;
    // animals: quiet ones first, calling ones on top with a glow
    const r = Math.max(1, G.s * 0.28);
    const calling = [];
    for (let i = 0; i < n; i++) {
      const o = 68 + i * 12;
      const flags = dv.getUint8(o + 10);
      const x = dv.getFloat32(o, true), y = dv.getFloat32(o + 4, true), sp = dv.getUint16(o + 8, true);
      if (flags & 2) { calling.push(x, y, sp); continue; }
      ctx.globalAlpha = flags & 1 ? 0.75 : 0.22;
      ctx.fillStyle = colour(sp);
      ctx.fillRect(X(x) - r, Y(y) - r, 2 * r, 2 * r);
    }
    for (let k = 0; k < calling.length; k += 3) {
      const px = X(calling[k]), py = Y(calling[k + 1]);
      ctx.globalAlpha = 0.28;
      ctx.fillStyle = colour(calling[k + 2]);
      ctx.beginPath();
      ctx.arc(px, py, r * 4, 0, Math.PI * 2);
      ctx.fill();
      ctx.globalAlpha = 1;
      ctx.beginPath();
      ctx.arc(px, py, r * 1.6, 0, Math.PI * 2);
      ctx.fill();
    }
    ctx.globalAlpha = 1;
    // listener, or the box
    if (frame.boxed || mode.value === 'box') {
      const b = drag && drag.kind !== 'listener' && boxRect ? boxRect : frame.boxed ? frame.box : boxRect;
      if (b) {
        const [x0, x1, y0, y1] = b;
        ctx.strokeStyle = '#fdb827';
        ctx.setLineDash([5, 4]);
        ctx.lineWidth = 1.5;
        ctx.strokeRect(X(x0), Y(y1), (x1 - x0) * G.s, (y1 - y0) * G.s);
        ctx.setLineDash([]);
        ctx.fillStyle = 'rgba(253,184,39,.06)';
        ctx.fillRect(X(x0), Y(y1), (x1 - x0) * G.s, (y1 - y0) * G.s);
        ctx.fillStyle = '#fdb827';
        for (const [cx, cy] of [[x0, y0], [x1, y0], [x0, y1], [x1, y1]]) ctx.fillRect(X(cx) - 3, Y(cy) - 3, 6, 6);
      }
    } else {
      const radius = info && info.forest.listen.mode === 'listener' ? info.forest.listen.radius : 150;
      ctx.strokeStyle = 'rgba(92,200,245,.18)';
      ctx.setLineDash([2, 5]);
      ctx.beginPath();
      ctx.arc(X(frame.lx), Y(frame.ly), radius * G.s, 0, Math.PI * 2);
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
  const clampW = (p) => ({ x: Math.min(W, Math.max(0, p.x)), y: Math.min(TOP, Math.max(BOTTOM, p.y)) });
  function sendBox(force) {
    const now = performance.now();
    if (!force && now - lastSend < 150) return;
    lastSend = now;
    const [x0, x1, y0, y1] = boxRect;
    if (x1 - x0 > 1 && y1 - y0 > 1) send({ type: 'hear', rect: [x0, x1, y0, y1] });
  }
  canvas.addEventListener('pointerdown', (e) => {
    const r = canvas.getBoundingClientRect();
    const p = clampW(toWorld(e.clientX - r.left, e.clientY - r.top));
    canvas.setPointerCapture(e.pointerId);
    if (mode.value === 'listener') {
      drag = { kind: 'listener', ...p };
      send({ type: 'listener', x: p.x, y: p.y });
      return;
    }
    const near = (a, b) => Math.abs(a - b) * G.s < 8;
    if (boxRect) {
      const [x0, x1, y0, y1] = boxRect;
      for (const [cx, cy, kx, ky] of [[x0, y0, 0, 2], [x1, y0, 1, 2], [x0, y1, 0, 3], [x1, y1, 1, 3]]) {
        if (near(p.x, cx) && near(p.y, cy)) {
          drag = { kind: 'corner', kx, ky };
          return;
        }
      }
      if (p.x > x0 && p.x < x1 && p.y > y0 && p.y < y1) {
        drag = { kind: 'move', from: p, start: [...boxRect] };
        return;
      }
    }
    drag = { kind: 'new', from: p };
    boxRect = [p.x, p.x, p.y, p.y];
  });
  canvas.addEventListener('pointermove', (e) => {
    const r = canvas.getBoundingClientRect();
    const p = clampW(toWorld(e.clientX - r.left, e.clientY - r.top));
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
    } else if (drag.kind === 'new') {
      boxRect = [Math.min(drag.from.x, p.x), Math.max(drag.from.x, p.x), Math.min(drag.from.y, p.y), Math.max(drag.from.y, p.y)];
      sendBox(false);
    } else if (drag.kind === 'move') {
      const [x0, x1, y0, y1] = drag.start;
      let dx = p.x - drag.from.x, dy = p.y - drag.from.y;
      dx = Math.min(W - x1, Math.max(-x0, dx));
      dy = Math.min(TOP - y1, Math.max(BOTTOM - y0, dy));
      boxRect = [x0 + dx, x1 + dx, y0 + dy, y1 + dy];
      sendBox(false);
    } else if (drag.kind === 'corner') {
      const b = [...boxRect];
      b[drag.kx] = p.x;
      b[drag.ky] = p.y;
      boxRect = [Math.min(b[0], b[1]), Math.max(b[0], b[1]), Math.min(b[2], b[3]), Math.max(b[2], b[3])];
      drag.kx = boxRect[0] === b[drag.kx] ? 0 : 1;
      drag.ky = boxRect[2] === b[drag.ky] ? 2 : 3;
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
