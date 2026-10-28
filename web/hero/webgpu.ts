// Tier A: WebGPU. A compute shader advances every particle (same rules as the WebGL2 shaders in
// shaders.ts and the CPU twin in canvas.ts — keep the three in step), then particles and the named
// stars are drawn as instanced quads (WebGPU has no point sizes).

import type { Frame, Renderer } from './frame.ts';
import { EDGES, FIELD_SHARE, GROW, HALF_CHANNEL, LANES, LOGO, MINTAKA_S, NB, THREAT_SHARE, U, type Edge, type V2 } from './geometry.ts';
import { STRIDE } from './particles.ts';

const f = (n: number) => (Number.isInteger(n) ? `${n}.0` : `${+n.toFixed(6)}`);
const e3 = (e: Edge) => `vec3f(${f(e[0])}, ${f(e[1])}, ${f(e[2])})`;
const corner = (p: V2, c: V2) => `vec2f(${f(c[0] + (p[0] - c[0]) * 0.82)}, ${f(c[1] + (p[1] - c[1]) * 0.82)})`;
const T: V2 = [-2, -48.5];
const B: V2 = [2, 48.5];
const L = LOGO as Record<string, V2>;

/** Uniform block: 8 vec4 of scalars, then the arrays. Must match `writeFrame` below. */
const WGSL_COMMON = /* wgsl */ `
struct Particle { pos: vec2f, vel: vec2f, seed: f32, phase: f32, life: f32, spare: f32 };
struct Uni {
  a: vec4f,        // time, dt, flowAlpha, count
  fig: vec4f,      // ox, oy, scale, _
  w: vec4f,        // flow, ray, orion, alpha
  ray: vec4f,      // x, y, width, height
  ptr: vec4f,      // x, y, strength, _
  tap: vec4f,      // x, y, t0, on
  view: vec4f,     // width, height, scrollX, scrollY
  par: vec4f,      // parallax x, y
  orion: array<vec4f, 8>,
  cdf: array<vec4f, 2>,
  lines: array<vec4f, 8>,
  stars: array<vec4f, 8>,
  starCol: array<vec4f, 8>,
};
@group(0) @binding(0) var<uniform> u: Uni;

const UB = vec2f(${f(U[0])}, ${f(U[1])});
const NBV = vec2f(${f(NB[0])}, ${f(NB[1])});
const E_TOP = ${e3(EDGES.top)};
const E_RT = ${e3(EDGES.rightT)};
const E_LT = ${e3(EDGES.leftT)};
const E_LB = ${e3(EDGES.leftB)};
const E_RB = ${e3(EDGES.rightB)};
const E_BOT = ${e3(EDGES.bottom)};
const GROW = ${f(GROW)};
const HALF = ${f(HALF_CHANNEL)};
const MINTAKA_S = ${f(MINTAKA_S)};
const FIELD = ${f(FIELD_SHARE)};
const THREAT = ${f(FIELD_SHARE + THREAT_SHARE)};

fn h1(n: f32) -> f32 { return fract(sin(n * 91.3458 + 17.17) * 47453.5453); }
fn ed(e: vec3f, p: vec2f) -> f32 { return dot(e.xy, p) - e.z; }
fn cdfAt(i: i32) -> f32 { return u.cdf[i / 4][i % 4]; }
fn pickStar(r: f32) -> i32 {
  var j = 7;
  for (var i = 0; i < 8; i++) { if (r <= cdfAt(i)) { j = i; break; } }
  return j;
}
`;

