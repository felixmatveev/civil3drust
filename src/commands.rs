//! The command registry (D-074): every edit civil3drust offers, callable by
//! name with text arguments.
//!
//! Each [`Command`] describes itself (name, help, argument spec) and runs
//! on an open document. The `c3dr` binary and the workbench both dispatch
//! through [`run_line`], so a command added here is available in both with
//! no GUI work; `help` lists them and [`spec`] gives the same list as data
//! (for generated forms, or for a GUI talking to a newer `c3dr`).
//!
//! Adding a command: write the edit in its module, then add one entry to
//! [`COMMANDS`] with a `run` function that looks up its arguments, calls
//! the edit and returns a one-line report.

use crate::aecc_alignment_edit::{set_pis, set_tangent_length};
use crate::aecc_alignment_refs::{alignment_refs, alignment_styles, set_alignment_style};
use crate::aecc_arc_edit::set_free_arc_radius;
use crate::aecc_create::{create_alignment, CreateOptions};
use crate::aecc_elements::{decode_elements, DecodedElements, Element, Segment};
use crate::aecc_station_refs::{
    set_masking_regions, set_station_equations, EquationSpec, MaskSpec,
};
use crate::alignment::Point2;
use crate::raw::{self, RawRecord};
use crate::{aecc_build, aecc_style, aecc_topology, handle_refs, string_stream};
use acadrust::document::CadDocument;
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;

/// What an argument holds; decides how it is parsed and how a GUI asks
/// for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArgKind {
    /// An `AECC_ALIGNMENT`: its name or hex handle (`2A3`, `#2A3`, `0x2A3`).
    Alignment,
    /// An `AECC_ALIGNMENT_STYLE`: name or hex handle.
    AlignmentStyle,
    /// Any record, by hex handle.
    Handle,
    Int,
    Number,
    Text,
    /// `"x,y x,y ..."`; with [`ArgKind::PointsRadii`] interior points may
    /// be `x,y,r`.
    Points,
    PointsRadii,
    /// A path to another DWG.
    File,
    /// A `--flag` with no value.
    Flag,
    /// One of a fixed set of words (listed in the help text).
    Choice,
}

/// One argument. Names starting with `--` are options (anywhere on the
/// line); the rest are positional, in order.
#[derive(Debug, Clone, Copy)]
pub struct ArgSpec {
    pub name: &'static str,
    pub kind: ArgKind,
    pub help: &'static str,
    pub optional: bool,
}

const fn arg(name: &'static str, kind: ArgKind, help: &'static str) -> ArgSpec {
    ArgSpec {
        name,
        kind,
        help,
        optional: false,
    }
}

const fn opt(name: &'static str, kind: ArgKind, help: &'static str) -> ArgSpec {
    ArgSpec {
        name,
        kind,
        help,
        optional: true,
    }
}

/// One command.
pub struct Command {
    pub name: &'static str,
    pub help: &'static str,
    pub args: &'static [ArgSpec],
    /// False for commands that only report (no file is written by `c3dr`).
    pub edits: bool,
    pub run: fn(&mut CadDocument, &Args) -> Result<String, String>,
}

impl Command {
    /// `name <arg> [<opt>] [--flag] [--opt <value>]`.
    pub fn usage(&self) -> String {
        let mut s = self.name.to_string();
        for a in self.args {
            let part = match (a.name.starts_with("--"), a.kind, a.optional) {
                (true, ArgKind::Flag, _) => format!("[{}]", a.name),
                (true, _, _) => format!("[{} <value>]", a.name),
                (false, _, false) => format!("<{}>", a.name),
                (false, _, true) => format!("[<{}>]", a.name),
            };
            s.push(' ');
            s.push_str(&part);
        }
        s
    }
}

/// A command's description as data (see [`spec`]).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandInfo {
    pub name: String,
    pub help: String,
    pub edits: bool,
    pub args: Vec<ArgInfo>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArgInfo {
    pub name: String,
    pub kind: ArgKind,
    pub help: String,
    pub optional: bool,
}

/// Arguments of one call, by name, as typed.
#[derive(Debug, Clone, Default)]
pub struct Args {
    values: Vec<(String, String)>,
    flags: Vec<String>,
}

impl Args {
    /// The raw text of argument `name`, if given.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.values
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }

    fn req(&self, name: &str) -> Result<&str, String> {
        self.get(name).ok_or_else(|| format!("missing <{name}>"))
    }

    pub fn flag(&self, name: &str) -> bool {
        self.flags.iter().any(|f| f == name)
    }

    pub fn text(&self, name: &str) -> Result<String, String> {
        self.req(name).map(str::to_string)
    }

    pub fn number(&self, name: &str) -> Result<f64, String> {
        let v = self.req(name)?;
        v.parse()
            .map_err(|_| format!("<{name}> must be a number, got '{v}'"))
    }

    pub fn int(&self, name: &str) -> Result<i64, String> {
        let v = self.req(name)?;
        v.parse()
            .map_err(|_| format!("<{name}> must be a whole number, got '{v}'"))
    }

    pub fn handle(&self, name: &str) -> Result<u64, String> {
        let v = self.req(name)?;
        parse_handle(v).ok_or_else(|| format!("<{name}> must be a hex handle, got '{v}'"))
    }

    pub fn points(&self, name: &str) -> Result<Vec<Point2>, String> {
        let (p, r) = parse_points(self.req(name)?)?;
        if r.iter().any(Option::is_some) {
            return Err(format!("<{name}> takes x,y points only"));
        }
        Ok(p)
    }

    /// Points with optional radii at interior points.
    pub fn points_radii(&self, name: &str) -> Result<(Vec<Point2>, Vec<Option<f64>>), String> {
        parse_points(self.req(name)?)
    }

    /// The alignment record named by argument `name`.
    pub fn alignment(&self, doc: &CadDocument, name: &str) -> Result<RawRecord, String> {
        find_record(doc, "AECC_ALIGNMENT", self.req(name)?)
    }
}

