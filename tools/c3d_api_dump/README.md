# C3DDUMP — Civil 3D API dump

A read-only Civil 3D add-in with two commands.

**`C3DDUMP`** writes to `<drawing>.api.txt` next to the drawing (and
prints it on the command line):

1. Every alignment (`AlignmentType`, `CreationMode`, `FingerPrint`,
   design speeds, entities and their constraints, …), each followed by
   its profiles (every simple property, then each PVI and each entity)
   and its profile views (every simple property).
2. `== Sites`: every site.
3. `== Feature lines`: every feature line.
4. `== Other Civil 3D entities`: every other `AECC_*` entity in the
   drawing (label groups, parcels, …).

Sections 2–4 give every simple property and the result of every public
`Get…()` method without parameters, and of `Get…(enum)` for each value of
the enum — so a feature line shows `GetPoints(AllPoints)`,
`GetPoints(PIPoints)`, `GetPoints(ElevationPoints)` and whatever else
this Civil 3D version offers, and a site its alignment and feature line
IDs. Everything is read by reflection, so the add-in builds against any
Civil 3D version.

**`C3DDUMPALL`** walks every handle in the database and writes every
Civil 3D object (`AECC_*` class: entities, styles, sites, profile helper
objects, label groups, settings …) with its owner and simple properties
to `<drawing>.api-all.txt`. Not printed (it is long). Use it to see a
class's public property names and values next to its record — the
skeleton of types (docs/open-blocks.md).

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
3. `C3DDUMP` (and/or `C3DDUMPALL`).
4. Send `<drawing>.api.txt` / `<drawing>.api-all.txt` (written next to
   the drawing, or to `%TEMP%` for an unsaved drawing).

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
- `C3DDUMPALL`: property names per class for the typed skeleton; handle
  values (`…Id` properties) against the records' handle streams.
