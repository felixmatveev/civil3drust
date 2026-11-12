# Sites — record, container, collection, parcels (S101–S112, S313–S331)

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
  high-bit key. Reading such a name needs the pool (MIST22).
- **Order.** By name, ascending, spaces ignored: `TEST2`, `TEST SITE`
  (S103); `TEST 2`, `TEST SITE` (S104); `TEST`, `TEST|1` (S317);
  unnamed, `B` (S329). (Not by key: S317 is ascending, S103 descending.)
  The strings follow the same order, unnamed sites skipped.
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

## Write experiments (S332–S340, `examples/exp_site.rs`)

**Results so far**

| sample | Civil 3D 2024 (owner) |
|---|---|
| S332 move-in | **accepted**: opens with no RECOVER, AUDIT clean; Prospector lists the alignment under the site; API: `IsSiteless: False`, `SiteName: A`, the site's `GetAlignmentIds()` = 8D6A, the container's `Name` = `A`. So Civil 3D takes the container without the site's redrawn graphics, without the optional site entry, with vertex flag 50 where it writes 2, and with node coordinates from the alignment. **Re-save S342** (site renamed `Site A`, whole drawing rewritten): the alignment is still in the site and its record equals the one in Civil 3D's own S323; Civil 3D rebuilt the container from the alignment (new node IDs 4–6 with its own coordinate rounding, edge flags 0x30, vertex flags 50/2/10, face flags 7, 19 style references), redrew the site's graphics (bit for bit those of S323) and set the site point from its nodes; it did not add the optional entry |
| S333 move-out | **accepted**: clean open, no AUDIT issues; API: `IsSiteless: True`, `SiteName` empty, the site's `GetAlignmentIds()` empty. **Re-save S341** (the owner also renamed the site `Site A`; Civil 3D rewrote the whole drawing): the refilled companion 37FD stays the alignment's companion, same nodes, with edge flags 0x10 → 0x810 and vertex flags 10 → 2; the site's container is written compact — no edge slot (next ID still 2), **no face 1** (next ID 2), 19 style references; the emptied companion 8D69 likewise has no slot. So refilling an emptied companion is a valid way to make an alignment siteless, and a container's free slots and loop-less face are optional |
| S334 move-out, one of two | **accepted**: clean; API: `Alignment - (1)` siteless, `Alignment - (2)` in the site, the site's `GetAlignmentIds()` = 8D8C. **Re-save S343**: same model; the refilled companion left exactly as written (edge 0x10, vertex flags 10 — S341 changed them, so that is not a rule); the container rewritten with a single edge entry that keeps **ID 2** |
| S335 move-in again | **AUDIT: 1 error, fixed.** The alignment's record holds the ID of its edge (see "The alignment's edge ID"); S335 left it at 1 while the new edge was ID 3. Civil 3D's fix (S344): the alignment added to the container a second time as edge 4 on the same nodes, the written edge 3 left behind, the field set to 4. S332–S334 passed only because every edge there was ID 1 |
| S345 = S335 with the edge ID field set to 3 | pending |
| S336–S340 | pending |


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
| S339 | `create` S322: `NEW SITE` | clone of an empty, unnamed site: handles (site, site + 1), key, name, empty tail and container, list entry in name order, last-created pointer |
| S340 | `create` S331: `A SITE` | clone of a site with members: tail reset, **graphics stripped**, the template's date stamp kept |

For each: does it open without RECOVER, is AUDIT clean, what does
Prospector show (sites, the alignment's site), and a re-save. The
re-saves show what Civil 3D rebuilds (graphics, keys, vertex flags, the
entry) and go into `civil3d-rebuilds.md` / `civil3d-rejects.md`.

Limits of the experiment code: an alignment that crosses or touches a
member is refused (straight members only once the site has members);
spirals are refused; `delete` takes empty sites only; `create` needs a
site without parcels to clone.

## Open

- **MIST22** — the name pool: where a key without the high bit is
  resolved to its text (`AECC_CLASS_NODE` BDA holds the key of `A`).
  Until then a site with such a name reads as unnamed.
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
- Crossing writes (planar arrangement, parcel creation) — stretch goal.

## Reader gaps found (to fix when the round closes)

- `aecc_site::site_handles` takes the number of sites from the list's
  strings, so unnamed sites are missed: S321–S327 report no site and
  S329–S330 report the wrong one. The count is the `BL n` before the
  keys; the name has to come from the site record's string count.
- `aecc_topology::graph` reads an edge header `next, count, 0, 1`; with
  a free slot the list reads `next, slots, 2, …` (S330, S105) and the
  parse fails or stops early (S329). Faces and the tail are not read.