/// `2A3`, `#2A3` or `0x2A3`.
pub fn parse_handle(s: &str) -> Option<u64> {
    let t = s.trim().trim_start_matches('#').trim_start_matches("0x");
    u64::from_str_radix(t, 16).ok()
}

/// A record of `class` by name or hex handle (a name wins over a handle
/// that happens to spell the same).
pub fn find_record(doc: &CadDocument, class: &str, key: &str) -> Result<RawRecord, String> {
    let recs = raw::records(doc, class);
    let by_name = recs
        .iter()
        .find(|r| raw::object_name(&r.data).as_deref() == Some(key));
    if let Some(r) = by_name {
        return Ok(r.clone());
    }
    let h = parse_handle(key);
    recs.into_iter()
        .find(|r| Some(r.handle) == h)
        .ok_or_else(|| format!("no {class} named or with handle '{key}'"))
}

fn parse_points(s: &str) -> Result<(Vec<Point2>, Vec<Option<f64>>), String> {
    let items: Vec<&str> = s.split_whitespace().collect();
    let mut pis = Vec::new();
    let mut radii = Vec::new();
    for (i, p) in items.iter().enumerate() {
        let f: Vec<f64> = p
            .split(',')
            .map(|v| v.trim().parse::<f64>())
            .collect::<Result<_, _>>()
            .map_err(|_| format!("bad number in point '{p}'"))?;
        if f.len() < 2 || f.len() > 3 {
            return Err(format!("point '{p}' must be x,y or x,y,r"));
        }
        pis.push(Point2::new(f[0], f[1]));
        let interior = i > 0 && i + 1 < items.len();
        if f.len() == 3 && !interior {
            return Err(format!("'{p}': only interior points can have a radius"));
        }
        if interior {
            radii.push(f.get(2).copied());
        }
    }
    Ok((pis, radii))
}

/// Split a command line into words; double quotes group words, `\"` is a
/// literal quote.
pub fn tokenize(line: &str) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_q = false;
    let mut any = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' if chars.peek() == Some(&'"') => {
                cur.push('"');
                chars.next();
                any = true;
            }
            '"' => {
                in_q = !in_q;
                any = true;
            }
            c if c.is_whitespace() && !in_q => {
                if any {
                    out.push(std::mem::take(&mut cur));
                    any = false;
                }
            }
            c => {
                cur.push(c);
                any = true;
            }
        }
    }
    if in_q {
        return Err("unclosed quote".into());
    }
    if any {
        out.push(cur);
    }
    Ok(out)
}

/// Quote a word for a command line if it needs it.
pub fn quote(s: &str) -> String {
    if !s.is_empty() && !s.chars().any(|c| c.is_whitespace() || c == '"') {
        s.to_string()
    } else {
        format!("\"{}\"", s.replace('"', "\\\""))
    }
}

/// Match `words` (without the command name) against the command's spec.
pub fn parse_args(cmd: &Command, words: &[String]) -> Result<Args, String> {
    let mut a = Args::default();
    let positional: Vec<&ArgSpec> = cmd
        .args
        .iter()
        .filter(|s| !s.name.starts_with("--"))
        .collect();
    let mut next = 0;
    let mut i = 0;
    let usage = || format!("usage: {}", cmd.usage());
    while i < words.len() {
        let w = &words[i];
        if w.starts_with("--") {
            let spec = cmd
                .args
                .iter()
                .find(|s| s.name == w)
                .ok_or_else(|| format!("unknown option {w}; {}", usage()))?;
            if spec.kind == ArgKind::Flag {
                a.flags.push(w.clone());
            } else {
                let v = words
                    .get(i + 1)
                    .ok_or_else(|| format!("{w} needs a value"))?;
                a.values.push((w.clone(), v.clone()));
                i += 1;
            }
        } else {
            let spec = positional
                .get(next)
                .ok_or_else(|| format!("too many arguments; {}", usage()))?;
            a.values.push((spec.name.to_string(), w.clone()));
            next += 1;
        }
        i += 1;
    }
    if let Some(missing) = positional[next.min(positional.len())..]
        .iter()
        .find(|s| !s.optional)
    {
        return Err(format!("missing <{}>; {}", missing.name, usage()));
    }
    Ok(a)
}

/// The command called `name`.
pub fn find(name: &str) -> Option<&'static Command> {
    COMMANDS.iter().find(|c| c.name == name)
}

/// Every command, in help order.
pub fn commands() -> &'static [Command] {
    COMMANDS
}

/// Every command as data.
pub fn spec() -> Vec<CommandInfo> {
    COMMANDS
        .iter()
        .map(|c| CommandInfo {
            name: c.name.into(),
            help: c.help.into(),
            edits: c.edits,
            args: c
                .args
                .iter()
                .map(|a| ArgInfo {
                    name: a.name.into(),
                    kind: a.kind,
                    help: a.help.into(),
                    optional: a.optional,
                })
                .collect(),
        })
        .collect()
}

/// Parse and run one command line on `doc`. On error `doc` may be partly
/// changed; callers that need all-or-nothing run it on a copy.
pub fn run_line(doc: &mut CadDocument, line: &str) -> Result<String, String> {
    let words = tokenize(line)?;
    let Some((name, rest)) = words.split_first() else {
        return Err("empty command".into());
    };
    let cmd = find(name).ok_or_else(|| format!("unknown command '{name}' (try: help)"))?;
    let args = parse_args(cmd, rest)?;
    (cmd.run)(doc, &args)
}

