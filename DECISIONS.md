# Design decisions

A running log of why things are the way they are. Add new entries at the
bottom; don't rewrite old ones — if a decision changes, add a new entry that
supersedes it and note that on the old one.

Template:

```
## D-NNN: Title
Date: YYYY-MM-DD · Status: accepted | superseded by D-NNN

Context — what problem or question came up.
Decision — what we chose.
Alternatives — what else was considered and why not.
```

---

## D-001: Build on acadrust, license MPL-2.0
Date: 2026-10-20 · Status: accepted

**Context.** Reading and writing Civil 3D objects requires a working DWG
reader/writer underneath. Writing one from scratch is a multi-year effort.

**Decision.** Depend on the `acadrust` crate for DWG/DXF I/O and license this
project MPL-2.0 to match it.

**Alternatives.** LibreDWG (GPL, C, would force GPL and FFI); ODA SDK
(proprietary, licensing conflicts with a cleanroom open-source project).

## D-002: Tangent-only alignments first
Date: 2026-10-20 · Status: accepted

**Context.** Civil 3D alignments can contain lines, arcs, spirals, and
fixed/floating/free constraints. Decoding all of it at once makes it hard to
tell which bytes mean what.

**Decision.** The first milestone handles only alignments made of straight
tangents. Curves and spirals come after a tangent-only round trip opens
cleanly in Civil 3D.

