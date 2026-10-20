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
