# acadrust 0.6.7: AECC records and the upgrade check

## Finding

acadrust 0.6.7 adds support for AutoCAD Architecture classes. Its DWG reader
decides which records are Architecture records with a DXF-name prefix test:
any name starting with `AEC`, `AAEC` or `ABST_`
(`acadrust::classes::arch::is_architecture_name`). Every Civil 3D class
(`AECC_*`) matches, so 0.6.7 reads them as:

- objects: `ObjectType::Arch(ArchObject)` instead of `ObjectType::Unknown`;
- entities: `EntityType::Extended(..)` with `ExtendedEntityData::Arch`
  instead of `EntityType::Unknown`.

The record bytes are kept (`raw_dwg_data`, handle-stream size, source
version), but two things change for civil3drust:

1. `raw::records` only looked at `Unknown` records, so it found nothing. On
   S11, `examples/alignments` reported 0 `AECC_ALIGNMENT` records under
   0.6.7 (1 under 0.6.5), with no error.
2. On save, 0.6.7 does **not** write Architecture records verbatim. Objects
   are re-encoded (common header from the struct's handle, owner, reactors
   and extension dictionary, then the parsed payload); entities are
   explicitly excluded from raw passthrough and re-encoded. `Unknown`
   records are still written byte for byte when the DWG version matches.

Source: acadrust 0.6.7 `classes/arch/mod.rs`, `io/dwg/dwg_document_builder.rs`
and `io/dwg/dwg_stream_writers/object_writer/{objects,entities,native_architecture}.rs`.

## Change

`raw::open` now calls `raw::restore_raw_records`, which turns every
Architecture-typed record that has DWG bytes back into the `Unknown` record
acadrust builds for a non-Architecture class (same fields, same bytes). The
rest of civil3drust, and its save path, are then the same as under 0.6.5
(D-033). `tests/acadrust_compat.rs` checks that no Architecture records
remain after `open`, that AECC records are found, and that the bytes are
unchanged by the conversion (S24).

## Check against 0.6.5

Steps: `examples/fingerprint` lists every raw record (handle, class,
handle-stream bits, size, hash of the bytes) after reading a drawing and
again after saving it with acadrust and reading it back. It was run on all
91 drawings in `samples/` with a 0.6.5 build and with the 0.6.7 build, and
the outputs were diffed. `examples/alignments` was compared the same way.

| Check | Result |
|---|---|
| Raw records after read, all 91 samples | identical |
| Raw records after save + read back, all 91 samples | identical |
| `alignments` output, all 91 samples | identical (apart from the failsafe note below) |
| `set_length S11 … 8C5C 3 200` output | identical records; same length change verified |
| `create_alignment S25 S24 …` output | identical except the random ID string (registry, companion) and the alignment's two identity-GUID copies (two 128-bit runs); two 0.6.5 runs differ in the same places |
| `cargo test` | same results as under 0.6.5 |

## Side effect: the 3DSOLID is kept

Under 0.6.5, seven samples (S1, S5, S6, S11, S12, S15, S89) needed failsafe
mode because of the `3DSOLID` in `PRESSUREPARTERRORBLOCK` (handle 7F72), and
the solid was dropped on save. 0.6.7 reads all seven normally: 3 `3DSOLID`
entities instead of 2, still 3 after a save and read back.

Confidence: the AECC bytes are shown to be unchanged by the tools above. No
0.6.7-written drawing has been opened in Civil 3D yet; the non-AECC parts of
a saved drawing come from acadrust's 0.6.7 writer, and a drawing that keeps
the solid is new territory (not yet checked in Civil 3D).

## Opened in Civil 3D

S125 (S24 read and written by civil3drust on 0.6.7 with no change) opened
in Civil 3D 2024 without RECOVER, and its re-save is S131 (owner,
MIST7 control).
