// kanopy · species colours. Twelve hues (the six colours and the in-betweens), each dark,
// normal and light, and three neutrals. A species without a colour of its own gets the next one
// not already taken, in a fixed order. Two species may share a colour.
(function () {
  const BASE = ['#61bb46', '#fdb827', '#f5821f', '#e03a3e', '#963d97', '#009ddc'];
  const rgb = (h) => [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16));
  const hex = (c) => '#' + c.map((v) => Math.round(Math.max(0, Math.min(255, v))).toString(16).padStart(2, '0')).join('');
  const mix = (a, b, t) => a.map((v, i) => v + (b[i] - v) * t);
  const hues = [];
  BASE.forEach((h, i) => {
    hues.push(h);
    hues.push(hex(mix(rgb(h), rgb(BASE[(i + 1) % BASE.length]), 0.5)));
  });
  // columns: [dark, normal, light] for each hue, around the wheel
  const columns = hues.map((h) => [hex(mix(rgb(h), [0, 0, 0], 0.38)), h, hex(mix(rgb(h), [255, 255, 255], 0.45))]);
  const neutrals = ['#6e6e78', '#c8c8ce', '#f4f4f6'];
  const order = [...BASE, ...hues.filter((_, i) => i % 2), ...columns.map((c) => c[2]), ...columns.map((c) => c[0]), ...neutrals];

  /** Colours for a list of species: their own, or the next free one in order. */
  function colours(species) {
    const taken = new Set(species.map((s) => s.colour).filter(Boolean));
    let k = 0;
    return species.map((s) => {
      if (s.colour) return s.colour;
      while (k < order.length && taken.has(order[k])) k++;
      const c = order[k % order.length];
      taken.add(c);
      k++;
      return c;
    });
  }

  window.palette = { columns, neutrals, order, colours };
})();
