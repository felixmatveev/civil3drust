// C3DDUMP: write the public API values of every alignment in the current
// drawing to <drawing>.api.txt, to compare with what civil3drust decodes.
// Read-only: it opens objects for read and changes nothing.

using System;
using System.IO;
using System.Text;
using Autodesk.AutoCAD.ApplicationServices;
using Autodesk.AutoCAD.DatabaseServices;
using Autodesk.AutoCAD.EditorInput;
using Autodesk.AutoCAD.Runtime;
using Autodesk.Civil.DatabaseServices;
using AcApp = Autodesk.AutoCAD.ApplicationServices.Application;

[assembly: CommandClass(typeof(C3dApiDump.Commands))]

namespace C3dApiDump
{
    public class Commands
    {
        [CommandMethod("C3DDUMP")]
        public void Dump()
        {
            Document doc = AcApp.DocumentManager.MdiActiveDocument;
            Editor ed = doc.Editor;
            var sb = new StringBuilder();
            sb.AppendLine("drawing: " + doc.Name);
            sb.AppendLine("written: " + DateTime.UtcNow.ToString("u"));

            var filter = new SelectionFilter(new[]
            {
                new TypedValue((int)DxfCode.Start, "AECC_ALIGNMENT"),
            });
            PromptSelectionResult sel = ed.SelectAll(filter);
            if (sel.Status != PromptStatus.OK)
            {
                ed.WriteMessage("\nC3DDUMP: no alignments found.\n");
                return;
            }

            using (Transaction tr = doc.Database.TransactionManager.StartTransaction())
            {
                foreach (ObjectId id in sel.Value.GetObjectIds())
                {
                    if (!(tr.GetObject(id, OpenMode.ForRead) is Alignment al))
                    {
                        continue;
                    }
                    DumpAlignment(sb, al);
                }
                tr.Commit();
            }

            string folder = Path.GetDirectoryName(doc.Database.Filename);
            if (string.IsNullOrEmpty(folder))
            {
                folder = Path.GetTempPath();
            }
            string path = Path.Combine(
                folder,
                Path.GetFileNameWithoutExtension(doc.Name) + ".api.txt");
            File.WriteAllText(path, sb.ToString());
            ed.WriteMessage("\n" + sb + "\nC3DDUMP: written to " + path + "\n");
        }

        static void DumpAlignment(StringBuilder sb, Alignment al)
        {
            sb.AppendLine();
            sb.AppendLine("[" + al.Handle + "] " + Get(() => al.Name));
            Line(sb, "AlignmentType", () => al.AlignmentType);
            Line(sb, "CreationMode", () => al.CreationMode);
            Line(sb, "FingerPrint", () => "0x" + al.FingerPrint.ToString("X16") + " (" + al.FingerPrint + ")");
            Line(sb, "Description", () => al.Description);
            Line(sb, "StyleName", () => al.StyleName);
            Line(sb, "IsSiteless", () => al.IsSiteless);
            Line(sb, "SiteName", () => al.SiteName);
            Line(sb, "StartingStation", () => al.StartingStation);
            Line(sb, "EndingStation", () => al.EndingStation);
            Line(sb, "ReferencePointStation", () => al.ReferencePointStation);
            Line(sb, "Length", () => al.Length);
            Line(sb, "UseDesignSpeed", () => al.UseDesignSpeed);
            Line(sb, "UseDesignCriteriaFile", () => al.UseDesignCriteriaFile);
            Line(sb, "CriteriaFileName", () => al.CriteriaFileName);
            Line(sb, "UseDesignCheckSet", () => al.UseDesignCheckSet);
            Line(sb, "DesignCheckSetName", () => al.DesignCheckSetName);
            Line(sb, "StationEquations.Count", () => al.StationEquations.Count);

            try
            {
                foreach (DesignSpeed ds in al.DesignSpeeds)
                {
                    sb.AppendLine("  DesignSpeed #" + Get(() => ds.SpeedNumber)
                        + ": station " + Get(() => ds.Station)
                        + ", value " + Get(() => ds.Value)
                        + ", comment \"" + Get(() => ds.Comment) + "\"");
                }
            }
            catch (System.Exception e)
            {
                sb.AppendLine("  DesignSpeeds: <" + e.GetType().Name + ": " + e.Message + ">");
            }

            try
            {
                foreach (AlignmentEntity ent in al.Entities)
                {
                    sb.AppendLine("  Entity " + Get(() => ent.EntityId)
                        + " (before " + Get(() => ent.EntityBefore)
                        + ", after " + Get(() => ent.EntityAfter) + "): "
                        + Get(() => ent.EntityType)
                        + ", Constraint1 " + Get(() => ent.Constraint1)
                        + ", Constraint2 " + Get(() => Constraint2(ent))
                        + ", sub-entities " + Get(() => ent.SubEntityCount));
                }
            }
            catch (System.Exception e)
            {
                sb.AppendLine("  Entities: <" + e.GetType().Name + ": " + e.Message + ">");
            }
        }

        static object Constraint2(AlignmentEntity ent)
        {
            if (ent is AlignmentLine line)
            {
                return line.Constraint2;
            }
            if (ent is AlignmentArc arc)
            {
                return arc.Constraint2;
            }
            return "-";
        }

        // Some properties throw for some alignments; record the error instead.
        static string Get(Func<object> f)
        {
            try
            {
                return Convert.ToString(f());
            }
            catch (System.Exception e)
            {
                return "<" + e.GetType().Name + ": " + e.Message + ">";
            }
        }

        static void Line(StringBuilder sb, string name, Func<object> f)
        {
            sb.AppendLine("  " + name + ": " + Get(f));
        }
    }
}