const COMPUTE = /* wgsl */ `${WGSL_COMMON}
@group(0) @binding(1) var<storage, read_write> ps: array<Particle>;

fn sdTop(p: vec2f) -> f32 { return max(max(ed(E_TOP, p), ed(E_RT, p)), max(ed(E_LT, p), dot(NBV, p))) - GROW; }
fn sdBot(p: vec2f, open: bool) -> f32 {
  let s = max(max(ed(E_LB, p), ed(E_RB, p)), -dot(NBV, p));
  return select(max(s, ed(E_BOT, p)), s, open) - GROW;
}
fn sdV(p: vec2f, open: bool) -> f32 { return min(sdTop(p), sdBot(p, open)); }
fn gradV(p: vec2f, open: bool) -> vec2f {
  let e = 0.6;
  return vec2f(sdV(p + vec2f(e, 0.0), open) - sdV(p - vec2f(e, 0.0), open),
               sdV(p + vec2f(0.0, e), open) - sdV(p - vec2f(0.0, e), open)) / (2.0 * e);
}
fn bottomY(x: f32) -> f32 { return (E_BOT.z + GROW - E_BOT.x * x) / E_BOT.y; }
fn curl(p: vec2f, t: f32) -> vec2f {
  let a = 0.045; let b = 0.061; let c = 0.033;
  let dpx = a * cos(p.x * a + t * 0.7) * cos(p.y * b - t * 0.5) + 0.6 * c * cos(p.x * c - p.y * a + t * 0.9);
  let dpy = -b * sin(p.x * a + t * 0.7) * sin(p.y * b - t * 0.5) - 0.6 * a * cos(p.x * c - p.y * a + t * 0.9);
  return vec2f(dpy, -dpx) * 22.0;
}
fn spawn(seed: f32, t: f32) -> vec2f {
  let r1 = h1(seed * 13.1 + floor(t * 0.37)); let r2 = h1(seed * 7.7 + floor(t * 0.53));
  let ang = mix(-2.6, -0.55, r1); let rad = mix(72.0, 118.0, r2);
  return vec2f(-4.0, -62.0) + vec2f(cos(ang), sin(ang)) * rad;
}
fn fillT(uu: f32, v: f32) -> vec2f {
  let a = ${corner(L['betelgeuse']!, T)}; let b = ${corner(L['bellatrix']!, T)}; let c = ${corner(L['mintaka']!, T)}; let d = ${corner(L['alnitak']!, T)};
  return mix(mix(a, b, uu), mix(d, c, uu), v * 0.78);
}
fn fillB(uu: f32, v: f32) -> vec2f {
  let a = ${corner(L['alnitak']!, B)}; let b = ${corner(L['mintaka']!, B)}; let c = ${corner(L['rigel']!, B)}; let d = ${corner(L['saiph']!, B)};
  return mix(mix(a, b, uu), mix(d, c, uu), 0.22 + v * 0.74);
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3u) {
  let i = gid.x;
  if (i >= u32(u.a.w)) { return; }
  var q = ps[i];
  let seed = q.seed;
  if (seed < FIELD) { return; }
  let time = u.a.x; let dt = u.a.y;
  var P = q.pos; var V = q.vel; var phase = q.phase; var life = q.life + dt;
  let threat = seed < THREAT;
  let k = i32(floor(h1(seed * 3.7) * 4.0));
  var lanes = array<f32, 4>(${LANES.map(f).join(', ')});
  let lane = lanes[k];
  var acc = vec2f(0.0);
  let sc = u.fig.z;

  if (u.w.x > 0.001) {
    var p = (P - u.fig.xy) / sc; var v = V / sc; var a = vec2f(0.0);
    if (phase < 0.5) {
      let tgt = fillT(h1(seed * 1.7), h1(seed * 2.9));
      a = (tgt - p) * 2.6 - v * 2.2 + curl(p, time) * 1.2;
      if (sdTop(p) < 0.0 && dot(NBV, p) < -HALF && length(tgt - p) < 9.0) { phase = 1.0; life = 0.0; v *= 0.5; }
    } else if (phase < 1.5) {
      a = vec2f(0.0, 13.0) + curl(p, time) * 2.2 - v * 2.1;
      if (dot(NBV, p) > -HALF) {
        if (threat) { phase = 5.0; life = 0.0; v *= 0.1; } else { phase = 2.0; life = 0.0; }
      }
    } else if (phase < 2.5) {
      let d = dot(NBV, p);
      a = (UB * 112.0 - v) * 7.0 - NBV * d * 30.0;
      if (dot(UB, p) > MINTAKA_S + 2.0) { phase = 3.0; life = 0.0; }
    } else if (phase < 3.5) {
      if (life < 1.5) {
        a = (fillB(h1(seed * 3.1), h1(seed * 4.3)) - p) * 3.0 - v * 2.4;
      } else {
        a = vec2f(0.0, 15.0) + curl(p, time * 0.6) * 0.8 - v * vec2f(1.2, 1.6);
        a.x += (lane - p.x) * 4.0 * smoothstep(70.0, 104.0, p.y);
      }
      if (p.y > bottomY(p.x) - 1.5) { phase = 4.0; life = 0.0; }
    } else if (phase < 4.5) {
      a = vec2f((lane - p.x) * 9.0 - v.x * 5.0, 62.0 - v.y * 0.4);
      if (p.y > bottomY(lane) + 120.0 || life > 3.6) { phase = 0.0; life = 0.0; p = spawn(seed, time); v = vec2f(0.0); }
    } else {
      v *= 0.82;
      if (life > 1.1) { phase = 0.0; life = 0.0; p = spawn(seed, time); v = vec2f(0.0); }
    }
    if (phase > 0.5 && phase < 3.5) {
      let open = phase > 2.5;
      let s = sdV(p, open);
      if (s > -2.0) { a -= normalize(gradV(p, open) + vec2f(1e-5)) * (s + 2.0) * 160.0; }
    }
    P = u.fig.xy + p * sc; V = v * sc;
    acc += a * sc * u.w.x;
  }

  let wt = u.w.y + u.w.z;
  if (wt > 0.001) {
    var tgt = vec2f(0.0);
    if (u.w.y > 0.0) {
      let along = fract(h1(seed * 5.3) + time * (0.06 + 0.1 * h1(seed * 9.1)));
      let lens = 1.0 - abs(2.0 * along - 1.0);
      let rt = vec2f(u.ray.x + along * u.ray.z, u.ray.y + (h1(seed * 2.9) - 0.5) * 14.0 * lens);
      if (along < 0.04 && P.x > u.ray.x + u.ray.z * 0.75 && u.w.y > 0.5) { P = rt; V = vec2f(u.ray.z * 0.1, 0.0); }
      tgt += rt * u.w.y;
    }
    if (u.w.z > 0.0) {
      var ot: vec2f;
      if (h1(seed * 4.1) < 0.12) {
        let l = u.lines[i32(floor(h1(seed * 6.3) * 8.0))];
        ot = mix(l.xy, l.zw, h1(seed * 8.7)) + (vec2f(h1(seed * 1.3), h1(seed * 2.1)) - 0.5) * 2.0;
      } else {
        let st = u.orion[pickStar(h1(seed * 7.9))];
        let ang = h1(seed * 3.3) * 6.2832 + time * (0.12 + 0.2 * h1(seed * 5.9));
        let rad = st.z * (0.35 + 2.6 * pow(h1(seed * 4.7), 2.0));
        ot = st.xy + vec2f(cos(ang), sin(ang)) * rad;
      }
      tgt += ot * u.w.z;
    }
    tgt /= wt;
    acc += ((tgt - P) * 26.0 - V * 7.0) * min(wt, 1.0);
  }

  if (u.ptr.z > 0.0) {
    let d = P - u.ptr.xy; let r = length(d) + 1e-3;
    if (r < 170.0) { acc -= d / r * u.ptr.z * 1100.0 * pow(1.0 - r / 170.0, 2.0); }
  }
  if (u.tap.w > 0.5) {
    let age = time - u.tap.z; let d = P - u.tap.xy; let r = length(d) + 1e-3;
    let band = exp(-pow((r - age * 720.0) / 42.0, 2.0));
    acc += d / r * band * 3200.0 * max(0.0, 1.0 - age / 1.3);
  }

  V += acc * dt;
  P += V * dt;
  q.pos = P; q.vel = V; q.phase = phase; q.life = life;
  ps[i] = q;
}
`;

