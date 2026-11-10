# Modularity review and proposal

Status: proposal (2026-11-04), deferred (2026-11-05, D-066) until more
Civil 3D objects are sampled. Reviewed at commit `754afb4`.
Nothing here changes behaviour; every step below is meant to keep
`examples/fingerprint.rs` output byte-identical across all samples unless
the step says otherwise.

**Goal.** Someone can take one unknown or unsupported piece — a MIST
field, a spiral type, an element kind, a whole new AECC class — and
replace its current pass behaviour (raw passthrough, zeroing, fallback,
refusal) with a real implementation, touching only that piece and its
tests. Their work cannot change the bytes civil3drust writes for anything
else.

---

## 1. Weak spots

Ordered by how much they block that goal.

### W1. One dependency knot covers most of the crate

16 of the 28 modules form a single dependency cycle: `raw`,
`string_stream`, `handle_refs`, `proxy_graphics`, `aecc_entity`,
`aecc_alignment`, `aecc_elements`, `aecc_layout`, `aecc_layout_solve`,
`aecc_build`, `aecc_encode`, `aecc_arc_edit`, `aecc_element_edit`,
`aecc_criteria`, `aecc_design_speed`, `aecc_station`. Any of them can
reach any other. Nothing in the knot can be tested, replaced or reasoned
about alone.

The edges that close it are small:

- Low-level primitives live in a high-level module. `aecc_build` (the
  record builder) owns the bit writer `Bits`, `read_bll` and `guid_v4`;
  `aecc_entity`, `aecc_station`, `aecc_criteria` and
  `aecc_design_speed` import the builder just for those.
- The foundation knows about alignments. `raw` imports `aecc_entity`
  and `handle_refs` and holds alignment-specific readers
  (`section2_form`, `layout_json_flag`); `string_stream` imports
  `raw::is_layout_json`.
- `aecc_alignment` ⇄ `aecc_elements`: the tangent decoder lives in
  `aecc_alignment`, the other element kinds in `aecc_elements`, and each
  calls the other.
- `aecc_elements` ⇄ `aecc_layout` and `aecc_layout_solve`: element
  records import the JSON layout and the solver, which import element
  records back.

### W2. Every module finds its own way into the record

There is no single map of an alignment record. Each feature locates its
part with its own scan or offset constant (`STATION_GAP_BITS`,
`ELEMENT_HEADER`, `geometry_key_bit`, `resave_field_bit`,
`start_station`, `section2_form` …): 9 files do this. The record is
decoded again from scratch at about 66 call sites in 11 files.

Effect: each discovery that shifts a boundary (a new optional field like
MIST2, a new form like the Properties header) has to be taught to every
locator separately. Two features can disagree about where a section
starts, and nothing checks that they agree.

### W3. Element records are found by trial, and unknown ones vanish

`element_records` walks the element area bit by bit and tries six
recognisers in a fixed order (tangent, group, free spiral, spiral, arc,
line+spiral). If none matches, it moves on one bit. So:

- an element kind civil3drust does not know is **silently skipped**, not
  kept; the chain then has a hole the caller may not notice;
- the order is hidden policy — a new recogniser placed early can steal a
  match from an existing kind, placed late can be shadowed;
- the `Element` enum has no `Unknown` / `Raw` case to carry a skipped
  record through to the writer.

### W4. Closed enums matched everywhere, with catch-alls

`Element` is matched at 141 sites across 9 files (`aecc_elements` 47,
`aecc_element_edit` 36, `aecc_build` 19, `aecc_encode` 12 …). Adding a
kind means visiting all of them. Many matches end in `_ =>`, so the
compiler does not point out the sites that need a decision — the new
kind quietly gets whatever the catch-all does.

Code enums turn unknown values into known ones: `SpiralType::from_code(…)
.unwrap_or(Clothoid)` makes any future code a clothoid and loses the
number. The same risk applies wherever a decoded code or count is defaulted with `unwrap_or(0)` / `unwrap_or_default()` (14 sites in `src/`); each needs checking.

### W5. Fallback and refusal policy lives in the callers

- The "no shape known → draw a clothoid" rule is in
  `SpiralGeometry::point_at` (`aecc_elements`), not in `spiral_type`.
- "Spirals are not editable yet" and similar refusals are written out
  separately in 9 files (`aecc_element_edit`, `aecc_criteria`,
  `aecc_design_speed`, `aecc_alignment_edit`, `aecc_registry` …), 18
  places in all. Lifting one restriction means finding all copies; a
  missed copy keeps refusing, a removed one may let unsupported data
  through elsewhere.

### W6. Two write strategies, chosen per feature