**Alternatives.** Start with LandXML import/export (easier, well-specified,
but doesn't prove DWG round-tripping, which is the hard part).

## D-003: Editing a segment length translates everything downstream
Date: 2026-10-20 · Status: accepted

**Context.** "Change segment *i*'s length" has several possible meanings: move
only the end PI (changes the next segment's direction and length), or move
the end PI and shift everything after it.

**Decision.** `set_segment_length` slides the end PI along the segment's
direction and translates all downstream PIs by the same offset. Every other
segment keeps its direction and length; only downstream stations change.

**Alternatives.** Moving only the end PI would silently change the next
segment's bearing — surprising for a "length" edit. That behavior can be
added later as a separate operation if needed.

## D-004: Keep geometry separate from DWG decoding
Date: 2026-10-20 · Status: accepted

**Context.** The DWG object layout for alignments is unknown and will change
as we learn more. Geometry rules (stationing, editing) are stable.

**Decision.** `alignment` holds pure geometry with no acadrust types. DWG
decode/encode code will convert to and from it in a separate module. This
also lets the same model back a LandXML reader/writer later.

## D-005: Study AECC records through acadrust's raw bytes only
Date: 2026-10-20 · Status: accepted

**Context.** acadrust keeps records of unknown classes as opaque bytes and
writes them back verbatim. Format study needs those bytes, and the cleanroom
rules limit which tools and sources may inform the decoding.

**Decision.** All byte-level study uses acadrust's stored record bytes
(`raw::records`) and tools in this repository (`rawdiff`, `dumprec`).
Offsets in `docs/` are bit offsets into those bytes. Edits are same-size
in-place patches of decoded fields until the surrounding encoding is
understood, so record sizes, strings and handles do not change.

**Alternatives.** Dumping with LibreDWG's `dwgread` (used once for early
exploration before this repo existed, as a byte dumper only): it adds a GPL
tool to the workflow and its bit offsets don't match acadrust's record
framing. The ODA SDK: excluded by the cleanroom rules.

## D-006: Controlled sample pairs are both saved by Civil 3D
Date: 2026-10-20 · Status: accepted

**Context.** Opening and saving a drawing in Civil 3D 2024 rewrote about 150
unrelated AECC records (styles, settings) in addition to the edited
alignment (docs/alignment-object.md, S2 vs S4).

**Decision.** Each pair of samples compared with `rawdiff` must be made the
same way and both saved by Civil 3D, differing only in the controlled
change. A file written by civil3drust is compared with its Civil 3D re-save,
not with the original, to see what Civil 3D adds or corrects.

**Alternatives.** Comparing a Civil 3D file with a civil3drust-written file
directly: mixes our edits with Civil 3D's save-time rewrites.

## D-007: Decode alignment geometry from element records, not the display cache
Date: 2026-10-20 · Status: accepted

**Context.** Two ways to read a tangent alignment were found: the polylines
in the proxy-graphics display cache, and the tangent element records in the
data section (docs/alignment-object.md). Samples S1 and S5 were saved before
Civil 3D drew the alignment, so their display cache is only a placeholder
text and box.

**Decision.** `aecc_alignment::decode` reads the tangent element records and
orders them by their before/after links. The display cache is treated as
derived data: it is not used for decoding, and an editor will update it
only so plain AutoCAD shows the change (Civil 3D redraws it).

**Alternatives.** Reading the display cache: fails on placeholder graphics,
shows only what the alignment style draws, and can't tell elements apart
(one polyline per tangent, but also arrow shafts and other primitives).

## D-008: Edits leave the volatile header field and label groups alone
Date: 2026-10-20 · Status: accepted

**Context.** When Civil 3D lengthens a segment (S5 → S6) it changes the
tangent records, the display cache, an unidentified ~64-bit field in the
record header, and the placeholder boxes of three label-group records
(docs/alignment-object.md).

**Decision.** `aecc_alignment_edit::apply` patches the tangent records,
cache records and the alignment's display cache, and leaves the volatile
field and label groups unchanged. Evidence: a file edited this way (S3)
opened, worked and was re-saved by Civil 3D, which regenerated both; and
the edited S5 (S7) matches Civil 3D's S6 bit for bit apart from that field.

**Alternatives.** Guess a new value for the volatile field: no basis yet,
and a wrong guess could be worse than a stale one. Recompute label-group
boxes: depends on label text metrics we don't model; Civil 3D redraws them.

## D-009: Read arcs through an element model; keep editing tangent-only
Date: 2026-10-21 · Status: accepted

**Context.** S10 and a production drawing showed how arc elements are
stored (docs/alignment-object.md). The geometry model in `alignment` is a
PI list that can only represent tangents (D-004), and the editor's
correctness was established only for tangents.

**Decision.** Add `aecc_elements` with an `Element` enum (tangent | arc),
arc records with their three points and derived circle, and
`decode_elements` for read-only use. `aecc_alignment::decode` stays
tangent-only and now returns `HasCurves` for alignments with arcs, so the
editor refuses them. The `alignment` model is unchanged until arc editing is
studied with controlled samples.

**Alternatives.** Extend `Alignment` to arcs now: premature, since we don't
yet know how Civil 3D constrains arcs (fixed, floating, free) or what an arc
edit changes. Treat arcs as unknown: loses information already decoded.

## D-010: Resolve free arcs from their neighbouring tangents
Date: 2026-10-21 · Status: accepted

**Context.** S11 showed a second arc kind: a free arc that stores only its
radius, with the neighbouring tangents keeping PI-to-PI coordinates
(docs/alignment-object.md). An element's stored values are therefore not
always the geometry it occupies.

**Decision.** `aecc_elements` keeps the stored records (`Element`, with
`ArcDef::ThreePoint` or `ArcDef::Free`) and adds a resolution step that
produces `Segment`s: a free arc is the fillet of the given radius between
its neighbouring tangents, and those tangents are trimmed to its PC/PT.
Callers that need lengths and stations use the segments; editors must
patch the records.

**Alternatives.** Store resolved geometry only: loses the distinction an
editor needs (moving a PI moves a free arc automatically; a three-point
arc must be patched). Treat free arcs as unsupported: S11 shows the model
is simple and verifiable.

## D-011: Radius edits patch the radius and cached PC stations only
Date: 2026-10-21 · Status: accepted

**Context.** Changing a free arc's radius in Civil 3D (S11 → S12) changed
the radius, two cached PC-station copies, the volatile header field, and
added a block of three BDs (2, 3, 4) (docs/alignment-object.md).

**Decision.** `aecc_arc_edit::set_free_arc_radius` patches the radius and
the cached PC-station copies (found as BDs after the element records whose
value matches the old PC station to 1e-9), and checks the new radius fits
on the neighbouring tangents. It does not write the volatile field or the
2/3/4 block, following D-008. The result (S13) matches S12 apart from
those and 3×10⁻¹⁴ in the station.

**Alternatives.** Insert the 2/3/4 block: its meaning is unknown and it
changes the record size, which nothing in civil3drust does yet. Leave the
PC station stale: Civil 3D might trust it for stationing until it
recomputes.

## D-012: Choose the arc layout from its parameters, not the flag bit
Date: 2026-10-21 · Status: accepted (refines D-010)

**Context.** The arc decoder used the bit after the header to choose
between "by three points" (1) and "free, by radius" (0). In S14 (S13
re-saved by Civil 3D) that bit is 1 on an arc that is still free (p1 = −1,
n2 = 29, radius only), so S14 failed to decode.

**Decision.** Pick the layout from the parameter block: n2 = 29 or
p1 = −1 → free; n2 = 2 or p1 = 60 → by three points; anything else → try
free, then three points, each fully validated. Keep the bit as
`ArcRecord::flag`, meaning unknown.

**Alternatives.** Keep using the bit and treat S14 as an exception: wrong,
since Civil 3D itself writes it this way.

## D-013: Tangent edits work on the PI polyline when curves are free
Date: 2026-10-21 · Status: accepted

**Context.** With free arcs, tangent records keep PI-to-PI coordinates
(S11), and lengthening a tangent in Civil 3D (S11 → S15) changed only that
tangent's record, the placeholder box and two unexplained values.

**Decision.** `aecc_alignment_edit::set_tangent_length` takes a decoded
element chain, treats the tangent records as the PI polyline, applies the
D-003 PI edit through the existing `apply`, updates cached PC stations of
free arcs whose station changes, and refuses the edit if a curve would no
longer fit. Segments are numbered by tangent (curves not counted), as in
Civil 3D's PI-based view. Alignments containing arcs by three points are
refused until editing around fixed arcs is studied. The `set_length`
example uses this path for all alignments.

**Alternatives.** Keep separate editors for tangent-only and curved
alignments: duplicate logic; the PI view covers both. Shift fixed arcs
rigidly: plausible, but not yet observed in a sample.

## D-014: Generate alignments by rewriting a template of the same shape
Date: 2026-10-21 · Status: accepted

**Context.** The owner asked for a DWG with a given alignment produced by
civil3drust. Creating an `AECC_ALIGNMENT` record from nothing needs the
record prefix, section 2 and the handle stream (links to style, site and
label groups), none of which are decoded.

**Decision.** Generate by rewriting a template record with the same shape
(`set_pis`: same number of PIs, free curves in the same places), reusing
the tested PI-edit path. Where no exact template exists, a shape with an
extra zero-deflection PI can stand in. Element insertion/removal and record
creation stay out of scope until studied with samples.

**Alternatives.** Build records from scratch: not possible yet. Resize
records to drop or add elements: changes record size and probably counts
in section 2; needs controlled samples first.

## D-015: Never ship Autodesk-provided styles; reference or copy from the user's drawing
Date: 2026-10-21 · Status: accepted

**Context.** Creating alignments needs a style, label set and site. Civil 3D
templates supply default styles, but Autodesk's Terms of Use put
Autodesk-provided content under the software's license (section 8.4) and
forbid reproducing or distributing any portion of the software (section
8.1), while users keep ownership of their own output (section 5). Not legal
advice; this is the conservative reading.

**Decision.** civil3drust ships no Autodesk-provided style data. New
alignments reference styles that already exist in the target drawing, or
copy them at runtime from a drawing the user supplies (the user's own
data). If built-in styles are ever needed, they are authored from scratch
with our own names, layers and values; only the format knowledge comes
from samples. Sample drawings are not mined for style libraries.

**Alternatives.** Embed the default template's styles: redistributes
Autodesk content. Create Civil 3D drawings from plain AutoCAD drawings:
needs the whole settings tree (about 1,500 settings nodes and hundreds of
styles per drawing); out of scope.

## D-016: Decode styles from the owner's own minimal styles
Date: 2026-10-21 · Status: accepted (extends D-015)

**Context.** Style records must be understood to create alignments that
use a chosen style, and eventually to author styles. Template styles are
Autodesk content (D-015).

**Decision.** Style records are studied from styles the owner creates in
Civil 3D with *New* (not *Copy*), with own names and generic values
(starting from a "nothing" style such as `CR-Empty`), changing one option
per sample. Only format knowledge goes into civil3drust; it writes style
records from decoded fields, never from copied bytes.

**Alternatives.** Study the template's default styles: works technically,
but mixes Autodesk content into the baseline and invites copying it.

## D-017: Viewer as a separate egui crate
Date: 2026-10-21 · Status: accepted

**Context.** A visual check of what civil3drust decodes, against the proxy
graphics Civil 3D saved, is useful for development and demonstration. It is
not part of the library.

**Decision.** `viewer/` is its own Cargo project (own `[workspace]`, not a
member of the library's) using eframe/egui, drawing everything through one
small 2D canvas trait so the same code renders the window and headless SVG.
The library gains no GUI dependencies and its `cargo test` stays fast.

**Alternatives.** Tauri: a web front end plus IPC for what is a few hundred
lines of line drawing. tkinter: needs Python bindings to civil3drust first.
A full 3D engine (wgpu/bevy): overkill for wireframe display.

## D-018: Reuse of permissively licensed related projects
Date: 2026-10-22 · Status: accepted

**Context.** Other open-source projects already implement parts of what
civil3drust needs, e.g. CivilCraft (Apache-2.0, JavaScript) has LandXML 1.2
I/O and alignment/profile geometry. Rewriting them blind wastes effort;
copying without records makes them hard to maintain and credit.

**Decision.** Related projects are listed in `SIMILAR_PROJECTS.md` with
license, reviewed commit and what is useful. Code from projects with
compatible permissive licenses may be ported: ported files keep their
original SPDX license, upstream path and commit, copyright line and a
"modified" note, and are recorded in `THIRD_PARTY.md` (crate license then
`MPL-2.0 AND Apache-2.0` where applicable). Otherwise they serve as
black-box test oracles pinned to a commit, credited in the docs. Such
projects are not sources for DWG/AECC format findings; the README's
cleanroom sources are unchanged. Their names and trademarks are used only
to identify the source.

**Alternatives.** Library linkage: not practical across JS/Rust. Ignore
them: loses free test vectors and a second implementation to check against.

## D-019: Change handle references in place, same byte length only
Date: 2026-10-22 · Status: accepted

**Context.** An alignment's style is a hard pointer in its handle stream
(S18/S19). A handle reference stores only as many bytes as the handle
needs, so pointing at a handle of a different byte length (e.g. 9B4 → 8DE7
is fine, 9B4 → 12345 is not) would change the record's bit length, and the
handle stream's size is also recorded by acadrust (`dwg_handle_bits`).

**Decision.** `handle_refs::retarget` only rewrites references whose new
handle has the same minimal byte count; otherwise it refuses and the caller
reports `SizeChange`. Every other edit in civil3drust is same-size too.
Resizing records (also needed for start stations of 0/1 and for adding
elements) will be handled once, for all edits, when creation needs it.

**Alternatives.** Re-encode the whole handle stream and update
`dwg_handle_bits` now: more code before any sample shows that Civil 3D
accepts a resized record from us.

## D-020: Locate style display components by a marker run, not a parser
Date: 2026-10-22 · Status: accepted

**Context.** The visible flag of a style display component is one bit
(S21 → S22), but the fields before it aren't decoded, so its offset can't
be computed by parsing from the record start.

**Decision.** `aecc_style` finds the component blocks (17 since S24; first
written as 16 with a 20-bit run that missed the last block) by a fixed
18-bit run present in each, and takes the flag 47 bits before it. It refuses any
record where the run doesn't occur exactly 17 times. It does not update the
style's modified-time stamp (Civil 3D does; S23 shows we can leave it).

**Alternatives.** Decode the style record field by field first: correct
long-term, but needs many more samples; the marker approach is checked on
every alignment style in S1–S24 and will be replaced when the layout is
decoded.

## D-021: First alignment creation by transplanting a donor's records
Date: 2026-10-22 · Status: superseded by D-025

**Context.** Creating an alignment adds four records (S24 → S25). The
companion registry's entry layout (an ID string plus a reference and some
bits) and parts of the alignment record aren't decoded, so they can't be
written from scratch yet.

**Decision.** `create_alignment` takes a donor drawing — the target plus
one alignment made in Civil 3D, from the owner's own samples — and copies
those records into the target, patching handles, references, the ID
string, the name and the PIs. The name must keep its length and the PI
count must match, so no record changes size. The name counter and command
settings are left alone.

Update (S30): optional records are selected with `CreateOptions`; leaving
out the parcel entry nulls the alignment's companion references, which
changes the record size — only the handle stream at the record's end is
rebuilt, the data stream is untouched.

**Alternatives.** Decode the registry and alignment fully first: the
right end state, but a transplant tells us now whether Civil 3D accepts
records we place, which de-risks the from-scratch writer.

## D-022: Files written by civil3drust identify as non-Autodesk
Date: 2026-10-22 · Status: accepted

**Context.** Civil 3D shows a "Non Autodesk DWG" notice when opening
S26/S27 (and other files written through acadrust): DWG files saved by
Autodesk products carry a marker identifying them as such, and files from
other writers don't.

**Decision.** civil3drust and the files it writes never imitate that
marker or otherwise claim to be produced by Autodesk software. The notice
is expected and harmless (AUDIT is clean); users who don't want it can turn
the check off in their own AutoCAD/Civil 3D settings, and any file
re-saved by Civil 3D no longer shows it.

**Alternatives.** Reproducing the marker: misrepresents the file's origin,
is not cleanroom knowledge we may use, and touches Autodesk's trademarks.

## D-023: Parcel companion set aside
Date: 2026-10-22 · Status: accepted

**Context.** Every alignment has a parcel-topology companion
(`AECC_FEATURE_STYLE`) and an entry in the registry owned by the alignment
parcel node. Without it Civil 3D's AUDIT reports 2 errors (S30), but after
AUDIT fixes them it accepts null companion references (S32: 0 errors on
re-audit); its stored points are recomputed by AUDIT anyway.

**Decision.** civil3drust doesn't decode or maintain the parcel companion
for now. `create_alignment` keeps copying the donor's parcel entry by
default (gives a clean AUDIT with no user action); `--no-parcel-entry`
remains for experiments. Parcels are out of scope until needed.

**Alternatives.** Decode the companion and registry entry now: effort
better spent on alignment creation itself (any name length, PI count,
curves) since nothing user-visible depends on it yet.

## D-024: Alignment geometry rebuilt from PIs, not patched
Date: 2026-10-22 · Status: accepted

**Context.** Same-size patches (D-019, D-021) can't change the number of
elements. S44 showed that only the graphics, the element count, the
element records and a fixed element-0 cache depend on the geometry.

**Decision.** `aecc_build::set_geometry` rebuilds those parts from a PI
list with optional free-curve radii and copies every other bit of the
donor record unchanged (graphics: the donor's leading attribute chunks plus
one polyline chunk per tangent; a placeholder is kept as is). New
alignments get a fresh identity value. The layout is checked before writing
and the edit refused if a field isn't where expected.

**Alternatives.** Keep requiring a donor with the same PI count: not
usable. Write the whole record from scratch: the unknown regions (section
2, the post-cache block) would have to be invented rather than copied.

## D-025: Alignment creation from a template; registry rebuilt
Date: 2026-10-22 · Status: accepted

**Context.** D-021 needed a donor that was the target plus one alignment,
because the registry entry layout was unknown. S24/S25/S44/S48 decoded it
(key list, ID strings, soft-owner references, sorted by ID string).

**Decision.** `create_alignment` takes a template alignment (with its
companion and label group) from any donor drawing and adds a new entry to
the target's own registry: key = largest key + 1, new ID string, entries
re-sorted. Handles are rewritten at any byte length rather than patched in
place. The template's style must exist in the target unless another is
given; the label group is written only when its label style exists.

**Alternatives.** Keep the donor + 1 requirement: unusable on real
drawings. Write companions and the registry from scratch: companions still
contain undecoded parcel data, which Civil 3D regenerates anyway (S35), so
copying a template companion is enough.

## D-026: Model alignments as minimal parameters; caches are derived
Date: 2026-10-22 · Status: accepted (direction)

**Context.** The record map (docs/alignment-map.md) shows most of an
alignment record is either constant or derived from the geometry: proxy
graphics, the element-0 station and cache, the companion's points, the
element-ID list Civil 3D adds on save. Civil 3D regenerates several of
these itself (S35, S47, S51, S54/S55).

**Decision.** civil3drust's model of an alignment is the minimal set of
meaningful parameters — name, description, style, start station, PIs and
curve definitions (and later profiles, station equations). Everything else
is either a documented constant or a cache computed from the parameters
when writing. Caches are kept out of the model; if some must be written
for Civil 3D to accept a file, they live in separate, clearly labelled
cache writers. The S56 experiments decide which caches can be omitted.

**Alternatives.** Keep copying a donor's binary block: works, but it
hides what matters and drags undecoded data into every new object.

## D-027: Proxy graphics optional, on by default
Date: 2026-10-23 · Status: accepted

**Context.** S56/S57: Civil 3D accepts an alignment without proxy graphics
and regenerates them. The graphics are what AutoCAD, viewers and other
programs without Civil 3D draw (OpenCADStudio shows them; BricsCAD Lite did
not open S56 at all, reason unknown).

**Decision.** `create_alignment` writes graphics by default (one polyline
per tangent, D-024) and can omit them (`graphics: false`). The 66-bit
section-2 field is treated as a cache (copied as is; Civil 3D rewrites
it). The identity GUID is always a random v4 value.

**Alternatives.** Always omit graphics: smaller, closer to D-026, but the
alignment would be invisible outside Civil 3D until Civil 3D re-saves the
file.

## D-028: Alignment type values named only from samples

**Context.** O1 (S58/S59) showed the alignment type is a small integer:
1 = Centerline, 4 = Miscellaneous. Civil 3D offers more types (Offset,
Curb Return, Rail).

**Decision.** civil3drust names only values confirmed by our own samples
(`TYPE_CENTERLINE`, `TYPE_MISCELLANEOUS`); others are stored and reported
as plain numbers until a sample (S61) shows which type they are.
`create_alignment` keeps the template's type unless one is given.

**Alternatives.** An enum with all types guessed from their order in the
Civil 3D UI: convenient, but not backed by a sample.

## D-029: Class entries copied from the donor, numbered by position

**Context.** A Civil 3D drawing without alignments (S65) has no
`AECC_ALIGNMENT` class entry, and class numbers differ between templates
(S2 vs S24). DWG class numbers are the position in the class section plus
500.

**Decision.** `create_alignment` uses the target's class number for each
record it adds and, when the target lacks the class, appends the donor's
class entry (names and flags as stored in the donor) with the next number.
Class entries are drawing structure, not Autodesk template content, so this
is consistent with D-015.

**Alternatives.** Hard-coding the class entries: no donor needed, but the
flags and version fields would be guesses until compared across samples.

## D-030: Minimal drawings postponed

**Context.** S64–S73: Civil 3D opens a plain AutoCAD drawing holding only an
alignment, but uses only alignment styles from its own style collection,
and it adds its full infrastructure on the first save anyway.

**Decision.** For now alignments are created in drawings Civil 3D has
saved at least once (S65/S66). Minimal drawings (alignment, our own style,
a minimal collection) are a later goal and an advantage over the standard
template-based approach; findings so far are in alignment-map.md §6.

**Alternatives.** Build Civil 3D's style collection chain now: more
unknown records before the alignment work is finished.

## D-031: Design speeds move with the start station

**Context.** Civil 3D removes all design speeds when the start station of
an alignment changes (S82, owner checked), even when they would still lie
on the alignment.

**Decision.** civil3drust keeps the design speeds and shifts their
stations by the change of the start station (the speeds stay at the same
places along the alignment). Alignments without design speeds stay
without. Civil 3D's behaviour is available as an option
(`clear_design_speeds`, `set_start --clear-speeds`).

**Alternatives.** Copy Civil 3D (wipe the speeds): matches Autodesk, but
loses user data for no reason; the owner prefers the better behaviour.

## D-032: Hex viewer maps records in the decompressed objects section

**Context.** A hex view that labels bytes by object needs a byte offset
for each record. From R2004 on, sections are stored as compressed pages,
so raw file offsets do not correspond to records at all.

**Decision.** `hexview` reads the decompressed `AcDb:AcDbObjects` buffer
with acadrust's public `get_section_buffer`, locates records through the
handle map (`read_handles`) and frames them per the public ODA spec
(`MS size, R2010+ MC handle bits, data, CRC16`). That pane carries the
overlay; the raw file is overlaid only for R2000, where the section is
stored uncompressed. Labels come from acadrust's document (who decodes the
record) and civil3drust's decoders; the DWG type code is read from the
record itself so unmapped records still get a class name.