/// `help` text: the list, or one command in full.
pub fn help_text(name: Option<&str>) -> Result<String, String> {
    let mut s = String::new();
    match name {
        None => {
            let w = COMMANDS.iter().map(|c| c.name.len()).max().unwrap_or(0);
            for c in COMMANDS {
                let _ = writeln!(s, "{:<w$}  {}", c.name, c.help);
            }
            s.push_str("help <command> for its arguments");
        }
        Some(n) => {
            let c = find(n).ok_or_else(|| format!("unknown command '{n}'"))?;
            let _ = writeln!(s, "{}\n  {}", c.usage(), c.help);
            for a in c.args {
                let _ = writeln!(s, "  {:<16} {}", a.name, a.help);
            }
        }
    }
    Ok(s.trim_end().to_string())
}

// ---------------------------------------------------------------------------
// Shared steps

/// A seed for new IDs: varies per call (new_id_string avoids clashes).
fn seed(salt: u64) -> u64 {
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(1);
    t ^ salt.rotate_left(17)
}

fn decoded(rec: &RawRecord) -> Result<DecodedElements, String> {
    decode_elements(rec).map_err(|e| format!("alignment {:X}: {e}", rec.handle))
}

fn label(rec: &RawRecord) -> String {
    match raw::object_name(&rec.data) {
        Some(n) => format!("[{:X}] {n}", rec.handle),
        None => format!("[{:X}]", rec.handle),
    }
}

/// After an in-place element edit: rebuild the companion graph and drop
/// re-save caches, as the `set_*` examples do (D-046, D-047).
fn after_geometry_edit(doc: &mut CadDocument, handle: u64, drop_caches: bool) -> String {
    let note = match aecc_topology::refresh_companion(doc, handle) {
        Ok(true) => "companion rebuilt".to_string(),
        Ok(false) => "companion kept (site member or unknown form)".to_string(),
        Err(e) => format!("companion not refreshed: {e}"),
    };
    if drop_caches {
        aecc_build::drop_record_caches(doc, handle);
    }
    note
}

fn put(doc: &mut CadDocument, handle: u64, data: Vec<u8>, hb: usize) -> Result<(), String> {
    if raw::set_record(doc, handle, data, hb) {
        Ok(())
    } else {
        Err(format!("could not replace record {handle:X}"))
    }
}

fn style_handle(doc: &CadDocument, key: &str) -> Result<u64, String> {
    let h = parse_handle(key);
    alignment_styles(doc)
        .into_iter()
        .find(|(sh, n)| n == key || Some(*sh) == h)
        .map(|s| s.0)
        .ok_or_else(|| format!("no alignment style '{key}'"))
}

// ---------------------------------------------------------------------------
// Commands

use ArgKind as K;

const ALIGNMENT: ArgSpec = arg("alignment", K::Alignment, "alignment name or hex handle");
const SITE: ArgSpec = arg("site", K::Text, "site name or hex handle");

