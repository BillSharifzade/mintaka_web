// Tier B: WebGL2. Two particle buffers ping-pong through transform feedback — the GPU advances
// every particle, then draws it; the CPU only sets a few dozen uniforms per frame.

import type { Frame, Renderer } from './frame.ts';
import { DRAW_FS, DRAW_VS, STAR_FS, STAR_VS, UPDATE_FS, UPDATE_VS } from './shaders.ts';
import { STRIDE } from './particles.ts';

function compile(gl: WebGL2RenderingContext, type: number, src: string): WebGLShader {
  const sh = gl.createShader(type)!;
  gl.shaderSource(sh, src);
  gl.compileShader(sh);
  if (!gl.getShaderParameter(sh, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(sh) ?? 'shader error');
  return sh;
}

function program(gl: WebGL2RenderingContext, vs: string, fs: string, feedback?: string[]): WebGLProgram {
  const p = gl.createProgram()!;
  gl.attachShader(p, compile(gl, gl.VERTEX_SHADER, vs));
  gl.attachShader(p, compile(gl, gl.FRAGMENT_SHADER, fs));
  if (feedback) gl.transformFeedbackVaryings(p, feedback, gl.INTERLEAVED_ATTRIBS);
  gl.linkProgram(p);
  if (!gl.getProgramParameter(p, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(p) ?? 'link error');
  return p;
}

type Uniforms = Record<string, WebGLUniformLocation | null>;

function uniforms(gl: WebGL2RenderingContext, p: WebGLProgram, names: string[]): Uniforms {
  const out: Uniforms = {};
  for (const n of names) out[n] = gl.getUniformLocation(p, n);
  return out;
}

export function createWebGL(canvas: HTMLCanvasElement, init: Float32Array, onLost: () => void): Renderer | null {
  const gl = canvas.getContext('webgl2', {
    alpha: true,
    premultipliedAlpha: true,
    antialias: false,
    depth: false,
    stencil: false,
    preserveDrawingBuffer: false,
    powerPreference: 'high-performance',
  });
  if (!gl) return null;

  let upd: WebGLProgram, drw: WebGLProgram, str: WebGLProgram;
  try {
    upd = program(gl, UPDATE_VS, UPDATE_FS, ['oA', 'oB']);
    drw = program(gl, DRAW_VS, DRAW_FS);
    str = program(gl, STAR_VS, STAR_FS);
  } catch (err) {
    console.warn('[mintaka] hero: WebGL2 shaders failed, falling back', err);
    return null;
  }
  const count = init.length / STRIDE;
  const uU = uniforms(gl, upd, ['uTime', 'uDt', 'uFig', 'uW', 'uRay', 'uOrion', 'uCdf', 'uLines', 'uPtr', 'uTap']);
  const uD = uniforms(gl, drw, ['uView', 'uScroll', 'uDpr', 'uTime', 'uW', 'uFig', 'uParallax', 'uFlowAlpha', 'uOrion', 'uCdf']);
  const uS = uniforms(gl, str, ['uView', 'uStar', 'uStarCol']);

  const bufs: WebGLBuffer[] = [];
  const vaos: WebGLVertexArrayObject[] = [];
  const tfs: WebGLTransformFeedback[] = [];
  for (let i = 0; i < 2; i++) {
    const b = gl.createBuffer()!;
    gl.bindBuffer(gl.ARRAY_BUFFER, b);
    gl.bufferData(gl.ARRAY_BUFFER, init, gl.DYNAMIC_COPY);
    const v = gl.createVertexArray()!;
    gl.bindVertexArray(v);
    gl.enableVertexAttribArray(0);
    gl.vertexAttribPointer(0, 4, gl.FLOAT, false, STRIDE * 4, 0);
    gl.enableVertexAttribArray(1);
    gl.vertexAttribPointer(1, 4, gl.FLOAT, false, STRIDE * 4, 16);
    const tf = gl.createTransformFeedback()!;
    gl.bindTransformFeedback(gl.TRANSFORM_FEEDBACK, tf);
    gl.bindBufferBase(gl.TRANSFORM_FEEDBACK_BUFFER, 0, b);
    bufs.push(b);
    vaos.push(v);
    tfs.push(tf);
  }
  gl.bindTransformFeedback(gl.TRANSFORM_FEEDBACK, null);
  gl.bindVertexArray(null);
  gl.bindBuffer(gl.ARRAY_BUFFER, null);
  const empty = gl.createVertexArray()!;
  let cur = 0;
  let lost = false;
  let destroyed = false;

  canvas.addEventListener(
    'webglcontextlost',
    (e) => {
      e.preventDefault();
      lost = true;
      if (!destroyed) onLost();
    },
    { once: true },
  );

  gl.enable(gl.BLEND);

  const r: Renderer = {
    tier: 'B',
    sample: null,
    resize(width, height) {
      if (canvas.width !== width || canvas.height !== height) {
        canvas.width = width;
        canvas.height = height;
      }
    },
    draw(f: Frame) {
      if (lost) return;
      const t0 = r.sample ? performance.now() : 0;
      // 1 · advance every particle on the GPU
      gl.useProgram(upd);
      gl.uniform1f(uU['uTime']!, f.time);
      gl.uniform1f(uU['uDt']!, f.dt);
      gl.uniform3f(uU['uFig']!, f.fig.ox, f.fig.oy, f.fig.scale);
      gl.uniform4f(uU['uW']!, f.w.flow, f.w.ray, f.w.orion, f.w.alpha);
      gl.uniform4f(uU['uRay']!, ...f.ray);
      gl.uniform3fv(uU['uOrion']!, f.orion);
      gl.uniform1fv(uU['uCdf']!, f.cdf);
      gl.uniform4fv(uU['uLines']!, f.lines);
      gl.uniform3f(uU['uPtr']!, ...f.ptr);
      gl.uniform4f(uU['uTap']!, ...f.tap);
      gl.bindVertexArray(vaos[cur]!);
      gl.bindTransformFeedback(gl.TRANSFORM_FEEDBACK, tfs[1 - cur]!);
      gl.enable(gl.RASTERIZER_DISCARD);
      gl.beginTransformFeedback(gl.POINTS);
      gl.drawArrays(gl.POINTS, 0, count);
      gl.endTransformFeedback();
      gl.disable(gl.RASTERIZER_DISCARD);
      gl.bindTransformFeedback(gl.TRANSFORM_FEEDBACK, null);
      cur = 1 - cur;

      // 2 · draw them, additively: dense regions glow like the light they stand for
      gl.viewport(0, 0, canvas.width, canvas.height);
      gl.clearColor(0, 0, 0, 0);
      gl.clear(gl.COLOR_BUFFER_BIT);
      gl.useProgram(drw);
      gl.uniform2f(uD['uView']!, ...f.view);
      gl.uniform2f(uD['uScroll']!, ...f.scroll);
      gl.uniform1f(uD['uDpr']!, f.dpr);
      gl.uniform1f(uD['uTime']!, f.time);
      gl.uniform4f(uD['uW']!, f.w.flow, f.w.ray, f.w.orion, f.w.alpha);
      gl.uniform3f(uD['uFig']!, f.fig.ox, f.fig.oy, f.fig.scale);
      gl.uniform2f(uD['uParallax']!, ...f.parallax);
      gl.uniform1f(uD['uFlowAlpha']!, f.flowAlpha);
      gl.uniform3fv(uD['uOrion']!, f.orion);
      gl.uniform1fv(uD['uCdf']!, f.cdf);
      gl.bindVertexArray(vaos[cur]!);
      gl.blendFunc(gl.ONE, gl.ONE);
      gl.drawArrays(gl.POINTS, 0, count);

      // 3 · the named stars on top, opaque discs with a soft halo
      gl.useProgram(str);
      gl.uniform2f(uS['uView']!, ...f.view);
      gl.uniform4fv(uS['uStar']!, f.stars);
      gl.uniform3fv(uS['uStarCol']!, f.starCol);
      gl.bindVertexArray(empty);
      gl.blendFunc(gl.ONE, gl.ONE_MINUS_SRC_ALPHA);
      gl.drawArrays(gl.TRIANGLES, 0, 48);
      gl.bindVertexArray(null);
      if (r.sample) {
        gl.finish(); // only while sampling: waits for the GPU so the time is real
        r.sample(performance.now() - t0);
      }
    },
    clear() {
      if (lost) return;
      gl.clearColor(0, 0, 0, 0);
      gl.clear(gl.COLOR_BUFFER_BIT);
    },
    destroy() {
      destroyed = true;
      for (const b of bufs) gl.deleteBuffer(b);
      for (const v of vaos) gl.deleteVertexArray(v);
      for (const t of tfs) gl.deleteTransformFeedback(t);
      gl.deleteVertexArray(empty);
      gl.deleteProgram(upd);
      gl.deleteProgram(drw);
      gl.deleteProgram(str);
      gl.getExtension('WEBGL_lose_context')?.loseContext();
    },
  };
  return r;
}