const DRAW = /* wgsl */ `${WGSL_COMMON}
@group(0) @binding(1) var<storage, read> ps: array<Particle>;
struct VOut { @builtin(position) pos: vec4f, @location(0) uv: vec2f, @location(1) col: vec4f, @location(2) r: f32 };
const CORNERS = array<vec2f, 6>(vec2f(-1, -1), vec2f(1, -1), vec2f(1, 1), vec2f(-1, -1), vec2f(1, 1), vec2f(-1, 1));
const STARLIGHT = vec3f(0.957, 0.961, 0.969);
const MIST = vec3f(0.788, 0.808, 0.847);
const STEEL = vec3f(0.431, 0.463, 0.525);
const BLUE = vec3f(0.239, 0.482, 1.0);
const SKY = vec3f(0.561, 0.702, 1.0);
const EMBER = vec3f(0.698, 0.227, 0.090);
const GLOW = vec3f(0.949, 0.604, 0.494);
const RUST = vec3f(0.886, 0.333, 0.176);

fn clip(px: vec2f) -> vec4f { return vec4f(px / u.view.xy * vec2f(2.0, -2.0) + vec2f(-1.0, 1.0), 0.0, 1.0); }

@vertex fn vs(@builtin(vertex_index) vi: u32, @builtin(instance_index) ii: u32) -> VOut {
  let q = ps[ii];
  let seed = q.seed; let phase = q.phase; let life = q.life; let time = u.a.x;
  var screen: vec2f; var col: vec3f; var a: f32; var size: f32;
  if (seed < FIELD) {
    let depth = 0.08 + 0.3 * h1(seed * 2.7);
    screen = vec2f(h1(seed * 1.1) * u.view.x, fract(h1(seed * 2.3) - u.view.w * depth / u.view.y) * u.view.y) + u.par.xy * depth * 18.0;
    let tw = 0.55 + 0.45 * sin(time * (0.6 + h1(seed * 4.4) * 1.6) + seed * 60.0);
    col = mix(STEEL, MIST, h1(seed * 3.1));
    a = (0.18 + 0.5 * h1(seed * 5.5)) * tw;
    size = 0.9 + 1.4 * pow(h1(seed * 6.6), 3.0);
  } else {
    screen = q.pos - u.view.zw;
    let p = (q.pos - u.fig.xy) / u.fig.z;
    let threat = seed < THREAT;
    size = 1.5 + 0.9 * h1(seed * 8.8);
    if (phase < 0.5) { col = mix(STEEL, MIST, h1(seed)); a = 0.42 * smoothstep(0.0, 0.8, life); }
    else if (phase < 1.5) { col = STARLIGHT; a = 0.6; }
    else if (phase < 2.5) { col = mix(STARLIGHT, BLUE, smoothstep(-30.0, MINTAKA_S, dot(UB, p))); a = 0.85; size += 0.4; }
    else if (phase < 3.5) { col = mix(BLUE, SKY, smoothstep(0.0, 90.0, p.y)); a = 0.7; }
    else if (phase < 4.5) { col = SKY; a = 0.75 * (1.0 - smoothstep(0.3, 2.6, life)); }
    else { let fl = 1.0 - smoothstep(0.05, 0.6, life); col = mix(EMBER, GLOW, fl); a = 0.9 * (1.0 - smoothstep(0.5, 1.1, life)); size += 3.5 * fl; }
    if (threat && phase < 1.5) { col = EMBER; a *= 0.7; }
    a *= u.a.z + (1.0 - u.a.z) * (1.0 - u.w.x);
    col = mix(col, mix(STARLIGHT, SKY, h1(seed * 2.2)), u.w.y);
    let along = fract(h1(seed * 5.3) + time * (0.06 + 0.1 * h1(seed * 9.1)));
    a *= mix(1.0, smoothstep(0.0, 0.06, along) * smoothstep(1.0, 0.94, along), u.w.y);
    if (u.w.z > 0.0) {
      let j = pickStar(h1(seed * 7.9));
      var tint = select(select(STARLIGHT, BLUE, j == 3), RUST, j == 0);
      let onLine = h1(seed * 4.1) < 0.12;
      if (onLine) { tint = STEEL; }
      col = mix(col, tint, u.w.z);
      a = mix(a, select(0.75, 0.3, onLine), u.w.z);
    }
  }
  a *= u.w.w;
  var corners = CORNERS;
  let c = corners[vi];
  var out: VOut;
  out.pos = clip(screen + c * size * 0.5);
  out.uv = c;
  out.col = vec4f(col * a, a);
  out.r = size;
  return out;
}

@fragment fn fs(v: VOut) -> @location(0) vec4f { return v.col * smoothstep(1.0, 0.35, length(v.uv)); }

@vertex fn starVs(@builtin(vertex_index) vi: u32) -> VOut {
  let s = u.stars[vi / 6u];
  var corners = CORNERS;
  let c = corners[vi % 6u];
  var out: VOut;
  out.pos = clip(s.xy + c * s.z * 2.2);
  out.uv = c * 2.2;
  out.col = vec4f(u.starCol[vi / 6u].xyz * s.w, s.w);
  out.r = s.z;
  return out;
}

@fragment fn starFs(v: VOut) -> @location(0) vec4f {
  let d = length(v.uv);
  let aa = 1.2 / max(v.r, 1.0);
  let disc = 1.0 - smoothstep(1.0 - aa, 1.0 + aa, d);
  let halo = 0.16 * exp(-d * d * 0.9) * (1.0 - disc);
  return v.col * (disc + halo);
}
`;

