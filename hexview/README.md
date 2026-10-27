# civil3drust hexview

A classic three-column hex view of a DWG — offset, hex bytes, decoded text —
where every object record is outlined and labelled by the library that
decodes it.

```text
cargo run --release -- [file.dwg] [--goto <hex offset|#handle>]
cargo run --release -- --dump file.dwg      # text listing, no window
```

## Colors

| Color | Meaning |
|---|---|
| blue | acadrust decodes the record (entity, object, table entry) |
| red | AECC record, read by civil3drust |
| amber | another class acadrust keeps as opaque bytes |
| gray | in the handle map but not in acadrust's document (BLOCK/ENDBLK, records dropped in failsafe mode) |

In the right column a record shows as `CLASS : name` (or `CLASS : #HANDLE`)
followed by `·` fill for the rest of its bytes. Bytes outside records show
as ASCII, or `.` when not printable. The size fields and CRC of each record
are tinted darker than its data.

Clicking a record opens a detail window: frame offsets, handle-stream size,
CRC check, everything civil3drust decodes (alignment elements, strings,
proxy graphics, handle references), and acadrust's full `Debug` view.
`bit N` buttons jump to the byte holding that bit; `open` follows a handle
reference. The bottom bar reads the bytes at the cursor as u8…i64/f32/f64
(little-endian) and, inside a record, gives the data byte and bit offset —
the same offsets civil3drust uses.

## Which bytes are mapped

- **R2004 and later**: sections are compressed in pages, so records are
  mapped in the decompressed `AcDb:AcDbObjects` pane, not in the raw file.
  Offsets in that pane are the ones acadrust's handle map uses.
- **R2000 (AC1015)**: the objects section is stored as is, so the raw-file
  pane is mapped too.
- Every other section (Header, Classes, Handles, AppInfo, …) is available
  decompressed in its own pane, without overlay.
- DXF and other files open in the raw pane without overlay.

DWG data is a bit stream, so field values inside a record are rarely
byte-aligned; use the detail window's bit offsets rather than the ASCII
column to find them.

## Version pin

The crate is pinned (via `Cargo.lock`) to the same acadrust version as
civil3drust. acadrust 0.6.7 decodes `AEC_*` objects into its own types, so
a newer acadrust would change which records show blue versus amber.