**Alternatives.** Overlay compressed pages in the raw file: possible only
as "this page belongs to section X", which says nothing about objects.
Patch acadrust to report offsets while reading: duplicates what the handle
map already gives.

## D-033: acadrust 0.6.7, with Architecture records restored to raw

**Context.** acadrust 0.6.7 reads every class whose DXF name starts with
`AEC` into its Architecture types, which catches all Civil 3D `AECC_*`
classes, and it re-encodes those records on save instead of writing them
verbatim (docs/acadrust-0.6.7.md). civil3drust found no AECC records under
0.6.7, and its edits rely on records being saved byte for byte. 0.6.7 also
reads the `PRESSUREPARTERRORBLOCK` `3DSOLID` that 0.6.5 dropped.

**Decision.** Move to 0.6.7 and have `raw::open` convert every
Architecture-typed record with DWG bytes back into an `Unknown` record
(`raw::restore_raw_records`), for real Architecture classes (`AEC_*`) as
well as `AECC_*`. civil3drust never edits Architecture objects, and a
verbatim copy is the safest thing to hand back to Civil 3D. The upgrade is
accepted on the evidence of the fingerprint comparison over all samples.

**Alternatives.** Stay on 0.6.5 pinned exactly: safe but keeps dropping the
solid. Read the bytes from the Architecture types and keep them: civil3drust
could read records, but every save would go through acadrust's
Architecture re-encoder, which has never been checked against Civil 3D.
Ask acadrust to exclude `AECC_` from its prefix test: worth reporting, but
the conversion is needed meanwhile and keeps working afterwards.