Some features patch bytes in place at the same size
(`aecc_alignment_edit`, `aecc_arc_edit`, `set_alignment_type`,
`set_pi_display`); others re-encode records (`aecc_build`,
`aecc_encode`, `aecc_create`). Cache handling is applied by hand at each
call site (`drop_caches`, `clear_geometry_key`, `to_plain_form`,
`remove_resave_field`). There is no one place that says "these sections
are written from the model, these are copied from the input, these
caches are zeroed / dropped / rebuilt".

The model / derived / binding / raw tiers proposed for
`docs/object-model.md` (that file was drafted but is not in the repo)
exist only as an idea; no type carries them.

### W7. Grab-bag modules

- `aecc_elements` (3,912 lines) mixes DWG record decoding, plane
  geometry (arcs through three points, fillets, spiral placement),
  solver glue, chain resolution and curve constructors.
- `aecc_build` (1,151 lines, 41 public items) mixes the bit writer,
  GUIDs, PI planning, geometry replacement, proxy graphics, identity,
  alignment type, cache removal and PI display.

A contributor working on spiral geometry has to work inside the record
decoder; one working on graphics has to work inside the builder.

### W8. Two alignment models

`alignment::Alignment` (tangent-only, PIs) is the type re-exported at the
crate root and described in `lib.rs` as *the* model; the real model is
`DecodedElements` / `ResolvedElement`. `lib.rs` still says "tangent-only,
no curves, no spirals" and "read-only". New contributors will build on
the wrong one.

### W9. Errors cannot say "unsupported"

14 error types; 5 are `String` newtypes (`CriteriaError`, `DeleteError`,
`SpeedError`, `ElementEditError`, `LayoutError`). "Not handled yet",
"malformed record" and "not found" all look the same to a caller. A
contributor cannot search for the one refusal their feature lifts, and
tests cannot assert that an unsupported case is refused for the right
reason.

### W10. Classes are dispatched by string literal

`"AECC_ALIGNMENT"` appears 17 times in 11 files. There is no per-class
entry point. The next class (profile, surface, pipe network) will have no
common shape to follow, and each will grow its own scattered literals.

### W11. Everything is public, and the apps use the internals

`hexview`, `viewer`, `examples` and `tools` import 23 different
internal modules, including `aecc_build` internals. Any restructuring
breaks them; conversely, nothing marks what a library user should rely
on. Research helpers (`find_f64s`, `utf16_strings`, bit pokes) and the
13 `exp_*` examples sit beside the product API.

---

## 2. Target shape

Layers, each depending only on the ones above it.

| layer | holds | today's sources |
|---|---|---|
| **bits** | bit reader/writer, BL/BD/RD/BLL, handle bits, GUIDs, UTF-16 | parts of `raw`, `aecc_build` (`Bits`, `read_bll`, `guid_v4`), `string_stream` primitives |
| **records** | acadrust bridge: open, `restore_raw_records`, records by class, write a record back | rest of `raw`, `handle_refs`, `inventory` |
| **record map** (per class) | one parse of a record into named spans, each tagged *model*, *derived*, *binding* or *raw*, with a MIST id where one applies | the scattered locators (W2) |
| **codecs** (per span kind) | decode a span to a value or `Unknown(span)`; encode a value back | element decoders, criteria, speeds, station, strings, style refs, graphics |
| **geometry** (format-free) | tangents, arcs, spirals and their shapes, groups, solver, stationing | `alignment`, geometry half of `aecc_elements`, `aecc_layout_solve`, `spiral_type` |
| **operations** | create / edit / delete as changes to the model | `aecc_*_edit`, `aecc_create`, `aecc_delete`, `aecc_topology` |
| **writer** | model spans encoded, raw spans copied, derived spans handled by the cache policy | `aecc_build`, `aecc_encode`, the hand-applied cache calls |
| **facade** | the small public API apps and users rely on | `lib.rs` |
| **research** (feature or separate crate) | bit search, dumps, hexview support, `exp_*` | `raw` helpers, `dump`, examples |

Four cross-cutting tables replace scattered decisions:

1. **Element kind table.** Each kind declares its header/type byte and
   its codec. Dispatch is by the header, not by trial; a header nobody
   claims becomes `Element::Unknown(raw span)` and is written back as
   read. The current trial scanner moves to the research layer (it is
   still the right tool for finding new kinds).
2. **Capability table.** One list of what is supported for read, edit
   and create, per element kind / spiral type / section. Refusals come
   from it as `Unsupported(feature)`. Lifting a restriction is one line
   plus tests.
