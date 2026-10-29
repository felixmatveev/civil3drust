# Working on civil3drust

## Git workflow
- Only the owner and Claude work on this repo. Commit and push directly to
  `main`; no feature branches or pull requests needed. If work happens on a
  branch, fast-forward merge it into `main` and push without asking.
- Before every commit: `cargo fmt`, `cargo test`, `cargo clippy --all-targets`
  clean.
- Every change: a descriptive commit message, a `CHANGELOG.md` entry under
  `[Unreleased]`, and a `DECISIONS.md` entry when a design choice is made.

## Cleanroom rules
Follow the rules in `README.md` strictly. Format findings come only from our
own sample drawings, public specifications (ODA DWG spec, LandXML schema),
acadrust's source, Autodesk's public documentation (names and meanings
only), and observed Civil 3D behavior. Every finding goes in `docs/` with the
samples and steps that produced it.

## Samples
- Purpose-made drawings live in `samples/` with IDs `S<n>_<name>.dwg` and a
  row in `samples/README.md` (how made, size, SHA-256). Docs refer to them by
  ID.
- Never commit client or production drawings. A production drawing may be
  used locally as a sanity check, described only generically in docs.

## Priorities
- acadrust is 0.6.7. It reads every `AEC*`/`AECC_*` class into its
  Architecture types and re-encodes them on save; always open drawings with
  `raw::open` (it restores raw records, D-033) and keep
  `tests/acadrust_compat.rs` passing. Before bumping acadrust again, rerun
  the `fingerprint` comparison in `docs/acadrust-0.6.7.md`.
- 0.6.7 reads the `PRESSUREPARTERRORBLOCK` `3DSOLID` that 0.6.5 dropped;
  no 0.6.7-written drawing has been opened in Civil 3D yet.
- Civil 3D 2027 support is **low priority**. S123/S124 are 2027 drawings;
  use 2027 samples for information only, and keep `tests/civil3d_2027.rs`
  passing (`docs/civil3d-2027.md`).
- `hexview/` and `viewer/` are **frozen**: leave them as they are (no
  updates for library changes) until the owner says otherwise.
- LandXML export is **low priority** for now.
- Current focus: **creating alignments** in existing Civil 3D drawings
  (handle stream, style/label-set/site references, registration, adding
  and removing elements).
- Never ship Autodesk-provided styles or other template content (D-015).

## Tools
- `cargo run --release --example rawdiff -- a.dwg b.dwg [AECC_CLASS]`
- `cargo run --release --example alignments -- file.dwg`
- `cargo run --release --example set_length -- in.dwg out.dwg <name|handle> <seg> <len>`
- `cargo run --release --example set_radius -- in.dwg out.dwg <name|handle> <arc id> <radius>`
- `cargo run --release --example set_pis -- template.dwg out.dwg <name|handle> "x,y x,y ..."`
- `cargo run --release --example dumprec -- file.dwg AECC_CLASS out/prefix`
- `cargo run --release --example refs -- file.dwg [AECC_CLASS] [handle]`
- `cargo run --release --example set_style -- in.dwg [out.dwg <alignment> <style>]`
- `cargo run --release --example style_components -- in.dwg [out.dwg <style> <component> on|off]`
- `cargo run --release --example create_alignment -- donor.dwg target.dwg out.dwg <name> "x,y x,y,r ..." [--style <name|hex>] [--template <hex>] [--type <centerline|misc|n>] [--no-label-group] [--no-parcel-entry] [--no-graphics]`
- `cargo run --release --example set_name -- in.dwg out.dwg <alignment> <new name|-> [--description "text"]`
- `cargo run --release --example set_speeds -- in.dwg [out.dwg <alignment> "station:speed ..."]`
- `cargo run --release --example set_start -- in.dwg out.dwg <alignment> <station> [--clear-speeds]`
- `cargo run --release --example exp_minimal -- donor.dwg target.dwg out.dwg` (experiment S56)
- `cargo run --release --example exp_bare -- donor.dwg base.dwg plain.dwg out.dwg [style-hex]` (experiments S67, S70)
- `cargo run --release --example exp_criteria -- donor.dwg target.dwg out.dwg <check-set-hex>` (experiment S80)
- `cargo run --release --example exp_header -- in.dwg out.dwg <alignment handle> <hex value|keep> [element id]` (experiment S125–S130, MIST7)
- `cd hexview && cargo run --release -- file.dwg [--goto <hex offset|#handle>] [--compare other.dwg]` (hex view with record overlay; `--dump file.dwg` for text)
- `cd hexview && cargo run --release -- --diff a.dwg b.dwg [--show civil,opaque,unmapped,acad|all] [--filter text]` (record-by-record bit diff of two drawings)
- `cargo run --release --example fingerprint -- file.dwg [roundtrip.dwg]` (raw-record hashes, for comparing acadrust versions)
- `cargo run --release --example edit_elements -- in.dwg out.dwg <alignment> remove <id> | curve <tangent id> <radius>`
- `cargo run --release --example sites -- file.dwg` (sites and each alignment's site)
- `cargo run --release --example scan_elements -- record.bin [handle bits]` (decode a raw `AECC_ALIGNMENT` record dump)
- `python3 tools/rebuild_from_report.py report.txt a_record.bin out.bin` (rebuild B's record from A's dump and a `--diff --filter <handle>` report)
- Record map: `docs/alignment-map.md`; layout methods → constraints: `docs/element-methods.md`; open questions: `docs/mysteries.md` (MIST1…, never renumbered)
