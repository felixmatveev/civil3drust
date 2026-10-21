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
- The acadrust 0.6.5 `3DSOLID` parsing problem (solids in
  `PRESSUREPARTERRORBLOCK` dropped on save) is **extremely low priority**.
  Don't work on it unless the owner asks; just keep warning about it.

## Tools
- `cargo run --release --example rawdiff -- a.dwg b.dwg [AECC_CLASS]`
- `cargo run --release --example alignments -- file.dwg`
- `cargo run --release --example set_length -- in.dwg out.dwg <name|handle> <seg> <len>`
- `cargo run --release --example set_radius -- in.dwg out.dwg <name|handle> <arc id> <radius>`
- `cargo run --release --example set_pis -- template.dwg out.dwg <name|handle> "x,y x,y ..."`
- `cargo run --release --example dumprec -- file.dwg AECC_CLASS out/prefix`
