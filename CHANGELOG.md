# Changelog

## 0.1.0 (unreleased)

First version.

- Engine: cellular automaton with powders, liquids (with density: sand sinks, oil floats), gases and solids.
- 15 materials: sand, water, fire, smoke, steam, wood, embers, oil, lava, acid, ice, plant, glass, stone, wall.
- Reactions: fire spreads through fuel and is put out by water, wood burns in place as embers, lava turns water
  to steam and stone and sand to glass, acid dissolves almost anything, ice freezes water, plants grow into it.
- WebAssembly module with no imports and no bindings generator; pixels are read straight from wasm memory.
- Web app: palette, brush sizes, pause, clear, keyboard shortcuts, touch support, installable (PWA), offline.
- Share links: run-length encoding in Rust plus deflate in the browser.
