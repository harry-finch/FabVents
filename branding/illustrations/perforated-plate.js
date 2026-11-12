/**
 * Perforated plate: a square plate with a 7 × 7 grid of round holes, bigger
 * towards the centre like the logo's perforations. The pointer swells the
 * holes around it, the farther the less, and they shrink back when it leaves.
 * At rest the centre hole is the one bright mark; under the pointer the
 * bright goes to the nearest hole. The slider is the reach, in plate units.
 *
 * The pattern: a field. One spring per pointer coordinate, a falloff to a
 * floor, every radius clamped, and the pick read from the spring's target on
 * the plate's own plane, so a swelling hole cannot flip the choice.
 */
const { Cam, clamp, facing, fit, poly, proj, prism, put, rings, solid, unproj, spring, stepS,
  disposer, mk, pointer, register } = HL;

const T = 5, HALF = 58, N = 7, PITCH = 14, RMAX = 6.6, RMIN = 1.2, GAIN = 5.6;

function mount({ stage, svg, read }, value) {
  const bag = disposer();
  let reach = value;

  const C = Cam(45, 0.5, 1.9);
  fit(C, [[-HALF, -HALF, 0], [HALF, HALF, 0], [HALF, -HALF, 0], [-HALF, HALF, 0], [-HALF, -HALF, T]], 200, 166);
  const P = proj(C), front = facing(C);

  const g = mk("g", {}, svg);
  const [outer, inner] = rings(-HALF, -HALF, HALF, HALF, 9, 1.6);
  put(solid(g), prism(P, front, outer, inner, 0, T));

  /** A flat circle on the plate's top face, as a closed path string. */
  const disc = (cx, cy, r) => poly(Array.from({ length: 28 }, (_, k) => P(cx + r * Math.cos((k / 28) * 2 * Math.PI), cy + r * Math.sin((k / 28) * 2 * Math.PI), T)));

  // holes, back to front; each rests at a radius that grows towards the centre
  const holes = [];
  for (let j = 0; j < N; j++) for (let i = 0; i < N; i++) {
    const x = (i - 3) * PITCH, y = (j - 3) * PITCH;
    const rest = RMIN + 2.0 * (1 - Math.hypot(x, y) / (3 * PITCH * Math.SQRT2));
    holes.push({ i, j, x, y, rest, last: -1, el: mk("path", { class: "nf" }, g) });
  }
  const mid = holes[24].el;
  mid.classList.add("hi");

  const px = spring(0), py = spring(0), on = spring(0);

  function draw() {
    for (const h of holes) {
      const f = clamp(1 - Math.hypot(h.x - px.x, h.y - py.x) / reach, 0, 1);
      const r = clamp(h.rest + GAIN * on.x * f, RMIN, RMAX);
      if (Math.abs(r - h.last) < 0.004) continue;
      h.last = r;
      h.el.setAttribute("d", disc(h.x, h.y, r));
    }
  }
  draw();

  const B = register(stage, (dt) => {
    const m = [stepS(px, dt), stepS(py, dt), stepS(on, dt)].some(Boolean);
    draw();
    return m;
  });
  bag.add(B.unregister);

  /** The hole nearest the pointer, on the plate's rest plane. */
  const hole = ([x, y]) => {
    const [wx, wy] = unproj(C, x, y, T);
    return { wx, wy, k: clamp(Math.round(wy / PITCH) + 3, 0, N - 1) * N + clamp(Math.round(wx / PITCH) + 3, 0, N - 1) };
  };

  let act = -1;
  function setActive(a, wx, wy) {
    if (a >= 0) {
      if (act < 0) { px.x = wx; py.x = wy; }
      px.t = wx; py.t = wy;
    }
    on.t = a >= 0 ? 1 : 0;
    if (a !== act) {
      holes.forEach((h, k) => h.el.classList.toggle("hi", a >= 0 ? k === a : k === 24));
      read.textContent = a < 0 ? "rest" : `hole ${(a % N) + 1}·${Math.floor(a / N) + 1}`;
      act = a;
    }
    B.wake();
  }

  bag.add(pointer(stage, {
    move: (p) => {
      const { wx, wy, k } = hole(p);
      if (Math.abs(wx) > HALF || Math.abs(wy) > HALF) setActive(-1);
      else setActive(k, wx, wy);
    },
    leave: () => setActive(-1),
  }));
  bag.add(() => svg.replaceChildren());
  return { set: (v) => { reach = v; B.wake(); }, destroy: bag.dispose };
}

hairline({
  name: "perforated-plate",
  means: "A perforated plate: the holes around the pointer swell, the farther the less.",
  rules: [1, 3, 4, 8],
  range: [26, 38, 54],
  tour: [[256, 152], [219, 190], [200, 166], null],
  mount,
});
