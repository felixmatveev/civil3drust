# Automated sample extraction (plan, not built)

**Status:** idea, parked (2026-11-12). Samples stay hand-made while the
broad skeleton is mapped: which objects exist, how they link, where each
sits in the file. Automation pays off later, on mystery blocks
(`docs/mysteries.md`) and field-level fine tuning, once the block to
chase is known. Tracked as FRT-5 in `FUTURE.md`.

## The idea

Let a script drive Civil 3D through its public .NET API: open a
baseline, change one thing, save a variant, dump it, diff it. Each run
is a sample made exactly the same way, so the diff shows only what that
one change moved.

```
baseline.dwg ──► script: one change ──► Save As S<n>_<name>.dwg
                                              │
             C3DDUMP Auto (dump on save) ◄────┤
                                              ▼
                 workbench --changes baseline variant --noise …
                                              ▼
                       "change K moved bits N..M of block X"
```

## Two existing Civil 3D MCP servers (reviewed 2026-11-12)

Both are MIT, both have a Node MCP server talking JSON-RPC over loopback
TCP (port 8080) to a C# add-in loaded with `NETLOAD`. Neither is a
source of format findings; they are tooling only, and anything they
report is observed Civil 3D behaviour like a C3DDUMP value.

| repo | commit | what it is | use here |
|---|---|---|---|
| [barbosaihan/civil3d-mcp](https://github.com/barbosaihan/civil3d-mcp) | `344ad15` (2026-05-15) | No fixed tools: `civil3d_execute` / `civil3d_query` compile any C# with Roslyn inside Civil 3D (globals `Document`, `CivilDoc`, `Database`, `Transaction`). About 740 lines of C#. | **The useful one.** Limited only by the Civil 3D API itself. |
| [Sacred-G/Civil3D-mcp](https://github.com/Sacred-G/Civil3D-mcp) | `9e437a3` (2026-08-16) | 219 fixed tools (grading, hydrology, plan production, QC, …), no raw code execution. | Design work, not research. Reference C# for awkward API calls (profiles, corridors, parcels). |

## Planned shape

- **Already in place (D-080):** C3DDUMP's `Auto` mode dumps every
  save to `c3dump\<drawing>.<timestamp>.api.txt`, and the workbench
  joins record bits with API changes (`--changes`, `--api-diff`,
  `--api-history`) and drops save noise (`--make-noise`, `--noise`).
  What is missing is only the *driving*: making the change and the
  save without the owner.
- **One add-in, not two.** Move the Roslyn executor and loopback
  listener into `tools/c3d_api_dump/`, so the same add-in changes,
  saves and dumps.
- **A save method outside the transaction.** barbosaihan commits the
  transaction only after the script returns, so a `SaveAs` inside the
  script saves the drawing *without* the change. Saving needs its own
  RPC call after the commit (about 20 lines).
- **A runner on the owner's PC** (Python): a list of variants
  (baseline, change script, sample name) → run, save, dump, diff →
  sample rows for `samples/README.md` with the script as "how made".
- **Side databases** (`new Database(false, true)`, `ReadDwgFile`,
  `CivilDocument.GetCivilDocument(db)`) could make many variants
  without opening each drawing. Some objects may only rebuild with the
  document open, so compare one side-database save with a normal save
  before trusting it; the difference itself may show which data are
  caches (`docs/civil3d-rebuilds.md`).

## Methods for mystery blocks

1. **Noise mask first.** Save the baseline twice with no edit and diff:
   timestamps, handseed, thumbnail and similar change on every save.
   Mask them out of every later diff (`--make-noise` / `--noise`,
   D-080).
2. **Marker values.** Set a coordinate or length to a distinctive,
   exactly representable double (`1234.5`, `-777.25`) and search for
   its 8 bytes. Finds a field faster than a small nudge.
3. **Small nudges.** Then move by small exact steps (0.5, 0.25) to see
   what Civil 3D recomputes: cached lengths, extents, fingerprints.
4. **Deletion cascades.** List handles through the API (or hook
   `ObjectErased`) before and after each delete. Gives the dependency
   graph as ground truth (S331: deleting a site deleted its feature
   line).
5. **API values as an answer key.** Export what Civil 3D reports
   (entity lengths, PI coordinates, start station, style names) per
   sample; tests decode the DWG and compare.
6. **Write acceptance.** Open a civil3drust-written drawing in Civil 3D,
   read the values back, AUDIT, re-save, and diff: the D-049 check
   without manual steps.

**MIST1 in particular.** The 66-bit field is probably the API's
`Entity.FingerPrint`, which C3DDUMP already reads. A runner can sweep
one geometry parameter at a time (length, position, rotation, radius)
and log fingerprint vs. geometry for hundreds of variants, a data set
for working out the algorithm from behaviour alone.

## Cautions

- The barbosaihan "sandbox" is a regex blocklist, not a sandbox. The
  listener is loopback-only but has no authentication: any local
  process can run code in Civil 3D. Load it only for test runs.
- Some data the API cannot set (style internals, internal caches).
  Those variants stay manual.
- Cleanroom: drive Civil 3D only through its public API and record
  observed results; no decompiling, no SDK headers (README).
