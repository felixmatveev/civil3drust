# Feature lines — first look (S313–S320)

Samples from the owner (Civil 3D 2024), one change per step:

| ID | step |
|---|---|
| S313 | empty site `TEST` |
| S314 | feature line `TEST FEATURE LINE` (8D60), three points, description "3 points" |
| S315 | fillet added at the middle point (description "4 points + curve R=50") |
| S316 | edits: an elevation changed to 200, an elevation point inserted |
| S317 | the feature line copied and pasted; the copy (8DAA) cannot be edited in Civil 3D (owner) |
| S318 | experiment: S317 with the new site `TEST|1` renamed `TEST1` (`exp_site_name`) |
| S319 | S318 opened and saved by the owner: clean open and AUDIT; the copy still cannot be quick-edited |
| S320 | S319 with the copy exploded to a 3D polyline and turned back into a feature line (8E0E): editable |

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
  `TEST|1` (`AECC_PARCEL` 8DA8, same styles)
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

**Not the site name, not the references (S318, S319).** The `|` is
AutoCAD's separator for names from an external reference, so S318
renamed the site to `TEST1` and changed nothing else. Civil 3D opened it
cleanly (clean AUDIT), but the copy still cannot be edited with quick
edit; its grips move and the tabular editor changes its elevations
(owner). Civil 3D's save (S319) also dropped the original container's
null references, so the two containers now have the same references
apart from their own site and feature line, and the same data after
the edges (test `resave_normalises_containers_s319`). The two
feature line records differ only in their strings and graphics. In the
file, the original and the copy differ only in:

- their site (`TEST` vs `TEST1`, each with its own container);
- coordinates, the name, and node IDs (the copy's elevation point is
  node 8, and its next free ID is 9 where the original's is 7).

**Exploded and rebuilt (S320, editable).** The owner exploded the copy
to a 3D polyline and converted it back. The new feature line (8E0E) has
no name, no style (null reference) and empty descriptions. It went into
the **first site's** container (8C43, beside the original), not the
copy's `TEST1`. The arc became three chords (nodes 9–12, elevations
interpolated 104.85 → 200), and the elevation point became an ordinary
vertex. The first vertex's flags are 178, the others 50, and the last 2
and 10, as in Civil 3D's other rebuilt records (MIST12).

So the leading suspect is now **the second site**. The copy is the
only object in `TEST1`, and S320's working line is in `TEST`. Tests for
the owner (MIST21): in S319, *Move to Site* the copy into `TEST`; and
draw a new feature line directly in `TEST1`. Either result locates the
cause.

## Checked against the Civil 3D API (S316, S319)

`C3DDUMPALL` (owner; `samples/S316_fl_edit.api-all.txt`,
`samples/S319_S318_civ3d_resaved.api-all.txt`):

- `AECC_PARCEL` with class `AeccImpFace` is the API's **`Site`** (its
  `Name` `TEST`, `Description` empty, `StyleName` Property); the string
  after the name, `felix`, is the Windows user name, the same user string
  Civil 3D writes into an alignment's Properties form — not the
  description. The site container (`AECC_FEATURE_STYLE` owned by the
  site) has the site's name as its `Name`.
- Feature lines: `PointsCount` 5, `PIPointsCount` 4,
  `ElevationPointsCount` 1, `CurvesCount` 1, `SiteId`, `MinElevation`
  100, `MaxElevation` 200, `Length2D`, `Length3D`, `MinGrade`,
  `MaxGrade`, `RelativeSurfaceId` null. The counts, the site and the
  extreme elevations agree with the decoded vertices (test
  `counts_match_civil3d_api_s316_s319`): elevation points are the
  flagged vertices, all other vertices (including the arc's ends) are
  PIs, and each bulge is one curve.
- `C3DDUMP` on S316 (`samples/S316_fl_edit.api.txt`): `GetPoints(PIPoint)`,
  `GetPoints(ElevationPoint)` and `GetPoints(AllPoints)` are the decoded
  vertices (test `points_match_civil3d_api_s316`). x and y agree to the
  last digit; the z of the fillet's start (104.85362636185407 stored,
  …204809 in the API) and of the elevation point (…416952 stored,
  …43552 in the API) differ by 2e-10: Civil 3D recomputes these
  dependent elevations on open, and its re-save (S319) stores the
  recomputed values. No public property or method gives the arc's
  radius; the bulge is the only curve data (the add-in now also tries
  `Get…(int)` methods per index).
- The pasted copy (8DAA) reports exactly what the original does, apart
  from name, site and coordinates — including **`IsEditable: True`**. The
  API sees nothing that would stop editing (MIST21, parked).

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
