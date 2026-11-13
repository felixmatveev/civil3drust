# Brief: site study (analysis, then writing)

For a model picking this up in a new chat. Read this, then `CLAUDE.md`
and `README.md` (the cleanroom rules), before touching anything.

## The project in one paragraph

civil3drust (GitHub `felixmatveev/civil3drust`, Rust) decodes and edits
Civil 3D objects in DWG files on top of the `acadrust` crate. Every
format finding comes only from our own sample drawings, made by the
owner (Feliks) in Civil 3D 2024 and stored in `samples/` as
`S<n>_<name>.dwg`, from public specs, acadrust's source and observed
Civil 3D behaviour — never from Autodesk binaries or third-party
decoders. The end goal (D-049): open or create drawings, edit them, and
write files Civil 3D opens with no RECOVER, a clean AUDIT, and a
re-save that reads back to the same model.

## Goal of this study

Make **sites** fully readable and writable, in this order:

1. create an empty site; rename it, change its description; delete it;
2. move an alignment that crosses nothing into a site, and back out to
   siteless (today civil3drust only creates siteless alignments, D-038);
3. the same for a feature line;
4. later: alignments/feature lines that cross (shared crossing nodes,
   faces, parcels).

Steps 1–2 are the target of this round; 3–4 are stretch.

## What is already known (read these sections)

- `docs/alignment-object.md` → **"Sites (O7: S101–S103)"**, the
  following S104–S106 part, and **"Site containers with crossings
  (S112)"**: site = `AECC_PARCEL` (first string `AeccImpFace`; API
  `Site`), its container = `AECC_FEATURE_STYLE` hard-owned by it, the
  sites collection = `AECC_PARCEL_NODE`, what moving an alignment in and
  out changes (companion emptied, container grows, site record +180
  bits), the topology graph (nodes, edges, vertex face IDs a/b, start of
  the face list).
- `docs/feature-line-object.md`: feature lines are edges of the same
  container graph with z = elevation; site and container references;
  API checks (`C3DUMP` output). The pasted-copy oddity (MIST21) is
  parked — ignore it.
- `docs/open-blocks.md` → "Shared objects": what is unread in sites and
  containers.
- `DECISIONS.md` D-038 (siteless only), D-062 (deleting siteless
  alignments), D-064 (companion/registry rebuilt on create/delete),
  D-076 (feature lines read from the topology).
- `docs/mysteries.md`: open questions are numbered MIST1…; never
  renumber, add new ones at the end.

**Unread, and needed for writing:** the face list after the edges (only
its header and first entries are known), the site record's own data
(the 180 bits that appear when a member is added), the container's
header before the nodes and its style slots, the parcel records Civil 3D
creates for closed regions.

## Code entry points

- `src/aecc_site.rs`: `sites`, `site_handles`, `membership`,
  `companion_refs`.
- `src/aecc_topology.rs`: `node_list`, `graph` (nodes, edges, vertices
  with flags/face IDs/bulge, `Edge::after`), `companion_graph`,
  `rebuild_companion*`, `refresh_companion` — the writer for siteless
  companions; a container writer will be its sibling.
- `src/aecc_registry.rs`: alignment registry and companion keys.
- `src/aecc_feature_line.rs`: feature line → its edge in the container.
- `src/aecc_create.rs`, `src/aecc_delete.rs`: how a new alignment is
  registered and removed (siteless).
- `src/raw.rs` (`open`, `records`, `set_record`, `class_of`),
  `src/handle_refs.rs`, `src/string_stream.rs` (string read/replace):
  the raw-record plumbing every writer uses. **Always open drawings with
  `raw::open`.**
- Tests to keep green and extend: `tests/site_samples.rs`,
  `tests/topology_samples.rs`, `tests/feature_lines.rs`.

Tools (full list in `CLAUDE.md`): `cd workbench && cargo run --release
-- --diff a.dwg b.dwg --show all` (record-by-record diff of two steps),
`--related file.dwg <handle>`, `--objects file.dwg`; `cargo run
--release --example refs -- file.dwg AECC_FEATURE_STYLE` (handle
streams), `--example dumprec -- file.dwg AECC_PARCEL out/prefix` (raw
record bits for scratch Python), `--example sites -- file.dwg`.

## Samples to ask the owner for

One change per step, each saved, each with `C3DUMP` run (default
`control` classes; the add-in is `tools/c3d_api_dump`, configure with
`C3DCONFIG`). Round coordinates so crossings are known by hand.
Check `git ls-tree origin/main samples/` for the next free S-number
before naming them.

| step | change |
|---|---|
| SITE_01 | new empty drawing, one empty site `A` |
| SITE_02 | a straight two-tangent alignment, siteless (`<None>`) |
| SITE_03 | that alignment moved into `A` |
| SITE_04 | a second alignment, not crossing, created directly in `A` |
| SITE_05 | a third alignment crossing the first once (X, no closed area) |
| SITE_06 | a fourth alignment closing a triangle (Civil 3D should create a parcel) |
| SITE_07 | a feature line in `A` crossing one alignment |
| SITE_08 | site properties: rename `A` → `B`, description, numbering/style options (screenshot of the dialog) |
| SITE_09 | second empty site `C`; the feature line moved into it |
| SITE_10 | the SITE_02 alignment moved back to `<None>` |
| SITE_11 | site `C` deleted (owner notes what happens to its feature line) |

## How to work (owner's rules, from CLAUDE.md)

- **Batches:** while samples come in, analyse with scratch scripts and
  existing tools; write findings into `docs/` (a new
  `docs/site-object.md` is the natural home) and add the samples with
  `samples/README.md` rows (how made, size in bytes, first 16 hex chars
  of SHA-256). Change library, tests and workbench **once per round**,
  when the owner closes it.
- Every finding is backed by named samples and the step that showed it.
- Prove writes in Civil 3D: write an experiment drawing (an
  `examples/exp_*.rs`, listed in `CLAUDE.md`'s tools), send it to the
  owner, record what Civil 3D does (open, AUDIT, re-save) in
  `docs/civil3d-rejects.md` / `docs/civil3d-rebuilds.md`. Write only
  model data and the plumbing Civil 3D proves it needs; never copy
  caches, history or timestamps; never ship Autodesk style content
  (D-015).
- Before every commit: `cargo fmt`, `cargo test`, `cargo clippy
  --all-targets` clean, and the workbench (`workbench/`) builds and its
  tests pass. Commit and push straight to `main` (`git pull --rebase`
  first; other sessions commit too). Each change: CHANGELOG
  `[Unreleased]` entry, `DECISIONS.md` entry for design choices,
  `COMPATIBILITY.md` row updates (Sites, Feature Lines).
- Don't touch `archive/` (hexview, viewer). Civil 3D 2027 and LandXML are
  low priority.

## Definition of done for this round

- `docs/site-object.md`: site record, container (header, nodes, edges,
  faces, style slots), collection, parcels — each field with the samples
  that show it; unread parts listed.
- Readers: face list and site record fields in `aecc_topology` /
  `aecc_site`, tested on the new samples and on S101–S112, S313–S320.
- Writers proved in Civil 3D by experiments: create an empty site;
  rename/describe it; delete it; move a non-crossing alignment into a
  site and back out. Each with a `c3dr` command once proven.
- `COMPATIBILITY.md` Sites row moved from Read-only to Partial with the
  exact scope.
