// C3DDUMP: write the public API values of every alignment in the current
// drawing, with its profiles and profile views, to <drawing>.api.txt, to
// compare with what civil3drust decodes.
// Read-only: it opens objects for read and changes nothing.

using System;
using System.Collections;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Text;
using Autodesk.AutoCAD.ApplicationServices;
using Autodesk.AutoCAD.DatabaseServices;
using Autodesk.AutoCAD.EditorInput;
using Autodesk.AutoCAD.Runtime;
using Autodesk.Civil.DatabaseServices;
using AcApp = Autodesk.AutoCAD.ApplicationServices.Application;
using DBObject = Autodesk.AutoCAD.DatabaseServices.DBObject;

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
                    DumpChildren(sb, tr, al, "GetProfileIds", "Profile", new[] { "PVIs", "Entities" });
                    DumpChildren(sb, tr, al, "GetProfileViewIds", "ProfileView", new string[0]);
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

        // Profiles and profile views are reached through the alignment's
        // public methods; their properties are read by reflection so the
        // add-in builds whatever members a Civil 3D version has.
        static void DumpChildren(StringBuilder sb, Transaction tr, Alignment al,
            string method, string label, string[] collections)
        {
            MethodInfo m = al.GetType().GetMethod(method, Type.EmptyTypes);
            if (m == null)
            {
                sb.AppendLine("  " + method + ": <not in this API>");
                return;
            }
            IEnumerable ids;
            try
            {
                ids = m.Invoke(al, null) as IEnumerable;
            }
            catch (System.Exception e)
            {
                sb.AppendLine("  " + method + ": <" + Inner(e) + ">");
                return;
            }
            if (ids == null)
            {
                return;
            }
            foreach (object o in ids)
            {
                if (!(o is ObjectId id))
                {
                    continue;
                }
                DBObject obj = tr.GetObject(id, OpenMode.ForRead);
                sb.AppendLine();
                sb.AppendLine("  " + label + " [" + obj.Handle + "] " + obj.GetType().Name);
                DumpSimple(sb, obj, "    ");
                foreach (string c in collections)
                {
                    PropertyInfo p = obj.GetType().GetProperty(c);
                    if (p == null)
                    {
                        continue;
                    }
                    object coll;
                    try
                    {
                        coll = p.GetValue(obj, null);
                    }
                    catch (System.Exception e)
                    {
                        sb.AppendLine("    " + c + ": <" + Inner(e) + ">");
                        continue;
                    }
                    if (!(coll is IEnumerable items))
                    {
                        continue;
                    }
                    int i = 0;
                    foreach (object item in items)
                    {
                        sb.AppendLine("    " + c + "[" + i++ + "] " + item.GetType().Name);
                        DumpSimple(sb, item, "      ");
                    }
                }
            }
        }

        static readonly HashSet<string> SkipProperties = new HashSet<string>
        {
            "Database", "Bounds", "GeometricExtents", "AcadObject", "ClassID",
            "UnmanagedObject", "AutoDelete", "IsDisposed", "DrawStream",
        };

        // Every readable property of a simple type (numbers, strings, enums,
        // points, object IDs as handles), sorted by name.
        static void DumpSimple(StringBuilder sb, object o, string indent)
        {
            IEnumerable<PropertyInfo> props = o.GetType()
                .GetProperties(BindingFlags.Public | BindingFlags.Instance)
                .Where(p => p.CanRead && p.GetIndexParameters().Length == 0
                    && !SkipProperties.Contains(p.Name))
                .OrderBy(p => p.Name, StringComparer.Ordinal);
            foreach (PropertyInfo p in props)
            {
                Type t = p.PropertyType;
                bool simple = t.IsPrimitive || t.IsEnum || t == typeof(string)
                    || t == typeof(ObjectId) || t.Name == "Point2d"
                    || t.Name == "Point3d" || t.Name == "Vector2d" || t.Name == "Vector3d";
                if (!simple)
                {
                    continue;
                }
                string v;
                try
                {
                    v = Format(p.GetValue(o, null));
                }
                catch (System.Exception e)
                {
                    v = "<" + Inner(e) + ">";
                }
                sb.AppendLine(indent + p.Name + ": " + v);
            }
        }

        static string Format(object v)
        {
            if (v == null)
            {
                return "null";
            }
            if (v is double d)
            {
                return d.ToString("R", CultureInfo.InvariantCulture);
            }
            if (v is ObjectId id)
            {
                return id.IsNull ? "null" : id.Handle.ToString();
            }
            return Convert.ToString(v, CultureInfo.InvariantCulture);
        }

        static string Inner(System.Exception e)
        {
            System.Exception x = e is TargetInvocationException && e.InnerException != null
                ? e.InnerException
                : e;
            return x.GetType().Name + ": " + x.Message;
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