3. **Cache policy table.** One row per derived span (MIST1 geometry key,
   MIST2, MIST3 date stamp, speed intervals, proxy graphics …): *zero*,
   *drop*, *rebuild* or *keep*, with the decision record that set it.
   Per D-049 the default is *zero* or *drop*; *keep* only where a
   rejection test proves Civil 3D needs it. `docs/mysteries.md` already
   is this table in prose; the code would mirror it row for row.
4. **Class contract.** Each AECC class provides the same five things:
   its record map, codecs, references (handles it points to and is
   pointed from), cache rows, and capabilities. Alignment is the only
   implementer for now; the contract exists so profiles start in the
   right shape.

### What a drop-in looks like after this

| piece someone replaces | they touch | everything else sees |
|---|---|---|
| NSW spiral shape (FRT-1) | its arm in the spiral shape table | the same `local_point` contract; fallback stays inside the spiral module |
| a MIST field, e.g. MIST16 | its row in the record map: *raw* → *model*, plus a codec | the span's bytes, unchanged until they choose to encode them |
| a new element kind | one row in the element kind table and its codec | `Unknown(span)` before, the decoded kind after; exhaustive matches list every site to decide |
| a cache rule | one row in the cache policy table | nothing else |
| a new class (profile) | a new module implementing the class contract | facade gains one accessor |

---

## 3. Rules that keep it modular

- **No catch-alls on domain enums.** Matches on `Element`, `SpiralType`
  and other code enums are exhaustive, so a new case is a compile error
  at every site that must decide.
- **Every code enum has `Unknown(raw value)`**, and unknown values are
  written back unchanged.
- **Fallbacks live in the module that owns the unknown**, never in the
  caller.
- **Unknown bytes are never skipped, only carried.** A span civil3drust
  cannot read stays a raw span and is copied to the output.
- **One parse per record per operation.** Features receive the record
  map; they do not search for their own offsets.
- **Each slot has a pinned fixture**: today's output for the samples
  that contain it, so a replacement cannot change anything outside its
  slot without a failing test.
- **Errors carry a kind**: *unsupported*, *not found*, *malformed (at
  bit)*, *invariant broken*.

---

## 4. Migration order

Each step is independent and ends with fmt / test / clippy and an
unchanged fingerprint over every sample. Steps 1–7 change no output.

1. **Extract the bits layer.** Move `Bits`, `read_bll`, `guid_v4` and the
   `raw` bit helpers into one module with no crate imports. Breaks most
   of the knot (W1) at no risk.
2. **Clean `raw`.** Move `section2_form`, `layout_json_flag`,
   `is_layout_json` and the `aecc_entity` call out to the alignment side.
   `raw` then depends on nothing in the crate.
3. **One error type with kinds**, and the capability table; the 18
   refusals become lookups (W5, W9).
4. **Record map for `AECC_ALIGNMENT`.** Build it from
   `docs/alignment-map.md`; move consumers onto it one at a time,
   checking at each step that the map's spans equal the old locators'
   offsets on every sample (W2).
5. **Element kind table and `Element::Unknown`.** Dispatch by header;
   keep the trial scanner as a research tool and as a cross-check test
   (both must find the same records on every sample) (W3).
6. **Remove catch-alls; add `Unknown(code)` to code enums**; move the
   spiral fallback into `spiral_type` (W4, W5).
7. **Split the grab-bags.** `aecc_elements` → element codecs + geometry;
   fold `alignment.rs` into geometry; `aecc_build` → writer + graphics +
   identity. Fix `lib.rs` docs and root re-exports (W7, W8).
8. **One write path.** Writer = encode model spans, copy raw spans, apply
   the cache table. Retire same-size patchers one feature at a time;
   each retirement is checked by fingerprint and, where bytes change,
   by an AUDIT round-trip in Civil 3D (W6). *This is the only step that
   may change output, and only where a patcher and the encoder already
   disagree.*
9. **Facade and research split.** Public API: open, alignments, the
   model, operations, save. Bit tools, dumps and `exp_*` go behind a
   `research` feature (or a `civil3drust-research` crate) (W11).
   `hexview/` and `viewer/` are archived (`archive/`, D-075) and no
   longer need to compile against new module paths; `workbench/` does
   and moves with the library.
10. **Class contract**, written down with alignment as its only
    implementer, before starting profiles (W10). Not earlier — one
    implementer is enough to fix the shape without guessing at classes
    not sampled yet.

**Not proposed.** A mirror of Civil 3D's own class layering (D-049: keep
the code minimal); the layers above are civil3drust's own, and the
record map names spans by what they mean to us, not by Civil 3D class.
Also not proposed: a plugin system or runtime registry. Tables compiled
into the crate give the same isolation with less machinery, and the
cleanroom rules (one finding, one sample, one decision) already make
contributions small and reviewable.