## D-034: hexview compares records with a recursive bit diff

**Context.** The owner wants to compare two drawings in hexview and paste
the differences into a conversation. DWG record data is a bit stream:
when a field grows (a string, a bit-coded double going from 2 to 66 bits)
everything after it shifts by a non-byte amount, so a byte diff or a
position-by-position bit diff reports the whole rest of the record as
changed.

**Decision.** Records are matched by handle. Changed records are diffed at
bit level: common prefix and suffix removed, then the longest common run
of ≥ 64 bits (found through 64-bit keys) splits the window, recursively.
The data stream and the handle stream are compared separately, and decoded
fields are compared by value. The report is plain text, copied with one
button, with each region tied to the decoded field before it.

**Alternatives.** A full LCS/Myers diff over bits: exact but quadratic for
records of 10⁴–10⁵ bits and fragmented on random-looking data (tried
earlier, see O1 notes). A byte-level diff: simple, but misaligned after the
first shift.

## D-035: Arc layout by constraint code; floating curves take the short way

**Context.** S94 (first tangent deleted in Civil 3D) turned the free curve
after it into a floating curve: constraint code `n2` 15, radius, start
point, ending tangent to the next element. The same edit set `p1` to 60
on both arcs, and the decoder used `p1 == 60` to mean "three points", so
a free arc with a design speed was misread. S89 shows `p1` is the design
speed at the curve (60, then 30 after a speed change).

