# C3DDUMP — Civil 3D API dump

A read-only Civil 3D add-in. The `C3DDUMP` command writes the public API
values of every alignment in the current drawing (`AlignmentType`,
`CreationMode`, `FingerPrint`, design speeds, entities and their
constraints, …) to `<drawing>.api.txt` next to the drawing, and prints
them on the command line.

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
3. `C3DDUMP`.
4. Send `<drawing>.api.txt` (written next to the drawing, or to `%TEMP%`
   for an unsaved drawing).

## What to compare

- `FingerPrint` against MIST1 (the 64 bits after the 2 leading bits, read
  little-endian). Expected if MIST1 is the fingerprint:
  S24 F9E `0x8BCE21E638811EEE`; S161 8C49 `0xA6C003E98CBE5C35`,
  8C56 `0x88C883E98CBE5C25`.
- `CreationMode` against the companion's edge flag `0x20` (MIST12).
- `Constraint1`/`Constraint2` against our line kinds and arc `n2` codes
  (`docs/element-methods.md`, `docs/class-catalog.md`).
