// Browser glue: input, drawing and share links. All physics runs in Rust (sabbia.wasm).

const MATERIALS = [
  { id: 2, name: "Sand", color: "#e2bf6e" },
  { id: 3, name: "Water", color: "#2f7de1" },
  { id: 4, name: "Fire", color: "#ff8a1c" },
  { id: 7, name: "Wood", color: "#7a4e2a" },
  { id: 8, name: "Oil", color: "#4a3a2a" },
  { id: 9, name: "Lava", color: "#f04a14" },
  { id: 10, name: "Acid", color: "#7ce83a" },
  { id: 11, name: "Ice", color: "#a8e4f5" },
  { id: 12, name: "Plant", color: "#2fa34a" },
  { id: 14, name: "Stone", color: "#8a8680" },
  { id: 1, name: "Wall", color: "#6b6f7a" },
  { id: 0, name: "Eraser", color: "#10121a" },
];
const STEPS_PER_SECOND = 60;

const canvas = document.getElementById("world");
const ctx = canvas.getContext("2d");
const hint = document.getElementById("hint");
const stats = document.getElementById("stats");
const toast = document.getElementById("toast");

const { instance } = await loadWasm("sabbia.wasm");
const wasm = instance.exports;

let material = MATERIALS[0];
let brush = 5;
let paused = false;
let image = null;

async function loadWasm(url) {
  if (WebAssembly.instantiateStreaming) {
    try {
      return await WebAssembly.instantiateStreaming(fetch(url), {});
    } catch {
      // Some servers send the wrong MIME type: fall back to bytes.
    }
  }
  const bytes = await (await fetch(url)).arrayBuffer();
  return WebAssembly.instantiate(bytes, {});
}

// ---------- world size ----------

function gridForScreen() {
  const stage = document.getElementById("stage").getBoundingClientRect();
  const cellPx = window.innerWidth < 700 ? 3 : 4;
  const width = Math.max(120, Math.min(480, Math.floor(stage.width / cellPx)));
  const height = Math.max(90, Math.min(320, Math.floor(stage.height / cellPx)));
  return { width, height };
}

function resizeCanvas() {
  const w = wasm.width();
  const h = wasm.height();
  canvas.width = w;
  canvas.height = h;
  // Largest integer-ish scale that fits, keeping pixels square.
  const stage = document.getElementById("stage").getBoundingClientRect();
  const scale = Math.max(1, Math.min((stage.width - 16) / w, (stage.height - 16) / h));
  canvas.style.width = `${Math.floor(w * scale)}px`;
  canvas.style.height = `${Math.floor(h * scale)}px`;
  image = null;
}

// ---------- drawing ----------

function draw() {
  const w = wasm.width();
  const h = wasm.height();
  // The pixels live in WebAssembly memory; the view must be rebuilt if memory grew.
  if (!image || image.data.buffer !== wasm.memory.buffer || image.data.byteOffset !== wasm.pixels()) {
    image = new ImageData(new Uint8ClampedArray(wasm.memory.buffer, wasm.pixels(), w * h * 4), w, h);
  }
  ctx.putImageData(image, 0, 0);
}

let last = performance.now();
let backlog = 0;
let frames = 0;
let fpsClock = last;
let stepTime = 0;

function frame(now) {
  if (!paused) {
    // Fixed timestep: the same speed on 60 Hz and 120 Hz screens, at most 3 steps per frame.
    backlog = Math.min(backlog + (now - last) / 1000, 3 / STEPS_PER_SECOND);
    let steps = 0;
    while (backlog >= 1 / STEPS_PER_SECOND) {
      backlog -= 1 / STEPS_PER_SECOND;
      steps++;
    }
    if (steps > 0) {
      const t0 = performance.now();
      wasm.step(steps);
      stepTime = (performance.now() - t0) / steps;
    }
  }
  last = now;
  paintStroke();
  draw();
  frames++;
  if (now - fpsClock > 500) {
    const fps = Math.round((frames * 1000) / (now - fpsClock));
    stats.textContent = `${wasm.width()}×${wasm.height()} · ${fps} fps · physics ${stepTime.toFixed(2)} ms`;
    frames = 0;
    fpsClock = now;
  }
  requestAnimationFrame(frame);
}

// ---------- input ----------

let pointer = null; // { x, y, lastX, lastY } in cells while drawing

function toCell(event) {
  const rect = canvas.getBoundingClientRect();
  return {
    x: Math.floor(((event.clientX - rect.left) / rect.width) * wasm.width()),
    y: Math.floor(((event.clientY - rect.top) / rect.height) * wasm.height()),
  };
}

canvas.addEventListener("pointerdown", (event) => {
  canvas.setPointerCapture(event.pointerId);
  const cell = toCell(event);
  pointer = { ...cell, lastX: cell.x, lastY: cell.y };
  hint.classList.add("gone");
});
canvas.addEventListener("pointermove", (event) => {
  if (pointer) Object.assign(pointer, toCell(event));
});
for (const type of ["pointerup", "pointercancel"]) {
  canvas.addEventListener(type, () => (pointer = null));
}

// Paints along the segment since the last frame, so fast strokes leave no gaps.
// Holding still keeps pouring, like a tap.
function paintStroke() {
  if (!pointer) return;
  const dx = pointer.x - pointer.lastX;
  const dy = pointer.y - pointer.lastY;
  const steps = Math.max(1, Math.ceil(Math.hypot(dx, dy) / Math.max(1, brush / 2)));
  for (let i = 1; i <= steps; i++) {
    const x = Math.round(pointer.lastX + (dx * i) / steps);
    const y = Math.round(pointer.lastY + (dy * i) / steps);
    wasm.paint(x, y, brush, material.id);
  }
  pointer.lastX = pointer.x;
  pointer.lastY = pointer.y;
}

