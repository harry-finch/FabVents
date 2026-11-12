/**
 * Fan grille: a 120 mm fan plate with its round opening and four concentric
 * bars, joined by four spokes. At rest the bars taper outwards (the logo's
 * perforations, grown towards the centre) and the hub is the one bright mark.
 * The bar under the pointer thins, and its neighbours follow in turn, staggered
 * outwards: the opening widens where the hand is. The slider is the stagger.
 *
 * The pattern: discrete items on tweens, a stagger by distance, and a hit test
 * on static radius bands on the plate's own plane, so a thinning bar cannot
 * flip the choice.
 */
const { Cam, clamp, facing, fit, lerp, open, poly, proj, prism, put, rings, run, ringAt, circ, seg,
  tdone, tset, tval, tween, disposer, mk, pointer, register, solid, unproj } = HL;

const T = 5, HALF = 60, R = 50, HUB = 9, RS = [16, 26, 36, 45], HW0 = [2, 2.6, 3.2, 3.7], HWMIN = 0.9, SPK = 1.4;
const ANG = [45, 135, 225, 315].map((a) => (a * Math.PI) / 180);

function mount({ stage, svg, read }, value) {
  const bag = disposer();
  let stag = value;

  const C = Cam(45, 0.5, 1.9);
  fit(C, [[-HALF, -HALF, 0], [HALF, HALF, 0], [HALF, -HALF, 0], [-HALF, HALF, 0], [-HALF, -HALF, T]], 200, 166);
  const P = proj(C), front = facing(C);

  /** A flat circle on the plate's top face, as a closed path string. */
  const disc = (cx, cy, r) => poly(Array.from({ length: 56 }, (_, k) => P(cx + r * Math.cos((k / 56) * 2 * Math.PI), cy + r * Math.sin((k / 56) * 2 * Math.PI), T)));

  const g = mk("g", {}, svg);
  const [outer, inner] = rings(-HALF, -HALF, HALF, HALF, 9, 1.6);
  put(solid(g), prism(P, front, outer, inner, 0, T));
  // the opening: its rim, and the far wall seen through it
  const wall = run(circ(R, 56), (q) => !front(q));
  mk("path", { d: open(ringAt(P, wall, 0)), class: "nf lo" }, g);
  mk("path", { d: disc(0, 0, R), class: "nf sil" }, g);
  // four screw holes
  for (const [x, y] of [[-51, -51], [51, -51], [-51, 51], [51, 51]]) {
    mk("path", { d: disc(x, y, 3.4), class: "nf" }, g);
    mk("path", { d: disc(x, y, 1.4), class: "nf lo" }, g);
  }

  const spokes = mk("path", { class: "nf" }, g);
  const bars = RS.map((_, i) => ({ el: mk("path", { class: "nf" }, g), o: tween(0), last: -1, i }));
  const hub = mk("path", { d: disc(0, 0, HUB), class: "hi" }, g);
  mk("path", { d: disc(0, 0, 3.2), class: "nf lo" }, g);

  const hw = (b, now) => lerp(HW0[b.i], HWMIN, tval(b.o, now));

  function draw(now) {
    const edges = [HUB];
    bars.forEach((b) => {
      const w = hw(b, now);
      b.el.setAttribute("d", disc(0, 0, RS[b.i] + w) + disc(0, 0, RS[b.i] - w));
      edges.push(RS[b.i] - w, RS[b.i] + w);
    });
    edges.push(R);
    let d = "";
    for (let j = 0; j < edges.length; j += 2) {
      const a = edges[j], b = edges[j + 1];
      if (b - a < 0.2) continue;
      for (const t of ANG) for (const l of [-SPK, SPK]) {
        const p = (s) => P(s * Math.cos(t) - l * Math.sin(t), s * Math.sin(t) + l * Math.cos(t), T);
        d += seg(p(a), p(b));
      }
    }
    spokes.setAttribute("d", d);
  }
  draw(0);

  const B = register(stage, (_dt, now) => {
    let moving = false;
    bars.forEach((b) => { if (!tdone(b.o, now)) moving = true; });
    draw(now);
    return moving;
  });
  bag.add(B.unregister);

  /** The bar whose radius band holds the point, on the plate's rest plane; -1 on the hub or outside. */
  const hit = ([x, y]) => {
    const [wx, wy] = unproj(C, x, y, T), r = Math.hypot(wx, wy);
    return r < HUB + 2 || r > R + 1 ? -1 : clamp(Math.round((r - RS[0]) / 10), 0, RS.length - 1);
  };

  let act = -1;
  function setActive(a) {
    if (a === act) return;
    const now = performance.now(), from = a >= 0 ? a : act;
    act = a;
    bars.forEach((b) => {
      const dist = Math.abs(b.i - from);
      tset(b.o, a < 0 ? 0 : dist === 0 ? 1 : dist === 1 ? 0.45 : 0, now, dist * stag);
      b.el.classList.toggle("hi", b.i === a);
    });
    hub.classList.toggle("hi", a < 0);
    hub.classList.toggle("sil", a >= 0);
    read.textContent = a < 0 ? "rest" : "bar " + (a + 1);
    B.wake();
  }

  bag.add(pointer(stage, { move: (p) => setActive(hit(p)), leave: () => setActive(-1) }));
  bag.add(() => svg.replaceChildren());
  return { set: (v) => { stag = v; }, destroy: bag.dispose };
}

hairline({
  name: "fan-grille",
  means: "A fan grille: the bar under the pointer thins and the opening widens, outwards from it.",
  rules: [1, 2, 4, 8],
  range: [0, 40, 90],
  tour: [[179, 173], [235, 179], [260, 132], null],
  mount,
});