static COMMANDS: &[Command] = &[
    Command {
        name: "help",
        help: "list commands, or show one command's arguments",
        args: &[opt("command", K::Text, "command name")],
        edits: false,
        run: |_, a| help_text(a.get("command")),
    },
    Command {
        name: "alignments",
        help: "list alignments: handle, name, elements, length, style",
        args: &[],
        edits: false,
        run: run_alignments,
    },
    Command {
        name: "elements",
        help: "list an alignment's elements with IDs, kinds and lengths",
        args: &[ALIGNMENT],
        edits: false,
        run: run_elements,
    },
    Command {
        name: "styles",
        help: "list alignment styles and their component visibility flags",
        args: &[],
        edits: false,
        run: run_styles,
    },
    Command {
        name: "refs",
        help: "list the handle references stored in a record",
        args: &[arg("record", K::Handle, "hex handle")],
        edits: false,
        run: run_refs,
    },
    Command {
        name: "set_length",
        help: "change the length of a tangent (segments numbered from 1)",
        args: &[
            ALIGNMENT,
            arg("segment", K::Int, "tangent number, from 1, in path order"),
            arg("length", K::Number, "new length"),
        ],
        edits: true,
        run: run_set_length,
    },
    Command {
        name: "set_radius",
        help: "change the radius of a free curve",
        args: &[
            ALIGNMENT,
            arg("arc_id", K::Int, "element ID of the curve"),
            arg("radius", K::Number, "new radius"),
        ],
        edits: true,
        run: run_set_radius,
    },
    Command {
        name: "set_pis",
        help: "move the PIs (same count as now)",
        args: &[ALIGNMENT, arg("pis", K::Points, "\"x,y x,y ...\"")],
        edits: true,
        run: run_set_pis,
    },
    Command {
        name: "curve",
        help: "add a free curve at the end of a tangent",
        args: &[
            ALIGNMENT,
            arg(
                "tangent_id",
                K::Int,
                "element ID of the tangent before the curve",
            ),
            arg("radius", K::Number, "curve radius"),
        ],
        edits: true,
        run: run_curve,
    },
    Command {
        name: "remove",
        help: "remove an element",
        args: &[ALIGNMENT, arg("element_id", K::Int, "element ID")],
        edits: true,
        run: run_remove,
    },
    Command {
        name: "set_name",
        help: "rename an alignment and/or set its description",
        args: &[
            ALIGNMENT,
            arg("name", K::Text, "new name, or - to keep it"),
            opt(
                "--description",
                K::Text,
                "description text (\\n for a new line)",
            ),
        ],
        edits: true,
        run: run_set_name,
    },
    Command {
        name: "set_start",
        help: "set the start station (design speeds move with it)",
        args: &[
            ALIGNMENT,
            arg("station", K::Number, "new start station"),
            opt(
                "--clear-speeds",
                K::Flag,
                "drop design speeds instead of moving them",
            ),
        ],
        edits: true,
        run: run_set_start,
    },
    Command {
        name: "set_speeds",
        help: "replace the design speeds",
        args: &[ALIGNMENT, arg("speeds", K::Text, "\"station:speed ...\"")],
        edits: true,
        run: run_set_speeds,
    },
    Command {
        name: "set_style",
        help: "give an alignment another alignment style",
        args: &[
            ALIGNMENT,
            arg("style", K::AlignmentStyle, "style name or hex handle"),
        ],
        edits: true,
        run: run_set_style,
    },
    Command {
        name: "style_component",
        help: "show or hide one display component of an alignment style",
        args: &[
            arg("style", K::AlignmentStyle, "style name or hex handle"),
            arg("component", K::Text, "view:name (e.g. plan:curve) or 0-16"),
            arg("state", K::Choice, "on | off"),
        ],
        edits: true,
        run: run_style_component,
    },
    Command {
        name: "equations",
        help: "replace the station equations (\"\" removes them)",
        args: &[
            ALIGNMENT,
            arg("equations", K::Text, "\"raw:ahead[:dec][:comment] ...\""),
        ],
        edits: true,
        run: run_equations,
    },
    Command {
        name: "masking",
        help: "replace the masking regions (\"\" removes them)",
        args: &[
            ALIGNMENT,
            arg(
                "regions",
                K::Text,
                "\"start-end[:off][:lockstart][:lockend][:comment] ...\"",
            ),
        ],
        edits: true,
        run: run_masking,
    },
    Command {
        name: "create",
        help: "create an alignment from PIs, using a template alignment from a donor drawing",
        args: &[
            arg("donor", K::File, "DWG holding the template alignment"),
            arg("name", K::Text, "new alignment name"),
            arg(
                "pis",
                K::PointsRadii,
                "\"x,y x,y,r ... x,y\" (r: curve at that PI)",
            ),
            opt("--style", K::AlignmentStyle, "style in this drawing"),
            opt(
                "--template",
                K::Handle,
                "template alignment handle in the donor",
            ),
            opt("--type", K::Text, "centerline | misc | a number"),
            opt(
                "--no-label-group",
                K::Flag,
                "skip the station-equation label group",
            ),
            opt(
                "--no-parcel-entry",
                K::Flag,
                "no companion or registry entry",
            ),
            opt("--no-graphics", K::Flag, "no proxy graphics"),
        ],
        edits: true,
        run: run_create,
    },
    Command {
        name: "delete",
        help: "delete a siteless alignment as Civil 3D does (D-062)",
        args: &[ALIGNMENT],
        edits: true,
        run: run_delete,
    },
    Command {
        name: "sites",
        help: "list sites (handle, name, description, members) and siteless alignments",
        args: &[],
        edits: false,
        run: run_sites,
    },
    Command {
        name: "site_create",
        help: "create an empty site, cloned from the drawing's first site (D-079)",
        args: &[arg("name", K::Text, "name of the new site")],
        edits: true,
        run: run_site_create,
    },
    Command {
        name: "site_rename",
        help: "rename a site and/or set its description",
        args: &[
            SITE,
            arg("name", K::Text, "new name, or - to keep it"),
            opt("--description", K::Text, "description text"),
        ],
        edits: true,
        run: run_site_rename,
    },
    Command {
        name: "site_delete",
        help: "delete a site; with --with-members its alignments and feature lines go too, as in Civil 3D (S375)",
        args: &[
            SITE,
            opt("--with-members", K::Flag, "also delete the site's alignments and feature lines"),
        ],
        edits: true,
        run: run_site_delete,
    },
    Command {
        name: "site_move_in",
        help: "move a siteless alignment into a site (crossings and spirals included; Civil 3D adds the crossing points)",
        args: &[ALIGNMENT, SITE],
        edits: true,
        run: run_site_move_in,
    },
    Command {
        name: "site_move_out",
        help: "move an alignment out of its site (back to siteless)",
        args: &[ALIGNMENT],
        edits: true,
        run: run_site_move_out,
    },
];

/// A site by name or hex handle.
fn site_arg(doc: &CadDocument, a: &Args) -> Result<crate::aecc_site::Site, String> {
    let t = a.text("site")?;
    let all = crate::aecc_site::sites(doc);
    let by_name: Vec<_> = all.iter().filter(|s| s.name == t).collect();
    if let [one] = by_name[..] {
        return Ok(one.clone());
    }
    if by_name.len() > 1 {
        return Err(format!(
            "{} sites are named '{t}': use the handle",
            by_name.len()
        ));
    }
    let h = u64::from_str_radix(t.trim_start_matches('#').trim_start_matches("0x"), 16).ok();
    all.into_iter()
        .find(|s| Some(s.handle) == h)
        .ok_or(format!("no site '{t}'"))
}

fn site_label(s: &crate::aecc_site::Site) -> String {
    format!("[{:X}] '{}'", s.handle, s.name)
}

fn run_sites(doc: &mut CadDocument, _: &Args) -> Result<String, String> {
    use crate::aecc_site::{key_has_string, membership, sites, Membership};
    let all = sites(doc);
    let als = raw::records(doc, "AECC_ALIGNMENT");
    let place: Vec<(String, Membership)> =
        als.iter().map(|r| (label(r), membership(doc, r))).collect();
    let mut s = String::new();
    for site in &all {
        let members: Vec<&str> = place
            .iter()
            .filter(|(_, m)| matches!(m, Membership::InSite { site: h, .. } if *h == site.handle))
            .map(|(l, _)| l.as_str())
            .collect();
        let key = if key_has_string(site.name_key) {
            String::new()
        } else {
            format!(" (built-in name {:08X})", site.name_key)
        };
        let _ = writeln!(
            s,
            "{}{key}: description '{}', alignments: {}",
            site_label(site),
            site.description,
            if members.is_empty() {
                "none".to_string()
            } else {
                members.join(", ")
            }
        );
    }
    let free: Vec<&str> = place
        .iter()
        .filter(|(_, m)| matches!(m, Membership::Siteless { .. }))
        .map(|(l, _)| l.as_str())
        .collect();
    let _ = writeln!(
        s,
        "siteless: {}",
        if free.is_empty() {
            "none".into()
        } else {
            free.join(", ")
        }
    );
    Ok(s.trim_end().to_string())
}

