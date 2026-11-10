# Archive

Tools that were replaced and are no longer maintained (D-075). They are
kept for reference; they are not built or tested and may not compile
against the current library.

| Folder | Replaced by |
|---|---|
| [`hexview/`](hexview/README.md) | `workbench/`: File (hex) tab (sections, record overlay, detail windows, Compare with…), hex pane, `--dump`, `--diff` |
| [`viewer/`](viewer/README.md) | `workbench/`: Drawing tab, `--objects` (was `--dump`), `--svg` |

The workbench's `filemap.rs`, `compare.rs` and `fileview.rs` started as
hexview's `model.rs`, `compare.rs` and `app.rs`; its `scene.rs`, `view.rs`
and `svg.rs` as the viewer's.
