# Sites — record, container, collection, parcels (S101–S112, S313–S375)

A site in the file is four things:

| part | class | what it holds |
|---|---|---|
| site record | `AECC_PARCEL`, first string `AeccImpFace` (API `Site`) | name, description, settings, extent point, parcel counters |
| container | `AECC_FEATURE_STYLE`, hard-owned by the site | the site's topology: nodes, edges (one per alignment or feature line), faces (the site and its parcels) |
| site list | `AECC_PARCEL_NODE` (one per drawing) | keys, names and pointers of all sites |
| parcels | `AECC_PARCEL`, one per closed region | a face of the container; made and deleted by Civil 3D |

An alignment companion (siteless alignment) is the same record as a
container with one edge and no styles. Earlier notes:
alignment-object.md "Sites", "Topology nodes", "Site containers with
crossings"; feature-line-object.md. This file replaces their layout
tables; the samples and experiments described there still stand.

## Samples of this round (S321–S331, owner, Civil 3D 2024)

One drawing, saved after each step. The alignments were drawn freehand
(coordinates around (2500, 7300)), not at round coordinates.

| ID | step | what changed in the file (apart from the dictionaries and XRECORDs rewritten on every save) |
|---|---|---|
| S321 | empty site (8D58, container 8D59) | — (no "before" drawing) |
| S322 | `Alignment - (1)` (8D6A) siteless: (2480.30, 7321.78) → (2548.82, 7405.54) → (2598.69, 7316.58) | alignment, label groups, companion 8D69, registry 12F9 |
| S323 | 8D6A moved into the site | alignment (two references; + the MIST2 block of any re-save), companion emptied, container, site |
| S324 | `Alignment - (2)` (8D8C) created in the site, crossing nothing | alignment, container, site |
| S325 | `Alignment - (3)` (8DA4), crossing (1) once at (2576.57, 7356.03) | alignment, container |
| S326 | `Alignment - (4)` (8DB7), crossing (1) at (2502.73, 7349.19) and (3) at (2576.57, 7325.58): a closed region | alignment, container, site, **parcel 8DBF** |
| S327 | feature line (8DD4) in the site, (2533.64, 7308.33, 0) → (2587.85, 7300.95, 5.47), crossing (3) at (2576.57, 7302.49) | feature line, container, site |
| S328 | Site Properties: renamed `B` | site, site list — nothing else (see "Open") |
| S329 | second site (8DEB, container 8DEC), the feature line moved into it | new site and container, site list, feature line (references), both containers, first site |
| S330 | 8D6A moved back to `<None>` | alignment (two references), new companion 8DFB, registry, container, site; **parcel 8DBF deleted** |
| S331 | second site deleted; Civil 3D deleted its feature line with it (owner) | site 8DEB, container 8DEC and feature line 8DD4 gone; site list |

The closed region of S326 is a quadrilateral, not a triangle: (4) crosses
(1) on its first tangent and (3) crosses it on the second, so the PI of
(1) is a corner of the parcel.

## Site list (`AECC_PARCEL_NODE`)

```text
object header …
BL 0, BL n, n × BL key (always the raw 32-bit form), 13 constant bits
string stream: "", then the names of the named sites
handle stream: soft → tree node, n × soft → site, soft → one site,
               then the label-collector styles (hard; two of them null
               in the S313/S321 template)
```

- **Key.** `0x8000_0000 | id` for a named site (S328 `B`: `80000AFB`),
  the same kind of key as in the alignment registry; the site record
  holds the same value. Civil 3D gives a new key on rename (S317 → S319:
  `…335C` → `…335E`, S327 → S328).
- **Sites whose name is not a string (S321–S327, S329–S330 8DEB).**
  The first site of this round is named `A` (API dump of S332: site and
  container `Name: A`), yet its record has four strings instead of
  five, the list has no string for it, and its key has no high bit:
  `0200D214`. The same value is in the template's
  `AECC_USER_DEFINED_ATTRIBUTE` 3620 — the expression `R/3<A<R`, which
  uses a name `A` — and in `AECC_CLASS_NODE` BDA, in every drawing of
  the series, whatever the site is called. So keys are IDs in a
  drawing-wide pool of names: a name the pool already has is stored as
  its ID alone; a new name gets `0x8000_0000 | id` and its string in the
  record and the list. The second site (key `00000101`) is the same
  case. Renaming the first to `B` (S328) gave it a string and a
  high-bit key; a literal `A` written over the pooled one (S337) came
  back from Civil 3D in the pooled form (S350). The pool's table is not in the
  drawing: the reader lists such a site with its key and names it from
  the IDs the samples have shown (`aecc_site::pooled_name`; MIST22).
  S352/S353 gave 19 such pairs: Civil 3D accepts any built-in ID as a
  site key and keeps it on save.
