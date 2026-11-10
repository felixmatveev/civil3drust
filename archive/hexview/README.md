# civil3drust hexview

A classic three-column hex view of a DWG — offset, hex bytes, decoded text —
where every object record is outlined and labelled by the library that
decodes it.

```text
cargo run --release -- [file.dwg] [--goto <hex offset|#handle>] [--compare other.dwg]
cargo run --release -- --dump file.dwg      # text listing, no window
cargo run --release -- --diff a.dwg b.dwg [--show civil,opaque,unmapped,acad|all] [--filter text]
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

## Comparing two drawings

**Compare with…** (or `--compare other.dwg`) loads a second drawing B and
opens the Comparison window. Records of the two objects sections are
matched by handle and listed when they differ: `~` changed, `−A` only in
the open drawing, `+B` only in the other. Clicking one shows it in the hex
view (which shows drawing A).

The checkboxes choose which kinds are listed (acadrust, civil3drust,
opaque, unmapped — civil3drust, opaque and unmapped by default, since a
Civil 3D save also touches many acadrust records); the text field filters
by class, name or `#handle`. **📋 Copy** copies the report shown below
the list — a plain-text description meant to be pasted into a
conversation. `--diff` prints the same report in a terminal.

For each changed record the report gives the sizes, then the **differing
regions of the data stream** as bit ranges in A and B with their bits, and
the decoded field the region follows (`after String 3 at bit 3308`). The
regions come from a bit diff: common prefix and suffix, then the longest
common run of at least 64 bits splits the rest, recursively — so a field
that grows shows as an insertion and the bits after it line up again. The
handle stream is compared separately. Last come the decoded fields whose
values changed (fields that only moved are counted, not listed).

S58 → S59 (Centerline → Miscellaneous), `--filter "#1089"`:

```text
   data stream: A 3511 B 3819 bits (+308); 5 differing region(s)
   [1] A bits 1420..1423  B bits 1420..1499  (3 bits replaced by 79), after Proxy graphics at bit 56
   [2] A bits 1534..1534  B bits 1610..1752  (142 bits inserted), after Proxy graphics at bit 56
   [3] A bits 2974..2977  B bits 3192..3195  (2 of 3 bits differ), after Element 0 at bit 1738
   [4] A bits 3314..3314  B bits 3532..3622  (90 bits inserted), after String 3 at bit 3308
   [5] A bits 3494..3510  B bits 3802..3818  (7 of 16 bits differ), after String 11 at bit 3492
```

Region 3 is the alignment type (`001` → `100`: BL 1 → 4).

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

## acadrust version

hexview uses the same acadrust as civil3drust (0.6.7) and opens drawings
with `civil3drust::raw::open`, which restores the records acadrust reads
into its Architecture types (all `AEC*` classes, D-033) to raw records. So
`AECC_*` records show red and `AEC_*` records amber, as under 0.6.5.
