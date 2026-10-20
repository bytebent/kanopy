// kanopy · the black screen: the forest slice drawn from the engine's info and frames.
// Shared by the Forest and Species pages.
(function () {
  const W = 200, TOP = 50, BOTTOM = -3, SHRUB_TOP = 3;
  const LAYERS = [['Soil', -3, 0], ['Floor', 0, 0.5], ['Shrubs', 0.5, 3], ['Understorey', 3, 5], ['Subcanopy', 5, 15], ['Canopy', 15, 30], ['Emergent', 30, 50]];
  const HEADER = 68, ANIMAL = 12;

  function decode(buf) {
    const dv = new DataView(buf);
    if (dv.byteLength < HEADER || dv.getUint8(0) !== 0x4b) return null;
    return {
      time: dv.getFloat64(4, true), version: dv.getUint32(12, true), playing: dv.getUint8(16) === 1, boxed: dv.getUint8(17) === 1, auditioning: dv.getUint8(18) === 1,
      calling: dv.getUint32(20, true), active: dv.getUint32(24, true), cpu: dv.getFloat32(28, true),
      peak: [dv.getFloat32(32, true), dv.getFloat32(36, true)], lx: dv.getFloat32(40, true), ly: dv.getFloat32(44, true),
      box: [dv.getFloat32(48, true), dv.getFloat32(52, true), dv.getFloat32(56, true), dv.getFloat32(60, true)], ear: dv.getFloat32(64, true),
      n: (dv.byteLength - HEADER) / ANIMAL, dv,
    };
  }

  function create(canvas) {
    const ctx = canvas.getContext('2d');
    const bg = document.createElement('canvas');
    const s = { canvas, ctx, G: { s: 5, x0: 10, y0: 10, w: 1000, h: 330, dpr: 1 }, info: null };
    s.X = (x) => s.G.x0 + x * s.G.s;
    s.Y = (y) => s.G.y0 + (TOP - y) * s.G.s;
    s.toWorld = (px, py) => ({ x: (px - s.G.x0) / s.G.s, y: TOP - (py - s.G.y0) / s.G.s });
    s.clamp = (p) => ({ x: Math.min(W, Math.max(0, p.x)), y: Math.min(TOP, Math.max(BOTTOM, p.y)) });
    s.layerAt = (y) => (LAYERS.find(([, , hi]) => y < hi) || LAYERS[LAYERS.length - 1])[0];
    s.pointer = (e) => {
      const r = canvas.getBoundingClientRect();
      return s.clamp(s.toWorld(e.clientX - r.left, e.clientY - r.top));
    };

    s.resize = () => {
      const r = canvas.getBoundingClientRect();
      const dpr = window.devicePixelRatio || 1;
      canvas.width = Math.round(r.width * dpr);
      canvas.height = Math.round(r.height * dpr);
      const padL = 10, padR = 92, padT = 8, padB = 8;
      const k = Math.min((r.width - padL - padR) / W, (r.height - padT - padB) / (TOP - BOTTOM));
      s.G = { s: k, x0: padL, y0: padT + (r.height - padT - padB - k * (TOP - BOTTOM)) / 2, w: r.width, h: r.height, dpr };
      s.drawBackground();
    };

    s.drawBackground = () => {
      const { G, X, Y, info } = s;
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
        b.fillRect(X(0), Y(TOP), W * G.s, TOP * G.s);
      }
      b.fillStyle = liquid ? 'rgba(90,80,60,.55)' : 'rgba(92,60,30,.6)';
      b.fillRect(X(0), Y(0), W * G.s, -BOTTOM * G.s);
      const full = info ? info.shrubs : 0.5;
      b.fillStyle = `rgba(60,130,60,${0.1 + 0.35 * full})`;
      b.fillRect(X(0), Y(SHRUB_TOP), W * G.s, (SHRUB_TOP - 0.5) * G.s);
      b.font = '9px ' + getComputedStyle(document.body).getPropertyValue('--mono');
      b.textBaseline = 'middle';
      // layer labels at each layer's middle, pushed apart where layers are thin
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
    };

    s.clear = () => {
      ctx.setTransform(s.G.dpr, 0, 0, s.G.dpr, 0, 0);
      ctx.clearRect(0, 0, s.G.w, s.G.h);
      ctx.drawImage(bg, 0, 0, s.G.w, s.G.h);
    };

    /** Animals: colours[species]; `only` (a species index) draws the others faint. */
    s.drawAnimals = (frame, colours, only) => {
      const { X, Y, G } = s;
      const dv = frame.dv;
      const r = Math.max(1, G.s * 0.28);
      const calling = [];
      for (let i = 0; i < frame.n; i++) {
        const o = HEADER + i * ANIMAL;
        const flags = dv.getUint8(o + 10);
        const x = dv.getFloat32(o, true), y = dv.getFloat32(o + 4, true), sp = dv.getUint16(o + 8, true);
        const dim = only != null && only >= 0 && sp !== only;
        if (flags & 2 && !dim) { calling.push(x, y, sp); continue; }
        ctx.globalAlpha = dim ? 0.12 : only != null && only >= 0 ? 0.9 : flags & 1 ? 0.75 : 0.22;
        ctx.fillStyle = colours[sp] || '#888';
        ctx.fillRect(X(x) - r, Y(y) - r, 2 * r, 2 * r);
      }
      for (let k = 0; k < calling.length; k += 3) {
        const px = X(calling[k]), py = Y(calling[k + 1]);
        ctx.fillStyle = colours[calling[k + 2]] || '#888';
        ctx.globalAlpha = 0.28;
        ctx.beginPath();
        ctx.arc(px, py, r * 4, 0, Math.PI * 2);
        ctx.fill();
        ctx.globalAlpha = 1;
        ctx.beginPath();
        ctx.arc(px, py, r * 1.6, 0, Math.PI * 2);
        ctx.fill();
      }
      ctx.globalAlpha = 1;
    };

    s.drawListener = (x, y, ghost) => {
      const px = s.X(x), py = s.Y(y), k = Math.max(1, s.G.s / 5);
      ctx.globalAlpha = ghost ? 0.4 : 1;
      ctx.fillStyle = '#5cc8f5';
      ctx.beginPath();
      ctx.arc(px, py - 9 * k, 2.6 * k, 0, Math.PI * 2);
      ctx.fill();
      ctx.fillRect(px - 2.4 * k, py - 6 * k, 4.8 * k, 6 * k);
      ctx.fillRect(px - 2.4 * k, py, 1.8 * k, 4 * k);
      ctx.fillRect(px + 0.6 * k, py, 1.8 * k, 4 * k);
      ctx.globalAlpha = 1;
    };

    /** A dashed rectangle with corner handles: [x0, x1, y0, y1]. */
    s.drawRect = (rect, colour, fill) => {
      const [x0, x1, y0, y1] = rect;
      const { X, Y, G } = s;
      ctx.strokeStyle = colour;
      ctx.setLineDash([5, 4]);
      ctx.lineWidth = 1.5;
      ctx.strokeRect(X(x0), Y(y1), (x1 - x0) * G.s, (y1 - y0) * G.s);
      ctx.setLineDash([]);
      if (fill) {
        ctx.fillStyle = fill;
        ctx.fillRect(X(x0), Y(y1), (x1 - x0) * G.s, (y1 - y0) * G.s);
      }
      ctx.fillStyle = colour;
      for (const [cx, cy] of [[x0, y0], [x1, y0], [x0, y1], [x1, y1]]) ctx.fillRect(X(cx) - 3, Y(cy) - 3, 6, 6);
    };

    /**
     * Dragging a rectangle: call with pointerdown's world point and the current rect (or null);
     * returns a drag object whose move(p) gives the new rect.
     */
    s.rectDrag = (p, rect) => {
      const near = (a, b) => Math.abs(a - b) * s.G.s < 8;
      const norm = (b) => [Math.min(b[0], b[1]), Math.max(b[0], b[1]), Math.min(b[2], b[3]), Math.max(b[2], b[3])];
      if (rect) {
        const [x0, x1, y0, y1] = rect;
        for (const [cx, cy, kx, ky] of [[x0, y0, 0, 2], [x1, y0, 1, 2], [x0, y1, 0, 3], [x1, y1, 1, 3]]) {
          if (near(p.x, cx) && near(p.y, cy)) {
            const d = { kx, ky, rect: [...rect] };
            d.move = (q) => {
              const b = [...d.rect];
              b[d.kx] = q.x;
              b[d.ky] = q.y;
              d.rect = norm(b);
              d.kx = d.rect[0] === b[d.kx] ? 0 : 1;
              d.ky = d.rect[2] === b[d.ky] ? 2 : 3;
              return d.rect;
            };
            return d;
          }
        }
        if (p.x > x0 && p.x < x1 && p.y > y0 && p.y < y1) {
          return {
            move: (q) => {
              let dx = q.x - p.x, dy = q.y - p.y;
              dx = Math.min(W - x1, Math.max(-x0, dx));
              dy = Math.min(TOP - y1, Math.max(BOTTOM - y0, dy));
              return [x0 + dx, x1 + dx, y0 + dy, y1 + dy];
            },
          };
        }
      }
      return { move: (q) => norm([p.x, q.x, p.y, q.y]) };
    };
    return s;
  }

  window.screen2 = { create, decode, W, TOP, BOTTOM, LAYERS };
})();