- **Order.** Key mod 8, then key — a hash table's order, the same as
  the container's nodes. It fits every list with two sites: S103
  (`…BDA`, `…BD7`), S104, S317 (`…3359`, `…335C`), S319, S329
  (`00000101`, `…AFB`), S347, S348. (An earlier reading, "by name,
  spaces ignored", fitted all but S348.) Civil 3D accepts another order
  and rewrites it: S340 was written `A SITE`, `B` and came back `B`,
  `A SITE` with new keys (S348). The strings follow the list's order,
  pooled names skipped.
- **The extra pointer** after the n sites is the site created last
  (S103, S317, S329); deleting that site left it **null** (S331).

## Site record (`AECC_PARCEL`, `AeccImpFace`)

Strings: `AeccImpFace`, `""`, [name], description, user name (`felix`).
The name string is absent for an unnamed site. Handle stream: 4 × hard →
layer, null, hard → site list, **hard owner → container**, parcel style,
null, parcel style ×2. With a parcel in the site (S326–S329) two more
references to the parcels' style (A27) appear, after the null and at
the end; they go when the parcel goes (S330).

Data after the proxy graphics (the graphics are a display cache: a box
around the members; they grow and shrink with them):

```text
760 bits, identical in every site of every sample except two fields:
  +108  BL key (raw; = the key in the site list)
  +288  BL 0x1600101F + 64-bit FILETIME: the date-modified block of
        MIST3; set at creation and by Site Properties (S328), not by
        adding or removing members
  (also in the block: the double 0.1 = curve tessellation mid-ordinate
  and 5.0 = site elevation of the 3D Geometry tab, at their defaults)
BD x, BD y, BD z      P: top-left corner of the members' extent
                      (min x, max y of the container's nodes); 0,0,0
                      while the site never had a member
BL flags              99 empty; 122 once it has had a member; 114 in S112
BL n (0 or 1), n × { BD x, BD y, BD z, BL 2, BL 1, BL 1 }
                      an entry with P as it was when the first member
                      arrived; dropped by Site Properties (S328) and
                      absent in a pasted site (S317 8DA8) — not needed
BL a                  0 in a never-used site, else 1
BL …                  three to four more values: 1, 1, 1 normally;
                      S112 (two parcels) 1, 1, 3; S326–S329 (one
                      parcel) 2, 2, 1, 1 — the parcel counters of the
                      Numbering tab, layout not settled (MIST23)
19 constant bits, strings
```

Checked on every site record of S101–S112 and S313–S331 (P against the
container's nodes: S324, S330, S314, S102).

Parcels use the same record with a shorter tail: `P` (a point inside
the parcel), flags 80, one entry `{P, 2, 1, face ID}`, `BL 0`. Their
handle stream: layer, null, **hard → site**, hard owner null, parcel
style, label-collector style.

## Container (`AECC_FEATURE_STYLE`) and companion

One layout for site containers and alignment companions. All values are
BL unless noted; the record's data ends exactly at the last list.

```text
object header
BL key                 0 for a site container; the registry key for a companion
77 constant bits       ending with the raw BL 0x80000001

NODES   next free ID, count, ID of the first node (0 if none),
        count × { BD x, BD y, BD z, bits 010, ID of the next node (0 = last) }

EDGES   next free ID, entry count,
        per entry:
          2                                  a freed entry (its member left), or
          0, ID, flags, vertex count, 0,
             per vertex: kind, flags, node ID, face a, face b
                         [kind 1: BD bulge, 0],
             m, m × value                    (`Edge::after`; elevation points)
             0, 0, 0

FACES   next free ID, count,
        per face: ID, ID, flags, loop count, loops × { node ID, k }

TAIL    X                      number of hard (style) references at the
                               end of the handle stream
        6 × { value, value }   counters: (1, 1) in a container, (1, 100)
                               in a companion; the first pair becomes
                               (1, 2) with one parcel (S326) and (1, 3)
                               with two (S112): next automatic parcel
                               number
        6 × { n, n × value }   lists, empty except the first with
                               parcels: {1} (S326), {1, 2} (S112)
```

Handle stream: `own−1` → the site (a companion: soft → registry); one
hard null per `after` value (S316); per **live edge**, in entry order,
`soft → member, soft null`; per **face**, in order, `soft → the site
(face 1) or the parcel, soft null` (a companion's face 1: two nulls);
then the X hard references. A parser built on this grammar reads all 105
container and companion records of S101–S112 and S313–S331 to their
last bit, with X, the edge references and the face references matching
the handle stream in each.

Details, each with the samples that show it:

- **Nodes** are stored in the order of `ID mod 8`, then ID (S327:
  16, 1, 17, 2, 18, 3, 19, 27, 12, 20 …; S48: 40–43, 39) — a hash
  table's order; Civil 3D re-reads any order (S110 → S111).
  IDs are never reused (S330: next free 28 with 8 nodes left).
- **Freed entries.** When a member leaves, its entry becomes `BL 2` and
  its nodes go (S329: the fifth entry, the feature line; S330: the
  first). Entries carry their ID; they are in ID order with the freed
  ones in place only until Civil 3D rewrites the record, which drops
  the freed entries and keeps the IDs of the rest (S343: `next 3,
  count 1`, one entry with ID 2; emptied companions: `2, 1, [2]` in
  S323, `2, 0` after a later save, S104). IDs are not reused.
- **Crossings** are nodes shared by both edges, vertex flags 1072
  (0x430); a feature line's vertex on an alignment has 0x800430 and the
  alignment's 1136 (S327). The crossing node keeps z = 0.
- **Vertex a/b** are the face IDs on the two sides of the segment that
  starts there (S326: (1) is 1/2 from node 20 through its PI to node 17;
  (4) is 2/1 from 20 to 21); 0/0 on the last vertex.
- **Edge flags.** 0x810 for an alignment moved into the site (S323) and
  for the new companion of one moved out (S330); 0x30 or 0x830 for one created in
  the site (S324: 0x30; S104, S325, S326: 0x830 — the 0x800 bit is not
  explained); + 0x80 once a later edge crosses it (S325 (1), S326 (3));
  0x10000 for a feature line. Vertex flags 50 / 2 / 10 / 178 / 242 vary
  with history (MIST12).
- **Faces.** Face 1 is the site; it gets its slot with the first member
  and keeps it, with no loops, when the last member leaves (S106 1069,
  S320 8DA9) — until Civil 3D rewrites the drawing, which drops it
  along with the free slots (S341). An empty, never-used container has no face slot but next
  free ID 2. One loop per connected group of edges: `(start node of the
  group's first edge, 0)` for a group that closes nothing (S323 `(1,0)`;
  S324 `(1,0),(12,0)`); a crossing node with k = 1, 2 or 3 where a region
  is closed (S326 `(20,3)`, parcel `(21,1)`; S330 `(21,2)`) — k probably
  selects one of the segments leaving the node. Face IDs are not reused
  (S330: one face, next free 3). Face flags: 14 with one fresh member,
  6 or 7 later (history, like MIST12).
- **Style slots.** A new container has 22 hard references: two nulls and
  20 style slots, one of them null (S313, S321); Civil 3D's rewrite of a
  pasted or re-saved container keeps the 19 non-null ones (S317, S319).
  The S101 template has one (851).

## The alignment's edge ID

An alignment record holds the ID of its edge in the site container (or
companion): one `BL`, 133 bits after MIST1 — after MIST2 when that is
present, 98 bits later in the Alignment Properties form
(alignment-map.md, section 2). Siteless alignments always have 1, which
is why it went unnoticed.

- S324/S326: 8D6A 1, 8D8C 2, 8DA4 3, 8DB7 4 — their edge IDs in 8D59.
- Checked on the 146 sample drawings between S24 and S344 that this
  check was run on (606 alignments): the field equals the edge ID in
  573, 38 of them with an ID above 1; 11 alignments have no edge at
  all; of the 22 that do not fit, 21 are in the connected (curb-return)
  alignment drawings S283–S297, whose records are laid out differently,
  and one is S335.
- A wrong value is an AUDIT error (S335 → S344): Civil 3D looks the
  edge up by this ID, does not find it, and adds the alignment to the
  container again.

Moving an alignment between a companion and a container, or between
containers, has to write the new edge's ID here.

## What each operation changes

**Create an empty site** (S98 → S101; the records of S313, S321): site record,
container (`1,0,0 / 1,0 / 2,0 / X / (1,1)×6 / six empty lists`), and
the site list (count, key, [name], pointer, last-created pointer).

**Rename / Site Properties** (S327 → S328): name string and key in the
site record and the list, the date stamp, and the entry after the flags
dropped. S318 → S319 showed that Civil 3D accepts the two strings alone
and re-keys.

**Delete a site** (S330 → S331): the site, its container and every
member object are removed (the feature line went with it); the site
list loses the entry and its last-created pointer becomes null. Nothing
else changes.

**Move an alignment into a site** (S322 → S323, S101 → S102):

- alignment: references 2 and 3 → the container, and its edge ID
  field → the ID of its new edge (1 in S323, so the data looked
  unchanged);
- companion: emptied — nodes `next, 0, 0`, edge slot free, no face,
  references cut to the registry pointer — and left in the registry;
- container: nodes, an edge slot (flags 0x810, a/b = 1), face 1 with
  the loop `(first node, 0)`, references `member, null` before
  `site, null`;
- site: P, flags 99 → 122, `a` 0 → 1 (and the optional entry).

**Move it back to `<None>`** (S329 → S330, S105 → S106): a **new**
companion (new key, new ID string, registry entry; edge 0x810, face
`1, 1, 7, 1, (1, 0)`, five references) — the emptied one stays; the
container frees the slot, drops the nodes and the member reference;
parcels that lose a side are deleted with their face; the site's P and
references follow.

**Move a feature line to another site** (S328 → S329): its two
container references; its edge leaves one container (free slot, crossing
node removed) and is rebuilt in the other with new node IDs from 1.

## Crossing and touching members (S354–S361, owner, Civil 3D 2024)

Four drawings with two straight alignments in one site, and a copy of
each in which Civil 3D moved alignment (2) to `<None>`. Alignment (1),
8C4F, is (0,0)–(100,0) in all of them; alignment (2), 8C58, varies.

| Case | Alignment (2) | Container 8C43 in the site |
|---|---|---|
| S354 cross | (50,50)–(50,−50) | node 5 = (50,0) shared; edge 1: 1 → **5** → 2, edge 2: 3 → **5** → 4; both middle vertices flags `0x430` (1072) |
| S356 tee | (50,0)–(50,−50) | node 6 = (50,0): start of edge 1 (6 → 7, start flags 50, end flags 10) and a middle vertex of edge 2 (3 → **6** → 4, flags `0x470` = 1136) |
| S358 end to end | (100,0)–(100,−50) | node 4 = (100,0): end of edge 2 (3 → 4, end flags 2) and start of edge 1 (4 → 8, start flags `0x800002`) |
| S360 arc | arc via (50,0) | node 24 = (50,0): the arc's edge 1 is 22 → **24** → 23, two kind-1 vertices with bulge −0.4142 each (two 90° arcs; the one-piece arc of S361's companion has bulge −1); arc vertex flags `0x10F00B2`, `0x10F0430`; the line's middle vertex `0x470` |

What this settles:

- **No edge is split.** A crossing or touching point is one node,
  shared, inserted into each edge as an extra vertex where the point is
  inside the edge; an endpoint on an endpoint or on another edge is the
  same node (S356, S358). Each alignment keeps exactly one edge entry,
  so the alignment's edge ID (D-079) does not change.
- **An arc is cut at the crossing**: its vertices are the pieces, each
  kind 1 with the bulge of its own piece (`tan(θ/4)`).
- **One loop per connected part.** The site's single face has one loop
  in every case: the two alignments are one connected part. (S343's two
  separate alignments gave two loops.) The loop's node is a node of that
  part; which one Civil 3D picks is not a rule we can see (2, 3, 3, 4 in
  S354/S356/S358/S360).
- **Vertex flags** at shared nodes: `0x430` where both edges pass
  through, `0x470` on the edge passed through by another edge's end or
  by an arc, `0x800002` on a start that is another edge's end. Arc
  vertices carry `0x10F0000` (also in companions). Flags are history
  elsewhere (MIST12); whether Civil 3D needs these exact values in a
  site container is untested.
- **The site's extent point P** (tail, `BD3`) is (min x, max y) of the
  geometry. With an arc it is not the arc's true top: S360 has
  P = (0, 60.3553) = the arc piece (50,0)–(100,50): chord midpoint
  y 25 + half the chord 35.3553. So Civil 3D bounds an arc piece by
  the circle on its chord; one sample, so a hypothesis.
- **Nothing else changes.** Neither alignment record changes when they
  meet or part, apart from the two container references of the one that
  moves (and one bit in S360/S361's arc element, MIST24). No new node or
  face object is made for a crossing.

Moving the crossing alignment out (S355, S357, S359, S361):

- Its edge entry is freed (`BL 2`); its nodes go, and so does the shared
  node, removed from the remaining edge, which is back to two vertices.
  The node counter is not lowered (IDs are never reused).
- The remaining edge's start flags change (178 → 242 in S355, S359,
  S361; unchanged 178 in S357) and S359's edge flags 0x890 → 0x810:
  history, as in MIST12.
- Civil 3D makes a new companion (8DC6, 8DD1, 8DDC, 8DE9) for the
  alignment and drops the freed entries of the two old empty
  companions 8C4E, 8C57; the registry 12F9 gains the new companion.
  The loop's node is renumbered to a node of the remaining part.
- P drops to the remaining geometry: (0, 0).

Our writers (2026-11-13): `site_move_in` refuses all four copies ("crosses
or touches", or arcs), and `site_move_out` refuses all four originals
("the edge crosses another…"), as designed. The samples give everything a
crossing move needs: intersection points, one shared node per point,
inserted vertices, arc pieces with their bulges, merged loops. The open
question for a writer is whether the vertex flags matter (an experiment:
S354's crossing written by civil3drust with plain flags, `0x30`/`0x10`).

## Spirals in a site (S368, S369, owner, Civil 3D 2024)

One alignment 8C49 — line, Free Spiral-Curve-Spiral (L 38.69, R 40.4944,
L 38.69), line — in `TEST SITE` (S368) and moved to `<None>` (S369).

- **A spiral is stored as arc pieces.** The container's edge (S368) has
  24 vertices: the first line, then **10 kind-1 pieces per spiral**, the
  circular arc as **one** piece (bulge −0.04934, its 11.30°), and the
  last line. Each spiral is cut into 5 parts of equal length L/5, each
  part drawn as two arcs; the pieces' turning angles (4·atan|bulge|) are
  k × 0.3650° with k = 1, 2, 4, 5, 7, 8, 10, 11, 13, 14 for the spiral
  into the curve (the reverse for the one out of it), and add up to the
  spiral's own deflection L / 2R = 27.37° to 4 decimals. A cache of the
  geometry, computed by Civil 3D.
- **The companion holds the same edge** (S369, 8C48): the same 24
  points and bulges, bit for bit, with companion flags (vertices 10,
  a = b = 0, no face) and new node IDs 19–42. Moving in or out copies
  the edge between companion and container.
- **The site's point P** (S368: (0, 0.0041)) fits a refined rule: per
  arc piece, the chord's bounding box grown on both sides by the
  sagitta (|bulge| · chord / 2) along the chord's normal; P = (min x,
  max y) over all pieces. It also gives S360's 60.3553. A cache: Civil 3D
  sets it on save (S342).
- **Decoder gap found:** civil3drust's element decoder places this free
  spiral-curve-spiral at the stored end of the first line (TS x = 53.49),
  so the group lands 264.73 too far back and the last line is not
  tangent to it. Civil 3D fits the group between the two lines (TS x =
  318.2213 in the container, the last line at −66.04° = the curve's exit
  direction). The free spiral between two lines (S165) is fitted that
  way already; the free spiral-curve-spiral is not (fix at round close).

**Experiments (S370, S371; `examples/exp_site_spiral.rs`, 2026-11-13).**
S369's alignment moved back into the empty site by civil3drust, its edge
taken from its own companion (`aecc_site_edit::move_into_site_with`,
hidden, experiments only):

| sample | the container's edge | Civil 3D 2024 (owner) |
|---|---|---|
| S370 | the companion's 24 vertices as they are (Civil 3D's arc pieces) | **accepted**: clean open, AUDIT clean. **Re-save S372** |
| S371 | only the element ends from the companion (start, TS, SC, CS, ST, end): each spiral a straight chord, the curve its one arc | **accepted**: clean open, AUDIT clean. **Re-save S373** |

**Result: the container's geometry is a cache.** Both re-saves hold the
same 24-vertex edge as Civil 3D's own S368 (points to 10⁻⁶, bulges to
10⁻⁹): from S371's six points Civil 3D rebuilt the spiral pieces from the
alignment, with new node IDs and its own flags, and set P = (0, 0.0041)
as in S368. With S366 (crossings rebuilt) this settles it: the edge's
points, pieces, shared nodes and flags are Civil 3D's planar-graph cache
for parcel boundaries, computed from the members. What a writer must
write is the structure — one edge entry per member with the member's
reference and its edge ID in the alignment, nodes for its vertices, a
loop in the site's face — and any reasonable polyline of the alignment;
the element ends are enough (owner: if we don't use it, we don't need
it). Parcels built from members are untested.

## Deleting a site with members (S374, S375, owner, Civil 3D 2024)

S374: site `TEST SITE` 8C44 (container 8C45) with two members — alignment
(1) 8C4B, a line (0,0)–(100,0), edge 2, and a feature line 8C53, edge 1
(edge flags 0x10030, three vertices); two separate parts, two loops.
S375: the site deleted in Prospector. **Deleting a site deletes its
contents** (owner; as S331 showed for a feature line).

Removed: the site 8C44, its container 8C45, the alignment 8C4B, six of
its seven label groups (station, geometry point, station equation,
design speed, superelevation, minor station — all `*_LABEL_GROUP`), and
the feature line 8C53. Changed:

- the site list C43 → the zero-site form `BL 0, BL 0`, no name strings,
  the last-created pointer null (as S338/S346);
- `AECC_ALIGNMENT_VAGEOMPOINT_LABELING` 8C50 (the vertical geometry
  point labels) **stays**, with its reference to the alignment (the 4th,
  code 4) set to null — Civil 3D does not delete it with the alignment;
- registry 12F9: 16 bits of its header region change (no reference
  removed; the alignment was in the site, not in the registry);
- the old emptied companion 8C4A stays as it was.

Unrelated to the delete (as S229): the class node 8BF0 and its
`AECC_USER_DEFINED_ATTRIBUTE_CLASSIFICATION` 8BF1 are written again
under new handles (8D83, 8D84), and style/criteria records change as on
any save.

For a writer, deleting a site with members = deleting each member as
Civil 3D does (the alignment with its label groups; the feature line)
and then the empty site (`delete_site`, S338). The alignment deletion
differs from the siteless one (D-062) only in that there is no companion
to empty. **Gap found:** `aecc_delete::delete_alignment` removes the
`*_LABEL_GROUP` records and leaves `*_LABELING` records that point at
the alignment as they are; Civil 3D nulls that reference (S375 8C50). To
fix at round close, for siteless deletes too.

## Write experiments (S332–S340, `examples/exp_site.rs`)

**Results so far**

| sample | Civil 3D 2024 (owner) |
|---|---|
| S332 move-in | **accepted**: opens with no RECOVER, AUDIT clean; Prospector lists the alignment under the site; API: `IsSiteless: False`, `SiteName: A`, the site's `GetAlignmentIds()` = 8D6A, the container's `Name` = `A`. So Civil 3D takes the container without the site's redrawn graphics, without the optional site entry, with vertex flag 50 where it writes 2, and with node coordinates from the alignment. **Re-save S342** (site renamed `Site A`, whole drawing rewritten): the alignment is still in the site and its record equals the one in Civil 3D's own S323; Civil 3D rebuilt the container from the alignment (new node IDs 4–6 with its own coordinate rounding, edge flags 0x30, vertex flags 50/2/10, face flags 7, 19 style references), redrew the site's graphics (bit for bit those of S323) and set the site point from its nodes; it did not add the optional entry |
| S333 move-out | **accepted**: clean open, no AUDIT issues; API: `IsSiteless: True`, `SiteName` empty, the site's `GetAlignmentIds()` empty. **Re-save S341** (the owner also renamed the site `Site A`; Civil 3D rewrote the whole drawing): the refilled companion 37FD stays the alignment's companion, same nodes, with edge flags 0x10 → 0x810 and vertex flags 10 → 2; the site's container is written compact — no edge slot (next ID still 2), **no face 1** (next ID 2), 19 style references; the emptied companion 8D69 likewise has no slot. So refilling an emptied companion is a valid way to make an alignment siteless, and a container's free slots and loop-less face are optional |
| S334 move-out, one of two | **accepted**: clean; API: `Alignment - (1)` siteless, `Alignment - (2)` in the site, the site's `GetAlignmentIds()` = 8D8C. **Re-save S343**: same model; the refilled companion left exactly as written (edge 0x10, vertex flags 10 — S341 changed them, so that is not a rule); the container rewritten with a single edge entry that keeps **ID 2** |
| S335 move-in again | **AUDIT: 1 error, fixed.** The alignment's record holds the ID of its edge (see "The alignment's edge ID"); S335 left it at 1 while the new edge was ID 3. Civil 3D's fix (S344): the alignment added to the container a second time as edge 4 on the same nodes, the written edge 3 left behind, the field set to 4. S332–S334 passed only because every edge there was ID 1 |
| S345 = S335 with the edge ID field set to 3 | **accepted**: no issues where S335 had an AUDIT error; API: both alignments in the site. **Re-save S351**: each alignment once in the container, edge 3 on the written nodes, the edge ID field still 3 |
| S336 rename + description | **accepted**: clean; API: site and container `Name: RENAMED`, `Description: written by civil3drust`, the members' `SiteName: RENAMED`. **Re-save S349**: both strings kept, a new key for the site |
| S337 literal name over a pooled one | **accepted**: clean; API: `Name: A`. **Re-save S350**: Civil 3D stores the name as the pool ID again (`0200D214`, no string) — a literal string for a pooled name is read and normalised |
| S338 delete the only site | **accepted**: clean; API: no site, no container. **Re-save S346**: the zero-site list is kept byte for byte |
| S339 create (clone of an empty site) | **accepted**: clean; API: `A` and `NEW SITE`, each with its container. **Re-save S347**: both kept; a new key for `NEW SITE` |
| S340 create (clone of a site with members, graphics stripped) | **accepted**: clean; API: `B` with its three alignments, `A SITE` empty. **Re-save S348**: Civil 3D drew the new site's graphics, re-keyed both sites and put the list in key order |


First writers, kept in the example until Civil 3D has accepted them.
The container is decoded to a model (nodes, edge slots, faces, tail,
references), changed, and written back whole; the site record's tail
and the site list are rebuilt the same way. Nothing is copied from
another drawing; `create` clones the drawing's first site.

| sample | command | tests |
|---|---|---|
| S332 | `move-in` S322: 8D6A → the empty site | the S322 → S323 step. Differs from Civil 3D's S323 only in the node coordinates' last digits, one vertex flag (50, 2 in S323), the site's graphics (not redrawn), the optional site entry (not written) and the alignment's MIST2 block (absent, as in S322) |
| S333 | `move-out` S323: 8D6A → `<None>` | last member leaves: free slot, face 1 without loops (S106 form), site point zeroed; an **emptied companion is refilled** (rebuilt form: edge 0x10, vertex flags 10, no face) instead of a new one |
| S334 | `move-out` S324: 8D6A, 8D8C stays | a free slot before a live one; loop and nodes of one member removed |
| S335 | `move-in` S334: 8D6A again | a site with a member and a free slot: edge ID 3, nodes 15–17, second loop `(15, 0)` in face 1 |
| S336 | `rename` S331: `B` → `RENAMED`, description | the two strings and the list's name only (as S318, plus the description) |
| S337 | `rename` S322: the pooled name `A` → a literal `A` | name string inserted, key `0x80000000 \| n` (largest key of the site list and the alignment registry + 1) in record and list (MIST22) |
| S338 | `delete` S322: the only site | site, container and list entry removed; zero-site list as in S24 |
| S339 | `create` S322: `NEW SITE` | clone of an empty site: handles (site, site + 1), key, name, empty tail and container, list entry, last-created pointer |
| S340 | `create` S331: `A SITE` | clone of a site with members: tail reset, **graphics stripped**, the template's date stamp kept |

**Crossing (S362, S363; `examples/exp_site_cross.rs`, 2026-11-13).** S355
with alignment (2) moved back into the site by civil3drust, crossing
alignment (1) as in S354: the move without the crossing test
(`move_into_site_unchecked`), then one new node 8 = (50,0) inserted as a
middle vertex into edge 2 (alignment 1: 3 → 8 → 4) and the new edge 3
(alignment 2: 6 → 8 → 7), and the new edge's loop dropped (one connected
part, one loop `(3, 0)`). The two drawings differ only in the inserted
vertices' flags — the question these settle:

