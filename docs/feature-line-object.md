# Feature lines — first look (S313–S318)

Samples from the owner (Civil 3D 2024), one change per step:

| ID | step |
|---|---|
| S313 | empty site `TEST` (description `felix`) |
| S314 | feature line `TEST FEATURE LINE` (8D60), three points, description "3 points" |
| S315 | fillet added at the middle point (description "4 points + curve R=50") |
| S316 | edits: an elevation changed to 200, an elevation point inserted |
| S317 | the feature line copied and pasted; the copy (8DAA) cannot be edited in Civil 3D (owner) |
| S318 | experiment: S317 with the new site `TEST|1` renamed `TEST1` (`exp_site_name`) |

## Objects

A feature line is one record, `AECC_FEATURE_LINE` (class
`AeccImpFeatureLine`); nothing else is created with it except two
XRECORDs (S314: 8D61, 8D62, replaced by new ones on later saves). Its
strings: the description twice, then the name. Its references: layer,
null, the **site container twice** (hard pointers), null, the feature
line style (`AECC_FEATURELINE_STYLE`). Its graphics are only a 3D
bounding box (S314: x 0–177.83, y 0–49.84, z 100–105).

The record keeps **no geometry**. Adding the feature line changed the
site (`AECC_PARCEL` with class `AeccImpFace`, 8C42: its graphics became
the same box) and the site's container (`AECC_FEATURE_STYLE` 8C43,
104 → 199 bytes), which now lists the feature line as a soft pointer
after its own owner, as it lists alignments in a site.

## Geometry: an edge of the site topology

The points are in the site container's topology graph, the one already
read for alignments in sites (`aecc_topology`, alignment-object.md
"Topology nodes"): nodes `BD x, BD y, BD z`, then an edge through them.
For alignments z is 0; for feature lines **z is the elevation**:

- S314: nodes 1 (0, 0, 100), 2 (100, 0, 105), 3 (177.83, 49.84, 103);
  one edge 1 → 2 → 3, edge flags 0x10830, vertex flags 50, 2, 10.
- S315 (fillet): node 2 is gone; the edge is 1 → 4 → 5 → 3 with the
  fillet's tangent points 4 (97.07, 0, 104.85) and 5 (102.47, 1.58,
  104.94) and the arc as a **bulge on vertex 4** (0.143365 = tan(Δ/4),
  Δ 32.64° = the deflection at the old point 2). Radius from bulge and
  chord: **10.0** — the description says R=50, but the stored geometry is
  a 10 radius. Vertex 4's flags are 0x10F0032.
- S316: node 5's elevation 200; node 6 (141.11, 26.33, 150.26) on the
  line from 5 to 3 is an **elevation point**, vertex flags 0x400400.
  After the edge's vertices the edge gains a list (`BL 1, BL 1`) where
  alignments have `BL 0` (`Edge::after`); the reader now takes
  `BL m, m × BL, BL 0 ×3` after each edge.

`aecc_feature_line::vertices` finds a feature line's edge by its place
among the container's soft pointers (the site's own pointer left out)
and returns the nodes with flags and bulges (tests
`tests/feature_lines.rs`).

So a feature line is a 3D polyline (lines and arcs) kept in the site's
point-and-edge graph, which is why feature lines in one site share
points and break each other where they cross: what the owner suspected
as "an amalgamation of alignment and profiles" is, in the file, the
same site topology as an alignment's with elevations on the nodes.

## The pasted copy (S317)

The owner copied the feature line and pasted it; the copy cannot be
edited with the GUI. What the paste did (S316 → S317):

- **A new site.** The copy is not in site `TEST`: Civil 3D created a site
  `TEST|1` (`AECC_PARCEL` 8DA8, same description `felix`, same styles)
  with its own container (8DA9), and listed it in the site collection
  (`AECC_PARCEL_NODE` C43: `TEST`, `TEST|1`). The copy (8DAA) points to
  8DA9. The original's site, container and record are unchanged.
- **The name.** `TEST FEATURE LINE (1) (1)` — renamed twice.
- **The geometry.** The same edge moved by (−0.2849, 57.0468), z and
  bulge unchanged; node IDs kept except the elevation point's, 6 → 8.
  The last vertex's flags are 2 instead of 10, and two values of the
  face entry after the edges differ (6 → 7, 22 → 19).
- **The container's references.** 8C43 has 28: owner, null, feature
  line, null, site, null, **null, null**, then 20 style slots of which
  the 7th is **null**. 8DA9 has 25: the two hard nulls and the null style
  slot are gone (the 19 styles follow one another).
- **Other objects.** Two anonymous blocks (`*U63`, `*U64`: lines,
  polylines, hatches) came with the paste; nothing references them.

The `|` in `TEST|1` is the character AutoCAD reserves for names that
come from an external reference (`xref|name`), and a paste inserts the
clipboard's drawing like a block; the leading guess is that Civil 3D
treats a site with such a name, and what is in it, as read-only. S318
renames the site to `TEST1` and changes nothing else, to test that
(MIST21). If S318's copy is still not editable, the dropped null
references are the next suspect.

## Open

- MIST21: why the pasted copy cannot be edited.
- The vertex flags (50, 2, 10, 0x10F0032, 0x400400) and the edge flags
  0x10830; the face entry after the edges.
- The record's data between the graphics and the strings; the two
  XRECORDs.
- The site's own data (its graphics follow its feature lines' extent).
- Feature line styles, labels, grading; feature lines from objects
  (polylines, alignments, corridors); siteless feature lines (the
  API allows none — every feature line is in a site?).