fn run_site_create(doc: &mut CadDocument, a: &Args) -> Result<String, String> {
    let name = a.text("name")?;
    if crate::aecc_site::sites(doc).iter().any(|s| s.name == name) {
        return Err(format!("a site named '{name}' exists"));
    }
    let c = crate::aecc_site_edit::create_site(doc, &name).map_err(|e| e.to_string())?;
    Ok(format!(
        "created site [{:X}] '{name}': container {:X}, key {:08X}",
        c.site, c.container, c.key
    ))
}

fn run_site_rename(doc: &mut CadDocument, a: &Args) -> Result<String, String> {
    let site = site_arg(doc, a)?;
    let name = a.text("name")?;
    let new = (name != "-").then_some(name.as_str());
    if let Some(n) = new {
        if n.is_empty() {
            return Err("a site needs a name".into());
        }
        if crate::aecc_site::sites(doc)
            .iter()
            .any(|s| s.name == n && s.handle != site.handle)
        {
            return Err(format!("a site named '{n}' exists"));
        }
    }
    if new.is_none() && a.get("--description").is_none() {
        return Err("nothing to change".into());
    }
    crate::aecc_site_edit::rename_site(doc, site.handle, new, a.get("--description"))
        .map_err(|e| e.to_string())?;
    Ok(format!(
        "{} -> '{}'{}",
        site_label(&site),
        new.unwrap_or(&site.name),
        a.get("--description")
            .map(|d| format!(", description '{d}'"))
            .unwrap_or_default()
    ))
}

