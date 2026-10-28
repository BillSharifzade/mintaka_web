// GLSL ES 3.00 for tier B. The simulation runs entirely on the GPU with transform feedback:
// one vertex shader advances every particle per frame; a second draws them as soft points.
// Geometry constants are injected from geometry.ts, so shader and tests share one source.

import { EDGES, FIELD_SHARE, GROW, HALF_CHANNEL, LANES, LOGO, MINTAKA_S, NB, THREAT_SHARE, U, type Edge, type V2 } from './geometry.ts';

const f = (n: number) => (Number.isInteger(n) ? `${n}.0` : `${+n.toFixed(6)}`);
const e3 = (e: Edge) => `vec3(${f(e[0])}, ${f(e[1])}, ${f(e[2])})`;
/** A quad corner pulled 18 % towards the bulb's centre (see fillTarget in geometry.ts). */
const corner = (p: V2, c: V2) => `vec2(${f(c[0] + (p[0] - c[0]) * 0.82)}, ${f(c[1] + (p[1] - c[1]) * 0.82)})`;
const T: V2 = [-2, -48.5];
const B: V2 = [2, 48.5];


const COMMON = /* glsl */ `
const vec2 U = vec2(${f(U[0])}, ${f(U[1])});
const vec2 NB = vec2(${f(NB[0])}, ${f(NB[1])});
const vec3 E_TOP = ${e3(EDGES.top)};
const vec3 E_RT = ${e3(EDGES.rightT)};
const vec3 E_LT = ${e3(EDGES.leftT)};
const vec3 E_LB = ${e3(EDGES.leftB)};
const vec3 E_RB = ${e3(EDGES.rightB)};
const vec3 E_BOT = ${e3(EDGES.bottom)};
const float GROW = ${f(GROW)};
const float HALF = ${f(HALF_CHANNEL)};
const float MINTAKA_S = ${f(MINTAKA_S)};
const float FIELD = ${f(FIELD_SHARE)};
const float THREAT = ${f(FIELD_SHARE + THREAT_SHARE)};

float h1(float n) { return fract(sin(n * 91.3458 + 17.17) * 47453.5453); }

// Even fill targets inside each bulb (bilinear over the shrunk quad).
vec2 fillT(float u, float v) {
  vec2 a = ${corner(LOGO['betelgeuse']!, T)}, b = ${corner(LOGO['bellatrix']!, T)}, c = ${corner(LOGO['mintaka']!, T)}, d = ${corner(LOGO['alnitak']!, T)};
  return mix(mix(a, b, u), mix(d, c, u), v * 0.78);
}
vec2 fillB(float u, float v) {
  vec2 a = ${corner(LOGO['alnitak']!, B)}, b = ${corner(LOGO['mintaka']!, B)}, c = ${corner(LOGO['rigel']!, B)}, d = ${corner(LOGO['saiph']!, B)};
  return mix(mix(a, b, u), mix(d, c, u), 0.22 + v * 0.74);
}
float ed(vec3 e, vec2 p) { return dot(e.xy, p) - e.z; }
`;

