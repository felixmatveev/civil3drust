# C3DDUMP — Civil 3D API dump

A read-only Civil 3D add-in with two commands (`C3DUMP` and
`C3DUMPCONFIG` are accepted as well).

**`C3DDUMP`** writes the objects of the configured classes to
`<drawing>.api.txt` next to the drawing (`%TEMP%` for an unsaved one),
as the drawing is now, saved or not:

1. Alignments (if their class is in the filter) in the detailed form:
   `AlignmentType`, `CreationMode`, `FingerPrint`, design speeds,
   entities and their constraints, …, each followed by its profiles
   (every simple property, then each PVI and each entity) and its
   profile views.
2. `== Objects`: every other matching object, in handle order, as
   `[handle] DXF-class API-type owner <handle>` and its simple
   properties; with **getters** on, for Civil 3D (`AECC_`) objects also
   the result of every public `Get…()` method, of `Get…(enum)` for each
   value of the enum, and of `Get…(int)` for each index below the
   object's largest `…Count` property (at most 200) — so a feature line
   shows `GetPoints(PIPoint)`, `GetPoints(ElevationPoint)`,
   `GetPoints(AllPoints)`, and a site its alignment and feature line IDs.

**Dump on save** (`C3DDUMPCONFIG` → `Auto`, D-080): while Auto is on,
every `QSAVE`, `SAVEAS` and `SAVE` of an open drawing also writes

```
<drawing folder>\c3dump\<drawing>.<yyyyMMdd-HHmmss>.api.txt
```

one file per save, so a drawing collects the history of what Civil 3D
reported after each step. The dump is made after the save has finished;
if it fails, a line on the command line says so and the save is not
affected. Autosave does not dump, and neither does closing a drawing
with "save changes" (save first). Auto is **off at the start of every
Civil 3D session** and is not saved, so production drawings get no dumps
unless it is switched on.

The workbench reads these files: `--changes a.dwg b.dwg` (records and
API values that changed), `--api-history file.dwg` (save by save),
`--api`, `--api-diff`, and the **API** tab (`docs/workbench.md`).

**`C3DDUMPCONFIG`** shows the configuration and changes it, one keyword
at a time (Enter when done):

| keyword | setting | default |
|---|---|---|
| `Auto` | dump after every save (on/off); never saved | off |
| `Classes` | `full`, `control`, `all`, or DXF name patterns, comma-separated, `*` wildcard (e.g. `AECC_PROFILE*,AECC_FEATURE_LINE`) | `full` |
| `Getters` | call the `Get…` methods on the Civil 3D objects (on/off) | on |
| `Displayreps` | leave out display representations, `AECC_DISP_REP_*` (on/off); not used with `full` | on |
| `Echo` | also print a manual dump on the command line (on/off) | off |
| `Reset` | back to the defaults | |

`full` is every object in the drawing: Civil 3D objects, display
representations, and plain AutoCAD objects (layers, dictionaries,
entities; simple properties only). It is the default, and the workbench
filters it afterwards. On a template-based drawing it is several
thousand objects: expect a few megabytes and some seconds per dump. If
that is too slow, or a property of some AutoCAD object upsets Civil 3D,
use one of the partial settings: `all` is every `AECC_` class;

`control` is the classes around alignments, profiles, feature lines and
sites: `AECC_ALIGNMENT`, `AECC_ALIGNMENT_*NODE`, `AECC_PROFILE`,
`AECC_PROFILE_VIEW`, `AECC_FEATURE_LINE`, `AECC_FEATURE_STYLE` (site
containers, companions, registry), `AECC_PARCEL` (sites),
`AECC_PARCEL_NODE`, `AECC_DB_*` and `AECC_ENTITY*` (profile helpers),
`AECC_NOTIFICATIONINFOS`, the design check sets, label groups and
labelings, `AECC_GRADING*`.

The configuration lasts for the Civil 3D session and is saved to
`%APPDATA%\civil3drust\c3ddump.cfg` (the filter file: `classes=…` and the
options), so the next session starts with it — except Auto. A file saved
by the add-in before `full` existed keeps its options, but its classes
are replaced by `full` once. The dump's header repeats the configuration
it used, what triggered it (`manual`, or `save (QSAVE)`) and the host
version.

Everything is read by reflection, so the add-in builds against any
Civil 3D version.

Cleanroom: it reads our own sample drawings through Civil 3D's public
API, so its output is observed Civil 3D behaviour (CLAUDE.md). Autodesk's
assemblies are referenced from the local install and never committed.

## Build (Civil 3D 2024, .NET Framework 4.8)

1. Install the .NET SDK (8 or later) — `dotnet --version` should work.
   Visual Studio is not needed.
2. Check that these exist (adjust `AcadDir` if Civil 3D is elsewhere):
   - `C:\Program Files\Autodesk\AutoCAD 2024\accoremgd.dll`, `acdbmgd.dll`, `acmgd.dll`
   - `C:\Program Files\Autodesk\AutoCAD 2024\ACA\AecBaseMgd.dll`
   - `C:\Program Files\Autodesk\AutoCAD 2024\C3D\AeccDbMgd.dll`
3. In this folder:

   ```
   dotnet build -c Release
   ```

   or, for another install folder:

   ```
   dotnet build -c Release -p:AcadDir="D:\Autodesk\AutoCAD 2024"
   ```

   The add-in is `bin\Release\net48\C3dApiDump.dll`.

For Civil 3D 2025 and later (.NET 8): in `C3dApiDump.csproj` set
`TargetFramework` to `net8.0-windows`, remove the
`Microsoft.NETFramework.ReferenceAssemblies` package, and point `AcadDir`
at that version's folder.

## Run

1. Open the drawing in Civil 3D.
2. `NETLOAD` → pick `C3dApiDump.dll` (answer **Load** if the security
   prompt appears).
3. `C3DDUMPCONFIG` once if the defaults don't fit, then `C3DDUMP`; or
   `C3DDUMPCONFIG` → `Auto` and save as usual.
4. Compare with the workbench and send the report
   (`civil3drust-workbench --changes before.dwg after.dwg --out report.txt`),
   or send `<drawing>.api.txt` itself (written next to the drawing, or
   to `%TEMP%` for an unsaved drawing).

After a rebuild, NETLOAD the new DLL in a fresh Civil 3D session (a
loaded .NET assembly cannot be replaced while Civil 3D runs).

## What to compare

- `FingerPrint` against MIST1 (the 64 bits after the 2 leading bits, read
  little-endian). Expected if MIST1 is the fingerprint:
  S24 F9E `0x8BCE21E638811EEE`; S161 8C49 `0xA6C003E98CBE5C35`,
  8C56 `0x88C883E98CBE5C25`.
- `CreationMode` against the companion's edge flag `0x20` (MIST12).
- `Constraint1`/`Constraint2` against our line kinds and arc `n2` codes
  (`docs/element-methods.md`, `docs/class-catalog.md`).
- Profiles: PVIs, entities (through points, curve length, K, BVC/EVC)
  against `aecc_profile::layout` (`docs/profile-object.md`; first run on
  S306, all values agree).
- Feature lines: `GetPoints(…)` per point type against
  `aecc_feature_line::vertices` (PIs, elevation points, z), and whatever
  curve methods the API lists against the bulges
  (`docs/feature-line-object.md`).
- The `== Objects` section: property names per class for the typed skeleton; handle
  values (`…Id` properties) against the records' handle streams.
