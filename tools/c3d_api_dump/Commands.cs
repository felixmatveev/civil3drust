// C3DDUMP: write the public API values of every alignment in the current
// drawing, with its profiles and profile views, then every site and
// feature line and the other Civil 3D entities in model space, to
// <drawing>.api.txt, to compare with what civil3drust decodes.
// C3DDUMPALL: Civil 3D objects in the database by class — "control" (the
// default: alignments, profiles, views, feature lines, sites, companions,
// helpers, label groups, check sets), "all", or DXF name patterns — with
// owner and simple properties, to <drawing>.api-all.txt.
// Read-only: both open objects for read and change nothing.

using System;
using System.Collections;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Text;
using System.Text.RegularExpressions;
using Autodesk.AutoCAD.ApplicationServices;
using Autodesk.AutoCAD.DatabaseServices;
using Autodesk.AutoCAD.EditorInput;
using Autodesk.AutoCAD.Geometry;
using Autodesk.AutoCAD.Runtime;
using Autodesk.Civil.ApplicationServices;
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
            var done = new HashSet<Handle>();

            using (Transaction tr = doc.Database.TransactionManager.StartTransaction())
            {
                foreach (ObjectId id in SelectAll(ed, "AECC_ALIGNMENT"))
                {
                    if (!(tr.GetObject(id, OpenMode.ForRead) is Alignment al))
                    {
                        continue;
                    }
                    done.Add(al.Handle);
                    DumpAlignment(sb, al);
                    DumpChildren(sb, tr, al, "GetProfileIds", "Profile", new[] { "PVIs", "Entities" }, done);
                    DumpChildren(sb, tr, al, "GetProfileViewIds", "ProfileView", new string[0], done);
                }

                sb.AppendLine();
                sb.AppendLine("== Sites");
                try
                {
                    foreach (ObjectId id in CivilApplication.ActiveDocument.GetSiteIds())
                    {
                        DumpObject(sb, tr, id, "Site", done);
                    }
                }
                catch (System.Exception e)
                {
                    sb.AppendLine("  <" + Inner(e) + ">");
                }

                sb.AppendLine();
                sb.AppendLine("== Feature lines");
                foreach (ObjectId id in SelectAll(ed, "AECC_FEATURE_LINE"))
                {
                    DumpObject(sb, tr, id, "FeatureLine", done);
                }

                sb.AppendLine();
                sb.AppendLine("== Other Civil 3D entities");
                foreach (ObjectId id in SelectAll(ed, "AECC_*"))
                {
                    if (!done.Contains(id.Handle))
                    {
                        DumpObject(sb, tr, id, "Entity", done);
                    }
                }
                tr.Commit();
            }
            Write(doc, ed, sb, ".api.txt", true);
        }

        [CommandMethod("C3DDUMPALL")]
        public void DumpAll()
        {
            Document doc = AcApp.DocumentManager.MdiActiveDocument;
            Editor ed = doc.Editor;
            Database db = doc.Database;
            var opt = new PromptStringOptions(
                "\nClasses: control, all, or DXF name patterns (comma-separated, * wildcard) <control>: ")
            {
                AllowSpaces = false,
            };
            PromptResult pr = ed.GetString(opt);
            if (pr.Status != PromptStatus.OK)
            {
                return;
            }
            string answer = string.IsNullOrWhiteSpace(pr.StringResult) ? "control" : pr.StringResult.Trim();
            string[] patterns = answer.Equals("control", StringComparison.OrdinalIgnoreCase)
                ? ControlClasses
                : answer.Equals("all", StringComparison.OrdinalIgnoreCase)
                    ? new[] { "AECC_*" }
                    : answer.Split(new[] { ',' }, StringSplitOptions.RemoveEmptyEntries);
            Regex match = new Regex(
                "^(" + string.Join("|", patterns.Select(x => Regex.Escape(x.Trim()).Replace("\\*", ".*"))) + ")$",
                RegexOptions.IgnoreCase);
            var sb = new StringBuilder();
            sb.AppendLine("drawing: " + doc.Name);
            sb.AppendLine("written: " + DateTime.UtcNow.ToString("u"));
            sb.AppendLine("classes: " + string.Join(", ", patterns));
            int n = 0;
            using (Transaction tr = db.TransactionManager.StartTransaction())
            {
                long seed = db.Handseed.Value;
                for (long h = 1; h < seed; h++)
                {
                    ObjectId id;
                    if (!db.TryGetObjectId(new Handle(h), out id) || id.IsNull || id.IsErased)
                    {
                        continue;
                    }
                    string dxf = id.ObjectClass.DxfName ?? "";
                    if (!dxf.StartsWith("AECC_", StringComparison.OrdinalIgnoreCase) || !match.IsMatch(dxf))
                    {
                        continue;
                    }
                    DBObject obj;
                    try
                    {
                        obj = tr.GetObject(id, OpenMode.ForRead);
                    }
                    catch (System.Exception e)
                    {
                        sb.AppendLine();
                        sb.AppendLine("[" + id.Handle + "] " + dxf + " <" + Inner(e) + ">");
                        continue;
                    }
                    sb.AppendLine();
                    sb.AppendLine("[" + obj.Handle + "] " + dxf + " " + obj.GetType().Name
                        + " owner " + Format(obj.OwnerId));
                    DumpSimple(sb, obj, "  ");
                    n++;
                }
                tr.Commit();
            }
            sb.AppendLine();
            sb.AppendLine("objects: " + n);
            Write(doc, ed, sb, ".api-all.txt", false);
        }

        // The classes around alignments, profiles, feature lines and sites.
        static readonly string[] ControlClasses =
        {
            "AECC_ALIGNMENT", "AECC_ALIGNMENT_*NODE", "AECC_PROFILE", "AECC_PROFILE_VIEW",
            "AECC_FEATURE_LINE", "AECC_FEATURE_STYLE", "AECC_PARCEL", "AECC_PARCEL_NODE",
            "AECC_DB_*", "AECC_ENTITY*", "AECC_NOTIFICATIONINFOS", "AECC_*DESIGN_CHECK_SET",
            "AECC_*LABEL_GROUP", "AECC_*LABELGROUP", "AECC_*LABELING", "AECC_GRADING*",
        };

        static ObjectId[] SelectAll(Editor ed, string dxfName)
        {
            var filter = new SelectionFilter(new[]
            {
                new TypedValue((int)DxfCode.Start, dxfName),
            });
            PromptSelectionResult sel = ed.SelectAll(filter);
            return sel.Status == PromptStatus.OK ? sel.Value.GetObjectIds() : new ObjectId[0];
        }

        static void Write(Document doc, Editor ed, StringBuilder sb, string suffix, bool echo)
        {
            string folder = Path.GetDirectoryName(doc.Database.Filename);
            if (string.IsNullOrEmpty(folder))
            {
                folder = Path.GetTempPath();
            }
            string path = Path.Combine(
                folder,
                Path.GetFileNameWithoutExtension(doc.Name) + suffix);
            File.WriteAllText(path, sb.ToString());
            ed.WriteMessage((echo ? "\n" + sb : "") + "\nC3DDUMP: written to " + path + "\n");
        }

        // One object: header line, every simple property, then the values
        // of its parameterless Get… methods (and of Get…(enum) for every
        // value of the enum), e.g. a feature line's GetPoints(PIPoints).
        static void DumpObject(StringBuilder sb, Transaction tr, ObjectId id, string label,
            HashSet<Handle> done)
        {
            DBObject obj;
            try
            {
                obj = tr.GetObject(id, OpenMode.ForRead);
            }
            catch (System.Exception e)
            {
                sb.AppendLine("  " + label + " [" + id.Handle + "] <" + Inner(e) + ">");
                return;
            }
            done.Add(obj.Handle);
            sb.AppendLine();
            sb.AppendLine("  " + label + " [" + obj.Handle + "] " + obj.GetType().Name
                + " (" + id.ObjectClass.DxfName + ")");
            DumpSimple(sb, obj, "    ");
            DumpGetters(sb, obj, "    ");
        }

        static readonly HashSet<string> SkipMethods = new HashSet<string>
        {
            "GetType", "GetHashCode", "GetRXClass", "GetObjectSaveVersion",
            "GetField", "GetFields", "GetPersistentReactorIds", "GetTransientReactors",
            "GetXDataForApplication", "GetObjectBirthVersion", "GetAcGeCurve",
            "GetGeCurve", "GetOrthoProjectedCurve", "GetPlane", "GetSubentityGeometry",
            "GetGripPoints", "GetStretchPoints", "GetOsnapPoints", "GetTransformedCopy",
            "GetHyperlinkCollection", "GetAnonymousBlockIds", "GetAnnotativeScaleContexts",
            "GetLifetimeService", "GetReferenceInfo",
        };

        static void DumpGetters(StringBuilder sb, object o, string indent)
        {
            IEnumerable<MethodInfo> methods = o.GetType()
                .GetMethods(BindingFlags.Public | BindingFlags.Instance)
                .Where(m => m.Name.StartsWith("Get", StringComparison.Ordinal)
                    && !m.IsSpecialName && !m.ContainsGenericParameters
                    && m.ReturnType != typeof(void) && !SkipMethods.Contains(m.Name))
                .OrderBy(m => m.Name, StringComparer.Ordinal);
            foreach (MethodInfo m in methods)
            {
                ParameterInfo[] ps = m.GetParameters();
                if (ps.Length == 0)
                {
                    Invoke(sb, o, m, null, m.Name + "()", indent);
                }
                else if (ps.Length == 1 && ps[0].ParameterType.IsEnum)
                {
                    foreach (object v in Enum.GetValues(ps[0].ParameterType))
                    {
                        Invoke(sb, o, m, new[] { v }, m.Name + "(" + v + ")", indent);
                    }
                }
            }
        }

        static void Invoke(StringBuilder sb, object o, MethodInfo m, object[] args, string name,
            string indent)
        {
            object r;
            try
            {
                r = m.Invoke(o, args);
            }
            catch (System.Exception e)
            {
                sb.AppendLine(indent + name + ": <" + Inner(e) + ">");
                return;
            }
            if (IsSimple(r == null ? typeof(string) : r.GetType()))
            {
                sb.AppendLine(indent + name + ": " + Format(r));
                return;
            }
            if (r is IEnumerable items && !(r is string))
            {
                var vals = new List<string>();
                foreach (object item in items)
                {
                    if (item == null || IsSimple(item.GetType()))
                    {
                        vals.Add(Format(item));
                    }
                    else
                    {
                        vals.Add(item.GetType().Name);
                    }
                }
                sb.AppendLine(indent + name + " [" + vals.Count + "]: " + string.Join(", ", vals));
            }
        }

        static bool IsSimple(Type t)
        {
            return t.IsPrimitive || t.IsEnum || t == typeof(string) || t == typeof(decimal)
                || t == typeof(ObjectId) || t == typeof(Handle) || t == typeof(Point2d)
                || t == typeof(Point3d) || t == typeof(Vector2d) || t == typeof(Vector3d);
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
            string method, string label, string[] collections, HashSet<Handle> done)
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
                done.Add(obj.Handle);
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

        // Not part of the object's data: handles repeated in the header,
        // and the transaction's view of the open object.
        static readonly HashSet<string> SkipProperties = new HashSet<string>
        {
            "Database", "Bounds", "GeometricExtents", "AcadObject", "ClassID",
            "UnmanagedObject", "AutoDelete", "IsDisposed", "DrawStream",
            "Handle", "Id", "ObjectId", "OwnerId", "DrawableType", "PaperOrientation",
            "Annotative", "MergeStyle", "HasFields", "HasSaveVersionOverride",
            "IsAProxy", "IsCancelling", "IsEraseStatusToggled", "IsErased", "IsModified",
            "IsModifiedGraphics", "IsModifiedXData", "IsNewObject", "IsNotifyEnabled",
            "IsNotifying", "IsObjectIdsInFlux", "IsPersistent", "IsReadEnabled",
            "IsReallyClosing", "IsTransactionResident", "IsUndoing", "IsWriteEnabled",
            "IsHighlighting", "ForceAnnoAllVisible", "CloneMeForDragging",
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
                if (!IsSimple(p.PropertyType))
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
            if (v is Point3d p3)
            {
                return "(" + Format(p3.X) + ", " + Format(p3.Y) + ", " + Format(p3.Z) + ")";
            }
            if (v is Point2d p2)
            {
                return "(" + Format(p2.X) + ", " + Format(p2.Y) + ")";
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