export const UPDATE_VS = /* glsl */ `#version 300 es
precision highp float;
layout(location = 0) in vec4 aA; // position.xy (document px), velocity.xy (px/s)
layout(location = 1) in vec4 aB; // seed, phase, time in phase, spare
out vec4 oA;
out vec4 oB;

uniform float uTime;
uniform float uDt;
uniform vec3 uFig;      // logo origin in document px, px per logo unit
uniform vec4 uW;        // scene weights: flow, ray, orion, global alpha
uniform vec4 uRay;      // light ray: x, y, width, height (document px)
uniform vec3 uOrion[8]; // stars of Orion in S3: x, y, radius (document px)
uniform float uCdf[8];  // cumulative share of particles per star
uniform vec4 uLines[8]; // constellation lines (document px)
uniform vec3 uPtr;      // pointer: x, y (document px), strength 0–1
uniform vec4 uTap;      // tap ripple: x, y, start time, active
${COMMON}

float sdTop(vec2 p) {
  return max(max(ed(E_TOP, p), ed(E_RT, p)), max(ed(E_LT, p), dot(NB, p))) - GROW;
}
float sdBot(vec2 p, bool open) {
  float s = max(max(ed(E_LB, p), ed(E_RB, p)), -dot(NB, p));
  return (open ? s : max(s, ed(E_BOT, p))) - GROW;
}
float sdV(vec2 p, bool open) { return min(sdTop(p), sdBot(p, open)); }
vec2 gradV(vec2 p, bool open) {
  const float e = 0.6;
  return vec2(sdV(p + vec2(e, 0.0), open) - sdV(p - vec2(e, 0.0), open),
              sdV(p + vec2(0.0, e), open) - sdV(p - vec2(0.0, e), open)) / (2.0 * e);
}
float bottomY(float x) { return (E_BOT.z + GROW - E_BOT.x * x) / E_BOT.y; }

// A divergence-free swirl: the curl of a sum of sines. Requests jostle; nothing piles up.
vec2 curl(vec2 p, float t) {
  float a = 0.045, b = 0.061, c = 0.033;
  float dpx = a * cos(p.x * a + t * 0.7) * cos(p.y * b - t * 0.5) + 0.6 * c * cos(p.x * c - p.y * a + t * 0.9);
  float dpy = -b * sin(p.x * a + t * 0.7) * sin(p.y * b - t * 0.5) - 0.6 * a * cos(p.x * c - p.y * a + t * 0.9);
  return vec2(dpy, -dpx) * 22.0;
}

vec2 spawn(float seed, float t) {
  float r1 = h1(seed * 13.1 + floor(t * 0.37)), r2 = h1(seed * 7.7 + floor(t * 0.53));
  float ang = mix(-2.6, -0.55, r1);
  float rad = mix(72.0, 118.0, r2);
  return vec2(-4.0, -62.0) + vec2(cos(ang), sin(ang)) * rad;
}

// Acceleration in logo units for the hourglass scene. Phase: 0 inflow, 1 top bulb, 2 belt,
// 3 bottom bulb, 4 output stream, 5 stopped threat.
vec2 flow(inout vec2 p, inout vec2 v, inout float phase, inout float life, float seed, bool threat, int k) {
  vec2 a = vec2(0.0);
  float lane = ${LANES.map((x, i) => `(k == ${i} ? ${f(x)} : `).join('')}0.0${LANES.map(() => ')').join('')};
  if (phase < 0.5) {
    // Each request heads for its own place in the top bulb, so the bulb fills evenly.
    vec2 tgt = fillT(h1(seed * 1.7), h1(seed * 2.9));
    a = (tgt - p) * 2.6 - v * 2.2 + curl(p, uTime) * 1.2;
    if (sdTop(p) < 0.0 && dot(NB, p) < -HALF && length(tgt - p) < 9.0) { phase = 1.0; life = 0.0; v *= 0.5; }
  } else if (phase < 1.5) {
    a = vec2(0.0, 13.0) + curl(p, uTime) * 2.2 - v * 2.1;
    if (dot(NB, p) > -HALF) {
      if (threat) { phase = 5.0; life = 0.0; v *= 0.1; }
      else { phase = 2.0; life = 0.0; }
    }
  } else if (phase < 2.5) {
    float d = dot(NB, p);
    a = (U * 112.0 - v) * 7.0 - NB * d * 30.0;
    // Past Mintaka the belt releases them into the lower bulb.
    if (dot(U, p) > MINTAKA_S + 2.0) { phase = 3.0; life = 0.0; }
  } else if (phase < 3.5) {
    if (life < 1.5) {
      // Fan out from Mintaka to a place of its own in the bulb…
      a = (fillB(h1(seed * 3.1), h1(seed * 4.3)) - p) * 3.0 - v * 2.4;
    } else {
      // …then settle into one of the four ordered lanes.
      a = vec2(0.0, 15.0) + curl(p, uTime * 0.6) * 0.8 - v * vec2(1.2, 1.6);
      a.x += (lane - p.x) * 4.0 * smoothstep(70.0, 104.0, p.y);
    }
    if (p.y > bottomY(p.x) - 1.5) { phase = 4.0; life = 0.0; }
  } else if (phase < 4.5) {
    a = vec2((lane - p.x) * 9.0 - v.x * 5.0, 62.0 - v.y * 0.4);
    if (p.y > bottomY(lane) + 120.0 || life > 3.6) { phase = 0.0; life = 0.0; p = spawn(seed, uTime); v = vec2(0.0); }
  } else {
    v *= 0.82;
    if (life > 1.1) { phase = 0.0; life = 0.0; p = spawn(seed, uTime); v = vec2(0.0); }
  }
  // Walls: everywhere but the inflow, the stream and the open bottom of the lower bulb.
  if (phase > 0.5 && phase < 3.5) {
    bool open = phase > 2.5;
    float s = sdV(p, open);
    if (s > -2.0) a -= normalize(gradV(p, open) + 1e-5) * (s + 2.0) * 160.0;
  }
  return a;
}

void main() {
  vec2 P = aA.xy, V = aA.zw;
  float seed = aB.x, phase = aB.y, life = aB.z + uDt;
  if (seed < FIELD) { oA = aA; oB = aB; return; }
  bool threat = seed < THREAT;
  int k = int(floor(h1(seed * 3.7) * 4.0));

  vec2 acc = vec2(0.0);
  float sc = uFig.z;
  if (uW.x > 0.001) {
    vec2 p = (P - uFig.xy) / sc, v = V / sc;
    vec2 a = flow(p, v, phase, life, seed, threat, k);
    // flow() may respawn or stop the particle: write its position back.
    P = uFig.xy + p * sc;
    V = v * sc;
    acc += a * sc * uW.x;
  }

  float wt = uW.y + uW.z;
  if (wt > 0.001) {
    vec2 tgt = vec2(0.0);
    if (uW.y > 0.0) {
      float along = fract(h1(seed * 5.3) + uTime * (0.06 + 0.1 * h1(seed * 9.1)));
      float lens = 1.0 - abs(2.0 * along - 1.0);
      vec2 rt = vec2(uRay.x + along * uRay.z, uRay.y + (h1(seed * 2.9) - 0.5) * 14.0 * lens);
      // The beam wraps from its right end to its left end; jump with it (faded out there).
      if (along < 0.04 && P.x > uRay.x + uRay.z * 0.75 && uW.y > 0.5) { P = rt; V = vec2(uRay.z * 0.1, 0.0); }
      tgt += rt * uW.y;
    }
    if (uW.z > 0.0) {
      vec2 ot;
      float pick = h1(seed * 4.1);
      if (pick < 0.12) {
        vec4 l = uLines[int(floor(h1(seed * 6.3) * 8.0))];
        ot = mix(l.xy, l.zw, h1(seed * 8.7)) + (vec2(h1(seed * 1.3), h1(seed * 2.1)) - 0.5) * 2.0;
      } else {
        float r = h1(seed * 7.9);
        int j = 7;
        for (int i = 0; i < 8; i++) { if (r <= uCdf[i]) { j = i; break; } }
        vec3 st = uOrion[j];
        float ang = h1(seed * 3.3) * 6.2832 + uTime * (0.12 + 0.2 * h1(seed * 5.9));
        float rad = st.z * (0.35 + 2.6 * pow(h1(seed * 4.7), 2.0));
        ot = st.xy + vec2(cos(ang), sin(ang)) * rad;
      }
      tgt += ot * uW.z;
    }
    tgt /= wt;
    acc += ((tgt - P) * 26.0 - V * 7.0) * min(wt, 1.0);
  }

  // The visitor's hand: a soft gravity well; a tap sends a ripple.
  if (uPtr.z > 0.0) {
    vec2 d = P - uPtr.xy;
    float r = length(d) + 1e-3;
    if (r < 170.0) acc -= d / r * uPtr.z * 1100.0 * pow(1.0 - r / 170.0, 2.0);
  }
  if (uTap.w > 0.5) {
    float age = uTime - uTap.z;
    vec2 d = P - uTap.xy;
    float r = length(d) + 1e-3;
    float band = exp(-pow((r - age * 720.0) / 42.0, 2.0));
    acc += d / r * band * 3200.0 * max(0.0, 1.0 - age / 1.3);
  }

  V += acc * uDt;
  P += V * uDt;
  oA = vec4(P, V);
  oB = vec4(seed, phase, life, 0.0);
}
`;

export const UPDATE_FS = /* glsl */ `#version 300 es
precision mediump float;
out vec4 o;
void main() { o = vec4(0.0); }
`;

export const DRAW_VS = /* glsl */ `#version 300 es
precision highp float;
layout(location = 0) in vec4 aA;
layout(location = 1) in vec4 aB;
uniform vec2 uView;     // viewport, CSS px
uniform vec2 uScroll;   // scroll offset, CSS px
uniform float uDpr;
uniform float uTime;
uniform vec4 uW;
uniform vec3 uFig;
uniform vec2 uParallax; // pointer offset from the centre, −1…1
uniform float uFlowAlpha;
uniform vec3 uOrion[8];
uniform float uCdf[8];
out vec4 vCol;
${COMMON}

const vec3 STARLIGHT = vec3(0.957, 0.961, 0.969);
const vec3 MIST = vec3(0.788, 0.808, 0.847);
const vec3 STEEL = vec3(0.431, 0.463, 0.525);
const vec3 BLUE = vec3(0.239, 0.482, 1.0);
const vec3 SKY = vec3(0.561, 0.702, 1.0);
const vec3 EMBER = vec3(0.698, 0.227, 0.090);
const vec3 GLOW = vec3(0.949, 0.604, 0.494);
const vec3 RUST = vec3(0.886, 0.333, 0.176);

void main() {
  float seed = aB.x, phase = aB.y, life = aB.z;
  vec2 screen;
  vec3 col;
  float a, size;
  if (seed < FIELD) {
    // The background sky: fixed stars with depth, drifting with scroll and the pointer.
    float depth = 0.08 + 0.3 * h1(seed * 2.7);
    screen = vec2(h1(seed * 1.1) * uView.x, fract(h1(seed * 2.3) - uScroll.y * depth / uView.y) * uView.y);
    screen += uParallax * depth * 18.0;
    float tw = 0.55 + 0.45 * sin(uTime * (0.6 + h1(seed * 4.4) * 1.6) + seed * 60.0);
    col = mix(STEEL, MIST, h1(seed * 3.1));
    a = (0.18 + 0.5 * h1(seed * 5.5)) * tw;
    size = 0.9 + 1.4 * pow(h1(seed * 6.6), 3.0);
  } else {
    screen = aA.xy - uScroll;
    vec2 p = (aA.xy - uFig.xy) / uFig.z;
    bool threat = seed < THREAT;
    size = 1.5 + 0.9 * h1(seed * 8.8);
    if (phase < 0.5) { col = mix(STEEL, MIST, h1(seed)); a = 0.42 * smoothstep(0.0, 0.8, life); }
    else if (phase < 1.5) { col = STARLIGHT; a = 0.6; }
    else if (phase < 2.5) { col = mix(STARLIGHT, BLUE, smoothstep(-30.0, MINTAKA_S, dot(U, p))); a = 0.85; size += 0.4; }
    else if (phase < 3.5) { col = mix(BLUE, SKY, smoothstep(0.0, 90.0, p.y)); a = 0.7; }
    else if (phase < 4.5) { col = SKY; a = 0.75 * (1.0 - smoothstep(0.3, 2.6, life)); }
    else { float fl = 1.0 - smoothstep(0.05, 0.6, life); col = mix(EMBER, GLOW, fl); a = 0.9 * (1.0 - smoothstep(0.5, 1.1, life)); size += 3.5 * fl; }
    if (threat && phase < 1.5) { col = EMBER; a *= 0.7; }
    a *= uFlowAlpha + (1.0 - uFlowAlpha) * (1.0 - uW.x);
    // Ray and Orion take over the colour as the scenes change.
    col = mix(col, mix(STARLIGHT, SKY, h1(seed * 2.2)), uW.y);
    float along = fract(h1(seed * 5.3) + uTime * (0.06 + 0.1 * h1(seed * 9.1)));
    a *= mix(1.0, smoothstep(0.0, 0.06, along) * smoothstep(1.0, 0.94, along), uW.y);
    if (uW.z > 0.0) {
      float r = h1(seed * 7.9);
      int j = 7;
      for (int i = 0; i < 8; i++) { if (r <= uCdf[i]) { j = i; break; } }
      vec3 tint = j == 0 ? RUST : (j == 3 ? BLUE : STARLIGHT);
      bool onLine = h1(seed * 4.1) < 0.12;
      if (onLine) tint = STEEL;
      col = mix(col, tint, uW.z);
      a = mix(a, onLine ? 0.3 : 0.75, uW.z);
    }
  }
  a *= uW.w;
  gl_Position = vec4(screen / uView * vec2(2.0, -2.0) + vec2(-1.0, 1.0), 0.0, 1.0);
  gl_PointSize = size * uDpr;
  vCol = vec4(col * a, a);
}
`;

export const DRAW_FS = /* glsl */ `#version 300 es
precision mediump float;
in vec4 vCol;
out vec4 o;
void main() {
  float d = length(gl_PointCoord * 2.0 - 1.0);
  o = vCol * smoothstep(1.0, 0.35, d);
}
`;

/** The named stars, drawn as glowing discs over the particles. */
export const STAR_VS = /* glsl */ `#version 300 es
precision highp float;
uniform vec2 uView;
uniform vec4 uStar[8];    // screen x, y, radius (CSS px), alpha
uniform vec3 uStarCol[8];
out vec4 vCol;
out vec2 vUv;
out float vR;
// Six vertices per star (two triangles), so size is never capped by the point-size limit.
const vec2 CORNERS[6] = vec2[6](vec2(-1, -1), vec2(1, -1), vec2(1, 1), vec2(-1, -1), vec2(1, 1), vec2(-1, 1));
void main() {
  int i = gl_VertexID / 6;
  vec4 s = uStar[i];
  vec2 c = CORNERS[gl_VertexID % 6];
  vUv = c;
  vR = s.z;
  vec2 pos = s.xy + c * s.z * 2.2;
  gl_Position = vec4(pos / uView * vec2(2.0, -2.0) + vec2(-1.0, 1.0), 0.0, 1.0);
  vCol = vec4(uStarCol[i] * s.w, s.w);
}
`;

export const STAR_FS = /* glsl */ `#version 300 es
precision mediump float;
in vec4 vCol;
in vec2 vUv;
in float vR;
out vec4 o;
void main() {
  // Disc of radius 1/2.2 of the quad, then a soft halo out to its edge.
  float d = length(vUv) * 2.2;
  float aa = 1.2 / max(vR, 1.0);
  float disc = 1.0 - smoothstep(1.0 - aa, 1.0 + aa, d);
  float halo = 0.16 * exp(-d * d * 0.9) * (1.0 - disc);
  o = vCol * (disc + halo);
}
`;