fn run_site_delete(doc: &mut CadDocument, a: &Args) -> Result<String, String> {
    let site = site_arg(doc, a)?;
    if !a.flag("--with-members") {
        crate::aecc_site_edit::delete_site(doc, site.handle).map_err(|e| e.to_string())?;
        return Ok(format!("deleted site {}", site_label(&site)));
    }
    let d = crate::aecc_site_edit::delete_site_with_members(doc, site.handle)
        .map_err(|e| e.to_string())?;
    let hx = |v: &[u64]| {
        v.iter()
            .map(|h| format!("{h:X}"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    Ok(format!(
        "deleted site {} with alignments [{}], feature lines [{}], label groups [{}]",
        site_label(&site),
        hx(&d.alignments),
        hx(&d.feature_lines),
        hx(&d.label_groups)
    ))
}

fn run_site_move_in(doc: &mut CadDocument, a: &Args) -> Result<String, String> {
    let rec = a.alignment(doc, "alignment")?;
    let site = site_arg(doc, a)?;
    let m = crate::aecc_site_edit::move_into_site(doc, rec.handle, site.handle)
        .map_err(|e| e.to_string())?;
    Ok(format!(
        "{} -> site {} (container {:X}); companion {:X} emptied",
        label(&rec),
        site_label(&site),
        m.container,
        m.companion
    ))
}

fn run_site_move_out(doc: &mut CadDocument, a: &Args) -> Result<String, String> {
    let rec = a.alignment(doc, "alignment")?;
    let m = crate::aecc_site_edit::move_out_of_site(doc, rec.handle).map_err(|e| e.to_string())?;
    Ok(format!(
        "{} -> siteless (companion {:X}); left site {:X}",
        label(&rec),
        m.companion,
        m.site
    ))
}

fn run_alignments(doc: &mut CadDocument, _: &Args) -> Result<String, String> {
    let styles = alignment_styles(doc);
    let mut s = String::new();
    for rec in raw::records(doc, "AECC_ALIGNMENT") {
        let style = alignment_refs(&rec)
            .and_then(|r| styles.iter().find(|x| x.0 == r.style.handle).cloned())
            .map(|(h, n)| format!("style [{h:X}] {n}"))
            .unwrap_or_default();
        match decode_elements(&rec) {
            Ok(d) => {
                let _ = writeln!(
                    s,
                    "{}: {} element(s), L={:.4}, start {:.4}  {style}",
                    label(&rec),
                    d.elements.len(),
                    d.length(),
                    d.stored_start_station.unwrap_or(0.0)
                );
            }
            Err(e) => {
                let _ = writeln!(s, "{}: not decoded ({e})", label(&rec));
            }
        }
    }
    if s.is_empty() {
        s.push_str("no alignments");
    }
    Ok(s.trim_end().into())
}

fn element_id(e: &Element) -> Option<i32> {
    match e {
        Element::Tangent(t) => Some(t.id),
        Element::Arc(a) => Some(a.id),
        _ => None,
    }
}

fn run_elements(doc: &mut CadDocument, a: &Args) -> Result<String, String> {
    let rec = a.alignment(doc, "alignment")?;
    let d = decoded(&rec)?;
    let mut s = format!("{}  L={:.4}\n", label(&rec), d.length());
    let mut sta = d.stored_start_station.unwrap_or(0.0);
    let mut tangent_no = 0;
    for e in &d.elements {
        let (p, q) = (e.segment.start(), e.segment.end());
        let kind = match &e.segment {
            Segment::Line { .. } => {
                tangent_no += 1;
                format!("tangent #{tangent_no}")
            }
            Segment::Arc { geometry, .. } => format!("arc R={:.4}", geometry.radius),
            Segment::Spiral { geometry, .. } => format!("spiral R={:.4}", geometry.radius),
        };
        let id = element_id(&e.record).map_or("-".into(), |i| i.to_string());
        let len = e.segment.length();
        let _ = writeln!(
            s,
            "  id {id:>3}  {kind:<16} ({:.3}, {:.3}) -> ({:.3}, {:.3})  L={len:.4}  sta {sta:.3}-{:.3}",
            p.x,
            p.y,
            q.x,
            q.y,
            sta + len
        );
        sta += len;
    }
    Ok(s.trim_end().into())
}

fn run_styles(doc: &mut CadDocument, _: &Args) -> Result<String, String> {
    let mut s = String::from(
        "flags: plan 8 | model 8 | section 1 (Line, Curve, Spiral, Arrow, extensions, warning)\n",
    );
    for (h, name) in alignment_styles(doc) {
        let flags = raw::records(doc, "AECC_ALIGNMENT_STYLE")
            .into_iter()
            .find(|r| r.handle == h)
            .and_then(|r| aecc_style::flags_string(&r.data))
            .unwrap_or_else(|| "?".into());
        let _ = writeln!(s, "  [{h:X}] {name:<24} {flags}");
    }
    Ok(s.trim_end().into())
}

fn run_refs(doc: &mut CadDocument, a: &Args) -> Result<String, String> {
    let h = a.handle("record")?;
    let rec = crate::dump::raw_record(doc, h).ok_or_else(|| format!("no raw record {h:X}"))?;
    let mut s = format!("[{h:X}] {}\n", rec.class);
    for r in handle_refs::of_record(&rec) {
        let class = raw::class_of(doc, r.handle).unwrap_or_else(|| "?".into());
        let _ = writeln!(
            s,
            "  bit {:>6}  code {:>2}  -> [{:X}] {class}",
            r.bit, r.code, r.handle
        );
    }
    Ok(s.trim_end().into())
}

fn run_set_length(doc: &mut CadDocument, a: &Args) -> Result<String, String> {
    let rec = a.alignment(doc, "alignment")?;
    let seg = a.int("segment")?;
    if seg < 1 {
        return Err("segments are numbered from 1".into());
    }
    let len = a.number("length")?;
    let d = decoded(&rec)?;
    let data = raw::record_data_mut(doc, rec.handle).ok_or("record bytes missing")?;
    let rep = set_tangent_length(data, &d, seg as usize - 1, len).map_err(|e| e.to_string())?;
    let note = after_geometry_edit(doc, rec.handle, true);
    Ok(format!(
        "{}: tangent {seg} L={len}; {} tangent record(s), {} curve(s) restationed; {note}",
        label(&rec),
        rep.patches.tangent_records,
        rep.arcs_restationed
    ))
}

fn run_set_radius(doc: &mut CadDocument, a: &Args) -> Result<String, String> {
    let rec = a.alignment(doc, "alignment")?;
    let id = a.int("arc_id")? as i32;
    let r = a.number("radius")?;
    let d = decoded(&rec)?;
    let data = raw::record_data_mut(doc, rec.handle).ok_or("record bytes missing")?;
    let rep = set_free_arc_radius(data, &d, id, r).map_err(|e| e.to_string())?;
    let note = after_geometry_edit(doc, rec.handle, true);
    Ok(format!(
        "{}: arc {id} R {} -> {}{}; {note}",
        label(&rec),
        rep.old_radius,
        rep.new_radius,
        if rep.graphics_stale {
            " (graphics stale until Civil 3D redraws)"
        } else {
            ""
        }
    ))
}

fn run_set_pis(doc: &mut CadDocument, a: &Args) -> Result<String, String> {
    let rec = a.alignment(doc, "alignment")?;
    let pis = a.points("pis")?;
    let d = decoded(&rec)?;
    let data = raw::record_data_mut(doc, rec.handle).ok_or("record bytes missing")?;
    let rep = set_pis(data, &d, &pis).map_err(|e| e.to_string())?;
    let note = after_geometry_edit(doc, rec.handle, true);
    Ok(format!(
        "{}: {} PI(s) set; {} tangent record(s), {} curve(s) restationed; {note}",
        label(&rec),
        pis.len(),
        rep.patches.tangent_records,
        rep.arcs_restationed
    ))
}

fn element_edit(
    doc: &mut CadDocument,
    rec: &RawRecord,
    r: Result<
        (Vec<u8>, usize, crate::aecc_element_edit::ElementEditReport),
        crate::aecc_element_edit::ElementEditError,
    >,
) -> Result<String, String> {
    let (bytes, hb, rep) = r.map_err(|e| e.to_string())?;
    put(doc, rec.handle, bytes, hb)?;
    let note = after_geometry_edit(doc, rec.handle, false);
    Ok(format!(
        "{}: {} element(s), L={:.4}{}{}; {note}",
        label(rec),
        rep.elements,
        rep.length,
        rep.added_id
            .map_or(String::new(), |i| format!(", new curve ID {i}")),
        if rep.floating.is_empty() {
            String::new()
        } else {
            format!(", now floating: {:?}", rep.floating)
        }
    ))
}

fn run_curve(doc: &mut CadDocument, a: &Args) -> Result<String, String> {
    let rec = a.alignment(doc, "alignment")?;
    let id = a.int("tangent_id")? as i32;
    let r = a.number("radius")?;
    let s = rec.handle ^ (rec.data.len() as u64) << 20;
    let res = crate::aecc_element_edit::add_free_curve(&rec, id, r, s);
    element_edit(doc, &rec, res)
}

fn run_remove(doc: &mut CadDocument, a: &Args) -> Result<String, String> {
    let rec = a.alignment(doc, "alignment")?;
    let id = a.int("element_id")? as i32;
    let s = rec.handle ^ (rec.data.len() as u64) << 20;
    let res = crate::aecc_element_edit::remove_element(&rec, id, s);
    element_edit(doc, &rec, res)
}

fn run_set_name(doc: &mut CadDocument, a: &Args) -> Result<String, String> {
    let rec = a.alignment(doc, "alignment")?;
    let old = raw::object_name(&rec.data).ok_or("alignment has no name")?;
    let new = match a.text("name")?.as_str() {
        "-" => old.clone(),
        n => n.to_string(),
    };
    let (mut bytes, mut hb) = string_stream::set_name(&rec.data, rec.handle_bits, &old, &new)
        .map_err(|e| e.to_string())?;
    if let Some(d) = a.get("--description") {
        let d = d.replace("\\n", "\n");
        (bytes, hb) = string_stream::set_description(&bytes, hb, &d).map_err(|e| e.to_string())?;
    }
    let n = bytes.len();
    put(doc, rec.handle, bytes, hb)?;
    Ok(format!(
        "[{:X}] '{old}' -> '{new}': {} -> {n} bytes",
        rec.handle,
        rec.data.len()
    ))
}

fn run_set_start(doc: &mut CadDocument, a: &Args) -> Result<String, String> {
    use crate::aecc_station::{move_start_station, set_start_station};
    let rec = a.alignment(doc, "alignment")?;
    let station = a.number("station")?;
    let clear = a.flag("--clear-speeds");
    let mut r2 = rec.clone();
    if clear {
        let (d, hb) =
            crate::aecc_design_speed::clear_design_speeds(&r2).map_err(|e| e.to_string())?;
        r2.data = d;
        r2.handle_bits = hb;
    }
    let moved = if clear {
        set_start_station(&r2, station)
    } else {
        move_start_station(&r2, station)
    };
    let (d, hb) = moved.ok_or("start station not found")?;
    let (d, hb) = handle_refs::trim_padding(&d, hb, rec.handle);
    put(doc, rec.handle, d, hb)?;
    Ok(format!(
        "{}: start station {station}{}",
        label(&rec),
        if clear { ", design speeds cleared" } else { "" }
    ))
}

fn run_set_speeds(doc: &mut CadDocument, a: &Args) -> Result<String, String> {
    let rec = a.alignment(doc, "alignment")?;
    let speeds: Vec<(f64, f64)> = a
        .text("speeds")?
        .split_whitespace()
        .map(|p| {
            let (st, sp) = p
                .split_once(':')
                .ok_or_else(|| format!("'{p}' must be station:speed"))?;
            Ok((
                st.parse().map_err(|_| format!("bad station '{st}'"))?,
                sp.parse().map_err(|_| format!("bad speed '{sp}'"))?,
            ))
        })
        .collect::<Result<_, String>>()?;
    let (d, hb) = crate::aecc_design_speed::set_design_speeds(&rec, &speeds, seed(rec.handle))
        .map_err(|e| e.to_string())?;
    let (d, hb) = handle_refs::trim_padding(&d, hb, rec.handle);
    put(doc, rec.handle, d, hb)?;
    Ok(format!("{}: {} design speed(s)", label(&rec), speeds.len()))
}

fn run_set_style(doc: &mut CadDocument, a: &Args) -> Result<String, String> {
    let rec = a.alignment(doc, "alignment")?;
    let style = style_handle(doc, &a.text("style")?)?;
    let old = set_alignment_style(doc, rec.handle, style).map_err(|e| e.to_string())?;
    Ok(format!("{}: style [{old:X}] -> [{style:X}]", label(&rec)))
}

fn run_style_component(doc: &mut CadDocument, a: &Args) -> Result<String, String> {
    let style = style_handle(doc, &a.text("style")?)?;
    let comp = a.text("component")?;
    let idx = aecc_style::component_index(&comp)
        .ok_or("component must be view:name (e.g. plan:curve) or 0-16")?;
    let on = match a.text("state")?.as_str() {
        "on" => true,
        "off" => false,
        _ => return Err("<state> must be on or off".into()),
    };
    let data = raw::record_data_mut(doc, style).ok_or("style record bytes missing")?;
    let was = aecc_style::set_component_visible(data, idx, on)
        .ok_or("can't locate display components in this style")?;
    let (view, name) = aecc_style::component_name(idx).ok_or("bad component")?;
    Ok(format!(
        "[{style:X}] {view:?} {name} (component {idx}): {was} -> {on}"
    ))
}

fn run_equations(doc: &mut CadDocument, a: &Args) -> Result<String, String> {
    let rec = a.alignment(doc, "alignment")?;
    let num = |s: &str| -> Result<f64, String> { s.parse().map_err(|_| format!("number: {s}")) };
    let specs: Vec<EquationSpec> = a
        .text("equations")?
        .split_whitespace()
        .map(|t| {
            let p: Vec<&str> = t.split(':').collect();
            if p.len() < 2 {
                return Err(format!("equation '{t}' must be raw:ahead[...]"));
            }
            let comment = p[2..].iter().find(|x| **x != "dec" && **x != "inc");
            Ok(EquationSpec {
                raw_station: num(p[0])?,
                ahead: num(p[1])?,
                increasing: p.get(2) != Some(&"dec"),
                comment: comment.map(|c| c.to_string()).unwrap_or_default(),
            })
        })
        .collect::<Result<_, String>>()?;
    let (bytes, hb) = set_station_equations(&rec, &specs, 0x5EED_0250u64 ^ rec.handle)
        .map_err(|e| e.to_string())?;
    put(doc, rec.handle, bytes, hb)?;
    Ok(format!(
        "{}: {} station equation(s)",
        label(&rec),
        specs.len()
    ))
}

fn run_masking(doc: &mut CadDocument, a: &Args) -> Result<String, String> {
    let rec = a.alignment(doc, "alignment")?;
    let num = |s: &str| -> Result<f64, String> { s.parse().map_err(|_| format!("number: {s}")) };
    let specs: Vec<MaskSpec> = a
        .text("regions")?
        .split_whitespace()
        .map(|t| {
            let p: Vec<&str> = t.split(':').collect();
            let (s, e) = p[0]
                .split_once('-')
                .ok_or_else(|| format!("region '{t}' must start with start-end"))?;
            let flags = &p[1..];
            Ok(MaskSpec {
                start: num(s)?,
                end: num(e)?,
                masked: !flags.contains(&"off"),
                lock_to_start: flags.contains(&"lockstart"),
                lock_to_end: flags.contains(&"lockend"),
                comment: flags
                    .iter()
                    .find(|x| !["off", "lockstart", "lockend"].contains(x))
                    .map(|c| c.to_string())
                    .unwrap_or_default(),
            })
        })
        .collect::<Result<_, String>>()?;
    let (bytes, hb) = set_masking_regions(&rec, &specs, 0x5EED_0250u64 ^ rec.handle)
        .map_err(|e| e.to_string())?;
    put(doc, rec.handle, bytes, hb)?;
    Ok(format!(
        "{}: {} masking region(s)",
        label(&rec),
        specs.len()
    ))
}

fn run_create(doc: &mut CadDocument, a: &Args) -> Result<String, String> {
    let donor_path = a.text("donor")?;
    let donor = raw::open(&donor_path).map_err(|e| format!("{donor_path}: {e}"))?;
    let name = a.text("name")?;
    let (pis, radii) = a.points_radii("pis")?;
    let style = a.get("--style").map(|s| style_handle(doc, s)).transpose()?;
    let template = a
        .get("--template")
        .map(|_| a.handle("--template"))
        .transpose()?;
    let alignment_type = a
        .get("--type")
        .map(|t| match t.to_ascii_lowercase().as_str() {
            "centerline" => Ok(aecc_build::TYPE_CENTERLINE),
            "misc" | "miscellaneous" => Ok(aecc_build::TYPE_MISCELLANEOUS),
            n => n
                .parse()
                .map_err(|_| "--type takes centerline, misc or a number".to_string()),
        })
        .transpose()?;
    let c = create_alignment(
        doc,
        &donor.doc,
        &name,
        &pis,
        &radii,
        seed(pis.len() as u64),
        CreateOptions {
            label_group: !a.flag("--no-label-group"),
            parcel_entry: !a.flag("--no-parcel-entry"),
            style,
            template,
            graphics: !a.flag("--no-graphics"),
            alignment_type,
        },
    )
    .map_err(|e| e.to_string())?;
    Ok(format!(
        "created [{:X}] '{}': style {:X}, companion {:X}, label group {:X}",
        c.alignment, c.name, c.style, c.companion, c.label_group
    ))
}

fn run_delete(doc: &mut CadDocument, a: &Args) -> Result<String, String> {
    let rec = a.alignment(doc, "alignment")?;
    let x = crate::aecc_delete::delete_alignment(doc, rec.handle).map_err(|e| e.to_string())?;
    Ok(format!(
        "deleted {}: label groups {:X?}, companion {:X} emptied",
        label(&rec),
        x.label_groups,
        x.companion
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenize_quotes() {
        assert_eq!(
            tokenize(r#"set_pis CL-1 "0,0 100,0" --x "a \"b\"""#).unwrap(),
            vec!["set_pis", "CL-1", "0,0 100,0", "--x", "a \"b\""]
        );
        assert_eq!(
            tokenize(r#"equations A """#).unwrap(),
            vec!["equations", "A", ""]
        );
        assert!(tokenize("a \"b").is_err());
    }

    #[test]
    fn quote_roundtrip() {
        for s in ["plain", "two words", "", "q\"x"] {
            assert_eq!(tokenize(&quote(s)).unwrap(), vec![s.to_string()]);
        }
    }

    #[test]
    fn parse_spec() {
        let c = find("set_start").unwrap();
        let w = |s: &str| tokenize(s).unwrap();
        let a = parse_args(c, &w("A1 100 --clear-speeds")).unwrap();
        assert_eq!(a.get("alignment"), Some("A1"));
        assert_eq!(a.number("station").unwrap(), 100.0);
        assert!(a.flag("--clear-speeds"));
        assert!(parse_args(c, &w("A1"))
            .unwrap_err()
            .contains("missing <station>"));
        assert!(parse_args(c, &w("A1 1 2")).is_err());
        assert!(parse_args(c, &w("A1 1 --bogus")).is_err());
    }

    #[test]
    fn points() {
        let (p, r) = parse_points("0,0 100,0,50 100,100").unwrap();
        assert_eq!(p.len(), 3);
        assert_eq!(r, vec![Some(50.0)]);
        assert!(parse_points("0,0,5 1,1").is_err());
    }

    #[test]
    fn every_command_has_help_and_unique_name() {
        let mut names: Vec<_> = commands().iter().map(|c| c.name).collect();
        names.sort();
        let n = names.len();
        names.dedup();
        assert_eq!(n, names.len());
        for c in commands() {
            assert!(!c.help.is_empty());
            assert!(help_text(Some(c.name)).is_ok());
        }
        assert_eq!(spec().len(), commands().len());
    }

    #[test]
    fn handles() {
        assert_eq!(parse_handle("#2A3"), Some(0x2A3));
        assert_eq!(parse_handle("0x2a3"), Some(0x2A3));
        assert_eq!(parse_handle("zz"), None);
    }
}