| sample | inserted vertex flags | Civil 3D 2024 (owner) |
|---|---|---|
| S362 | 50, as our other middle vertices (plain) | **accepted**: no RECOVER, AUDIT clean; API: both alignments `SiteName: TEST SITE`, the site's `GetAlignmentIds()` = 8C4F, 8C58. **Re-save S364** |
| S363 | 1072 (`0x430`), as Civil 3D writes a crossing in S354 | **accepted**, the same API values. **Re-save S365** |

**Result: the flags at shared nodes are not data.** Both re-saves have the
same model: Civil 3D rebuilt the container from the alignments on its
save — new node IDs (9–13 in S364; 9, 10, 12–14 in S365), one shared node
at (50,0) inserted into both edges as we wrote it, its own flags
(edge flags 0xB0 / 0x30, start 178, end 10, crossing 1072 on both edges
in S364, 1136 and 1072 in S365 — history again, MIST12), face flags 7,
one loop `(10, 0)`, the edge IDs 2 and 3 kept, alignment records
unchanged. So a crossing is written with plain flags: what Civil 3D
needs is the shared node in both edges and one loop per connected
part.

**Without the crossing point (S366, `exp_site_cross … none`).** The
same move with no shared node: the two edges keep their own nodes
(alignment 1: 3 → 4, alignment 2: 6 → 7) and the face has two loops,
`(3, 0)` and `(6, 0)`, as if they did not cross. The question: does
Civil 3D compute crossings itself (then a writer never needs them, arcs
and spirals included), keep the container as written, or report it in
AUDIT? Civil 3D 2024 (owner): **accepted** — clean open, AUDIT clean,
no issues. **Re-save S367: Civil 3D computed the crossing itself.** Its
container 8C43 is byte for byte the one of S365 (Civil 3D's re-save of
S363, where we wrote the crossing): nodes 9, 10, 12, 13 and a shared
node 14 at (50,0) inserted into both edges, one loop `(10, 0)`, edge
IDs 2 and 3 kept.