const FLOATS = (8 + 8 + 2 + 8 + 8 + 8) * 4;

export async function createWebGPU(canvas: HTMLCanvasElement, init: Float32Array, onLost: () => void): Promise<Renderer | null> {
  const gpu = (navigator as Navigator & { gpu?: GPU }).gpu;
  if (!gpu) return null;
  let device: GPUDevice;
  try {
    const adapter = await gpu.requestAdapter({ powerPreference: 'high-performance' });
    if (!adapter) return null;
    device = await adapter.requestDevice();
  } catch {
    return null;
  }
  const ctx = canvas.getContext('webgpu');
  if (!ctx) return null;
  const format = gpu.getPreferredCanvasFormat();
  ctx.configure({ device, format, alphaMode: 'premultiplied' });

  const count = init.length / STRIDE;
  const particles = device.createBuffer({ size: init.byteLength, usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST });
  device.queue.writeBuffer(particles, 0, init);
  const uniforms = device.createBuffer({ size: FLOATS * 4, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST });
  const data = new Float32Array(FLOATS);

  let compute: GPUComputePipeline, draw: GPURenderPipeline, stars: GPURenderPipeline;
  try {
    device.pushErrorScope('validation');
    const cm = device.createShaderModule({ code: COMPUTE });
    const dm = device.createShaderModule({ code: DRAW });
    compute = device.createComputePipeline({ layout: 'auto', compute: { module: cm, entryPoint: 'main' } });
    const additive: GPUBlendState = {
      color: { srcFactor: 'one', dstFactor: 'one', operation: 'add' },
      alpha: { srcFactor: 'one', dstFactor: 'one', operation: 'add' },
    };
    const over: GPUBlendState = {
      color: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' },
      alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' },
    };
    draw = device.createRenderPipeline({
      layout: 'auto',
      vertex: { module: dm, entryPoint: 'vs' },
      fragment: { module: dm, entryPoint: 'fs', targets: [{ format, blend: additive }] },
      primitive: { topology: 'triangle-list' },
    });
    stars = device.createRenderPipeline({
      layout: 'auto',
      vertex: { module: dm, entryPoint: 'starVs' },
      fragment: { module: dm, entryPoint: 'starFs', targets: [{ format, blend: over }] },
      primitive: { topology: 'triangle-list' },
    });
    const err = await device.popErrorScope();
    if (err) throw new Error(err.message);
  } catch (err) {
    console.warn('[mintaka] hero: WebGPU pipelines failed, falling back', err);
    device.destroy();
    return null;
  }

  const bind = (p: GPUComputePipeline | GPURenderPipeline, withParticles: boolean) =>
    device.createBindGroup({
      layout: p.getBindGroupLayout(0),
      entries: withParticles
        ? [{ binding: 0, resource: { buffer: uniforms } }, { binding: 1, resource: { buffer: particles } }]
        : [{ binding: 0, resource: { buffer: uniforms } }],
    });
  const computeGroup = bind(compute, true);
  const drawGroup = bind(draw, true);
  const starGroup = bind(stars, false);

  let destroyed = false;
  void device.lost.then(() => {
    if (!destroyed) onLost();
  });

  function writeFrame(fr: Frame): void {
    data.set([fr.time, fr.dt, fr.flowAlpha, count], 0);
    data.set([fr.fig.ox, fr.fig.oy, fr.fig.scale, 0], 4);
    data.set([fr.w.flow, fr.w.ray, fr.w.orion, fr.w.alpha], 8);
    data.set(fr.ray, 12);
    data.set([fr.ptr[0], fr.ptr[1], fr.ptr[2], 0], 16);
    data.set(fr.tap, 20);
    data.set([fr.view[0], fr.view[1], fr.scroll[0], fr.scroll[1]], 24);
    data.set([fr.parallax[0], fr.parallax[1], 0, 0], 28);
    for (let i = 0; i < 8; i++) {
      data.set([fr.orion[i * 3]!, fr.orion[i * 3 + 1]!, fr.orion[i * 3 + 2]!, 0], 32 + i * 4);
      data.set([fr.stars[i * 4]!, fr.stars[i * 4 + 1]!, fr.stars[i * 4 + 2]!, fr.stars[i * 4 + 3]!], 104 + i * 4);
      data.set([fr.starCol[i * 3]!, fr.starCol[i * 3 + 1]!, fr.starCol[i * 3 + 2]!, 0], 136 + i * 4);
    }
    data.set(fr.cdf, 64);
    data.set(fr.lines, 72);
    device.queue.writeBuffer(uniforms, 0, data);
  }

  const r: Renderer = {
    tier: 'A',
    sample: null,
    resize(width, height) {
      if (canvas.width !== width || canvas.height !== height) {
        canvas.width = width;
        canvas.height = height;
      }
    },
    draw(fr) {
      if (destroyed) return;
      writeFrame(fr);
      const enc = device.createCommandEncoder();
      const cp = enc.beginComputePass();
      cp.setPipeline(compute);
      cp.setBindGroup(0, computeGroup);
      cp.dispatchWorkgroups(Math.ceil(count / 256));
      cp.end();
      const rp = enc.beginRenderPass({
        colorAttachments: [{ view: ctx.getCurrentTexture().createView(), loadOp: 'clear', storeOp: 'store', clearValue: { r: 0, g: 0, b: 0, a: 0 } }],
      });
      rp.setPipeline(draw);
      rp.setBindGroup(0, drawGroup);
      rp.draw(6, count);
      rp.setPipeline(stars);
      rp.setBindGroup(0, starGroup);
      rp.draw(48);
      rp.end();
      const t0 = performance.now();
      device.queue.submit([enc.finish()]);
      const report = r.sample;
      if (report) void device.queue.onSubmittedWorkDone().then(() => report(performance.now() - t0));
    },
    clear() {
      if (destroyed) return;
      const enc = device.createCommandEncoder();
      enc.beginRenderPass({
        colorAttachments: [{ view: ctx.getCurrentTexture().createView(), loadOp: 'clear', storeOp: 'store', clearValue: { r: 0, g: 0, b: 0, a: 0 } }],
      }).end();
      device.queue.submit([enc.finish()]);
    },
    destroy() {
      destroyed = true;
      particles.destroy();
      uniforms.destroy();
      device.destroy();
    },
  };
  return r;
}
