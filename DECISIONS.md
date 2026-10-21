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
