# civil3drust-viewer

A 2D/3D look at the Civil 3D objects civil3drust can read from a DWG. A visual
check and demonstration, nothing more.

```
cargo run --release -- ../samples/S11_100-200-300_r50.dwg   # window
cargo run --release -- --dump file.dwg                       # list objects
cargo run --release -- --svg out.svg [--3d] file.dwg         # headless render
```

Layers (toolbar toggles):

- **Decoded** — geometry civil3drust decoded itself (alignments: path, PIs,
  station ticks, direction arrow).
- **Proxy graphics** — the display cache Civil 3D saved in each object. Overlay
  it on *Decoded* to check the decoder. In 3D, *Proxy lift* raises it so the
  two separate.
- **Placeholders** — Civil 3D's "not drawn yet" images (off by default).
- **AutoCAD entities** — plain lines, polylines, arcs, circles, in grey.

Navigation: wheel zooms, left-drag pans (2D) or orbits (3D), right/middle drag
pans, double-click fits. Click an object to see its details. DWGs can be
dropped on the window.

Samples S2–S4 carry real proxy graphics; the others only placeholders.