**Decision.** The arc record layout is chosen by `n2` alone (29 free,
15 floating with start point, 2 three-point; other codes: try free, then
three-point). A floating curve is resolved from its start point, radius
and the next tangent's line; the record stores no turning direction, so
of the candidate curves the one with the smallest central angle is used.

**Alternatives.** Keep `p1` as a hint: wrong as soon as any curve has a
design speed. Store or guess the direction from the old geometry: no
such field was found in S94's record (n1, p2, p3 are unchanged from the
free form), and the short curve is the one Civil 3D displays.

## D-036: Element edits in place, following Civil 3D's own edits

**Context.** Adding and removing elements on an existing alignment
(current focus). `set_geometry` (D-024) rebuilds the whole element list
from PIs, which renumbers everything and loses floating tangents,
three-point curves and other constraints. S94–S96 show what Civil 3D
does instead: it keeps every other record and its ID, relinks the
neighbours, stores new records last with an unused ID, turns an orphaned
free curve into a floating curve, keeps stationing and updates the
first-element IDs in section 2.

**Decision.** `aecc_element_edit` edits the element list in place by
those rules and re-encodes only the links (and a tangent end point or a
curve's constraint where the rules require it); other records are
copied bit for bit. New curves get the highest ID + 1 (equal to Civil
3D's choice in S96 whichever rule it uses). Derived parts are rebuilt
rather than left as Civil 3D leaves them: the design-speed intervals
(Civil 3D's stale interval makes its speed editor fail, S94) — speeds
before the new start move onto it, speeds past the end are dropped — and
the optional element-ID list is removed (S50: optional). Edits whose
resulting form has no sample (interior tangents; a curve losing the
element it is tangent to) are refused.

**Alternatives.** Re-plan from PIs with `set_geometry`: simple but
destroys element identity and constraints, and its station cache layout
predates the interval block. Copy Civil 3D's stale speed state exactly:
reproduces a state that crashes Civil 3D's editor.

## D-037: Only the first chain is the alignment; detached pieces are kept

**Context.** Deleting the tangent between two curves in Civil 3D (S100)
leaves two disconnected pieces in one alignment record: two records
without a predecessor. The decoder required a single chain. The owner
checked Civil 3D: only the chain from the first element is the
alignment (stations 0+00–3+45.66); the other piece is drawn but has no
stations and every operation ignores it — a placeholder for a later
join.

**Decision.** Records are grouped into connected pieces. The piece that
starts at the first-element ID (section 2, S94) is the alignment
(`DecodedElements::elements`); length, stations, design speeds and
element edits refer to it alone. The other pieces are kept in
`DecodedElements::detached`: decoded, drawn in rebuilt graphics, counted
in the element count and copied unchanged by edits, but never edited
(refused) or stationed. A first version counted stations across the gap;
replaced the same day after the owner's check.

**Alternatives.** Station across the gap: not what Civil 3D does.
Refusing to decode such alignments: hides geometry Civil 3D shows and
blocks edits of the alignment proper.

## D-038: New alignments are siteless; site membership is read-only

**Context.** O7 (S101–S106): an alignment in a site is part of the site's
container, one record per site holding every alignment in it as a
topology graph (nodes, edges, faces — the alignments split the site into
parcels). Only the geometry points in it are decoded. Moving an alignment
out of a site makes Civil 3D create a new siteless companion.

**Decision.** civil3drust reads sites and each alignment's site
(`aecc_site`), creates alignments siteless only (also in drawings with
sites, S107), and refuses donor alignments that are in a site. Writing
site membership waits until the container is decoded.

**Alternatives.** Transplant a container from a Civil 3D donor: the graph
depends on the geometry of every alignment in the site, so a copied
container would describe the wrong topology. Write the container by
patching the geometry points only: same problem as soon as the site has
more than one alignment.

## D-039: Any alignment can be a template

**Context.** `create_alignment` refused S98's `Remove first` (its last
handle reference is a design check set, which `alignment_refs` required
to be null) and failed on `Chain` (`set_geometry` expects the plain
layout: fixed tangents, free arcs and a speed interval right after the
records).

**Decision.** `alignment_refs` exposes the last reference as `check_set`
(null or a check set). A template's check set is kept when the target
has that object, otherwise nulled with the "use design check set" box
cleared. `set_geometry` keeps its exact plain-layout path (tests against
S44/S49) and otherwise replaces the element list in place with
`aecc_element_edit::replace_elements`, rebuilding graphics,
first-element IDs and speed intervals. New alignments start at 0+00:
a template's start station is moved to 0 with its speeds (D-031); the
identity GUID is renewed only when the template has a speed interval.

**Alternatives.** Refuse such templates: forces the user to keep a plain
alignment around. Map check sets across drawings by name: no sample of
two drawings with different check sets yet.

## D-040: Companions are rewritten in the uniform form of S49

**Context.** A siteless alignment's companion holds its topology: nodes
at the element end points and one edge through them (polyline with
bulges). `create_alignment` copied the template's companion, so S107's
described the template's geometry. Civil 3D writes several variants
(vertex flags 2/10/50/178, a/b 0 or 1, edge flag 0x20, a face-like entry
present or not) whose meanings are unknown.

**Decision.** Rebuild the companion's nodes and the edge's vertices from
the geometry: node IDs 1… in path order, vertex flags 10, a = b = 0, a
bulge per arc. Keep everything else from the existing companion: object
header, key, **edge flags and the tail after the edge** (its face-like
entry must match the number of handle references), strings and handle
stream. Only companions already in that vertex form are rewritten.
(Revised after S109: the first version wrote S49's edge flags and tail,
and Civil 3D discarded the companions whose template had no face-like
entry.) Alignments in a site are not
touched (their graph is the site's). If a companion's graph doesn't parse,
it is left as it is.

**Alternatives.** Keep copying the template's companion: accepted by
Civil 3D (S107, S28) but wrong data. Copy the template's vertex flags
per position: they depend on history we can't reproduce. Verified: S110
→ S111, accepted and kept (renumbered only).