So **a writer does not need to compute crossings**: Civil 3D rebuilds
the site's planar graph from its members when it loads or saves the
drawing, crossings included. Writing each member as its own edge with
its own nodes and loop — what `move_into_site` already writes for a
non-crossing alignment — is enough. One X of two straight alignments
proves it; tees, end-to-end joins and arcs are the same rebuild and
are worth one check each before relying on them.

For each: does it open without RECOVER, is AUDIT clean, what does
Prospector show (sites, the alignment's site), and a re-save. The
re-saves show what Civil 3D rebuilds (graphics, keys, vertex flags, the
entry) and go into `civil3d-rebuilds.md` / `civil3d-rejects.md`.

Limits of the experiment code: an alignment that crosses or touches a
member is refused (straight members only once the site has members);
spirals are refused; `delete` takes empty sites only; `create` needs a
site without parcels to clone.

## In the library (round closed 2026-11-12, D-078, D-079)

What the round settled, in one place:

| Finding | Samples | Where it lives |
|---|---|---|
| A site is three records: site (`AECC_PARCEL`), container (`AECC_FEATURE_STYLE`, handle + 1), entry in the site list (`AECC_PARCEL_NODE`) | S321–S331 | this file |
| The site list holds `BL n` keys in hash order (key mod 8, then key), names only for keys with the high bit, a last-created pointer | S103–S348 | `aecc_site::site_keys`, `site_entries` |
| A key without the high bit is one of Civil 3D's built-in names; its text is not in the drawing | S321, S352/S353 | `aecc_site::pooled_name` (19 IDs), MIST22 |
| Container/companion: nodes in hash order, edge entries with their own IDs (freed entries stay as `BL 2`, IDs never reused), faces with loops, counters, lists; handle stream in the same order | 150+ records | `aecc_site_edit::Topology` (parse and write), `aecc_topology::graph` (skips freed entries, gives edge IDs) |
| The alignment stores the ID of its edge; a stale value is an AUDIT error and Civil 3D adds a second edge | S335 → S344, S345 → S351 | `move_into_site`, `move_out_of_site`; `docs/civil3d-rejects.md` |
| Site record tail: extent point, flags (99 empty, 122 used), optional entry, three values | S321–S331 | `aecc_site_edit` (internal), `describe` |
| Civil 3D rebuilds: site graphics, the container graph (new node IDs, freed entries dropped), list order, the pooled form of a built-in name | S341–S351 | `docs/civil3d-rebuilds.md` |

Library calls and `c3dr` commands (the workbench runs the same commands):

| Edit | Library (`aecc_site_edit`) | Command | Proved by |
|---|---|---|---|
| List | `aecc_site::sites`, `membership`; `describe` | `sites` | — |
| Create an empty site | `create_site` (clones the first site without parcels) | `site_create <name>` | S339/S340 → S347/S348 |
| Rename, describe | `rename_site` | `site_rename <site> <name\|-> [--description t]` | S336/S337 → S349/S350 |
| Delete an empty site | `delete_site` | `site_delete <site>` | S338 → S346 |
| Alignment into a site | `move_into_site` | `site_move_in <alignment> <site>` | S332/S345 → S342/S351 |
| Alignment back to siteless | `move_out_of_site` | `site_move_out <alignment>` | S333/S334 → S341/S343 |
| Site under a built-in name ID | `create_site_with_builtin_name` | (`exp_site pool`) | S352 → S353 |

Refused, with a message: a move that would cross or touch a member; a
site with parcels as target or as template; alignments with spirals;
arcs when the site already has members (no arc crossing test); moving
out an edge that shares a node or has elevation points; deleting a site
with members; moving out when the registry has no emptied companion to
refill. `tests/site_edit.rs` regenerates S336–S339 and S345 record for
record and round-trips every container of S101–S112 and S313–S353.

## Open

- **MIST22** — built-in names: 19 IDs are known (S353); a site named
  with another built-in name reads as unnamed, with its key.
- **MIST23** — the parcel counters: the site record's last values and
  the container's counter pairs and lists, with one and two parcels
  only (S112, S326). A sample with the Numbering counters set to
  distinct values would settle it.
- S328 changed only the name: the description `Test site` and the 3D
  Geometry / Numbering values in the screenshots are not in S328–S331
  (description empty, block unchanged).
- The 760-bit block of the site record apart from key, stamp, 0.1 and
  5.0; site display mode and construction layers.
- The loop selector k; the second (null) reference of each member and
  face pair; face and vertex flag bits (MIST12).
- Parcel records beyond their tail and references (their graphics,
  area, label, number, name template).

- Crossing writes: tees, end-to-end joins and arcs written by
  civil3drust (only an X of two straight alignments is proved, S362);
  the loop's node rule.
- Feature lines into and out of a site; closed regions and parcels; spirals and arc crossing tests in moves;
  deleting a site with its members (FRT-4).

## Reader gaps found — fixed at round close

- `aecc_site::site_handles` took the number of sites from the list's
  strings, so sites with built-in names were missed (S321–S327 reported
  no site). The count and keys now come from `BL n` and the key list.
- `aecc_topology::graph` read an edge header `next, count, 0, 1`; with
  a freed entry the parse failed (S330, S105). It now reads entries
  (`BL 2` freed, `BL 0, ID, edge`) and reports each edge's ID; faces and
  the tail are read by `aecc_site_edit::Topology`.