// ---------- toolbar ----------

const palette = document.getElementById("palette");
MATERIALS.forEach((m, index) => {
  const button = document.createElement("button");
  button.className = "material";
  button.setAttribute("role", "radio");
  button.innerHTML = `<span class="swatch" style="background:${m.color}"></span>${m.name}` +
    (index < 10 ? ` <kbd>${(index + 1) % 10}</kbd>` : "");
  button.addEventListener("click", () => select(m));
  m.button = button;
  palette.append(button);
});

function select(m) {
  material = m;
  for (const other of MATERIALS) {
    other.button.classList.toggle("active", other === m);
    other.button.setAttribute("aria-checked", String(other === m));
  }
}

function setBrush(size) {
  brush = size;
  for (const b of document.querySelectorAll(".size")) b.classList.toggle("active", Number(b.dataset.size) === size);
}
for (const b of document.querySelectorAll(".size")) b.addEventListener("click", () => setBrush(Number(b.dataset.size)));

const pauseButton = document.getElementById("pause");
function togglePause() {
  paused = !paused;
  pauseButton.textContent = paused ? "▶" : "❚❚";
  pauseButton.classList.toggle("active", paused);
}
pauseButton.addEventListener("click", togglePause);
document.getElementById("clear").addEventListener("click", () => wasm.clear());
document.getElementById("share").addEventListener("click", share);

window.addEventListener("keydown", (event) => {
  if (event.target instanceof HTMLInputElement || event.metaKey || event.ctrlKey) return;
  const digit = Number(event.key);
  if (event.key >= "0" && event.key <= "9") select(MATERIALS[(digit + 9) % 10]);
  else if (event.key === "e") select(MATERIALS.find((m) => m.id === 0));
  else if (event.key === "w") select(MATERIALS.find((m) => m.id === 1));
  else if (event.key === " ") { event.preventDefault(); togglePause(); }
  else if (event.key === "c") wasm.clear();
  else if (event.key === "[") setBrush(brush === 10 ? 5 : 2);
  else if (event.key === "]") setBrush(brush === 2 ? 5 : 10);
});

// ---------- share links ----------

function toBase64Url(bytes) {
  let binary = "";
  for (const b of bytes) binary += String.fromCharCode(b);
  return btoa(binary).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

function fromBase64Url(text) {
  const binary = atob(text.replace(/-/g, "+").replace(/_/g, "/"));
  return Uint8Array.from(binary, (c) => c.charCodeAt(0));
}

// Scenes are encoded by Rust (run-length), then deflated by the browser when it can: links shrink a lot more.
async function deflate(bytes) {
  const stream = new Blob([bytes]).stream().pipeThrough(new CompressionStream("deflate-raw"));
  return new Uint8Array(await new Response(stream).arrayBuffer());
}

async function inflate(bytes) {
  const stream = new Blob([bytes]).stream().pipeThrough(new DecompressionStream("deflate-raw"));
  return new Uint8Array(await new Response(stream).arrayBuffer());
}

async function share() {
  const length = wasm.export_scene();
  const bytes = new Uint8Array(wasm.memory.buffer, wasm.buffer(), length).slice();
  let hash = `s=${toBase64Url(bytes)}`;
  if ("CompressionStream" in window) {
    hash = `z=${toBase64Url(await deflate(bytes))}`;
  }
  const url = `${location.origin}${location.pathname}#${hash}`;
  history.replaceState(null, "", url);
  try {
    await navigator.clipboard.writeText(url);
    notify(`Link copied (${url.length} characters)`);
  } catch {
    notify("Link is in the address bar");
  }
}

async function loadFromHash() {
  const match = location.hash.match(/^#([sz])=([A-Za-z0-9_-]+)$/);
  if (!match) return false;
  try {
    let bytes = fromBase64Url(match[2]);
    if (match[1] === "z") bytes = await inflate(bytes);
    new Uint8Array(wasm.memory.buffer, wasm.reserve(bytes.length), bytes.length).set(bytes);
    return wasm.import_scene(Date.now() >>> 0) === 1;
  } catch {
    return false;
  }
}

let toastTimer;
function notify(text) {
  toast.textContent = text;
  toast.classList.add("show");
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => toast.classList.remove("show"), 2200);
}

// ---------- start ----------

function demoScene() {
  const w = wasm.width();
  const h = wasm.height();
  const line = (x0, x1, y, id) => { for (let x = x0; x <= x1; x += 2) wasm.paint(x, y, 1, id); };
  line(Math.round(w * 0.08), Math.round(w * 0.38), Math.round(h * 0.55), 7); // wooden shelf
  line(Math.round(w * 0.55), Math.round(w * 0.92), Math.round(h * 0.4), 1); // stone ledge
  wasm.paint(Math.round(w * 0.75), h - 6, Math.round(Math.min(w, h) * 0.12), 3); // a pond
  wasm.paint(Math.round(w * 0.2), Math.round(h * 0.5), 3, 4); // a fire on the shelf
}

wasm.init(...Object.values(gridForScreen()), Date.now() >>> 0);
if (await loadFromHash()) {
  notify("Shared scene loaded");
} else {
  demoScene();
}
resizeCanvas();
select(material);
window.addEventListener("resize", resizeCanvas);
requestAnimationFrame(frame);

if ("serviceWorker" in navigator && location.protocol === "https:") {
  navigator.serviceWorker.register("sw.js");
}
