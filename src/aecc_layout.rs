//! The JSON layout string of alignments whose spiral groups Civil 3D keeps
//! as separate elements (S177, S178; docs/alignment-object.md, "Stored as
//! separate elements, with a JSON layout").
//!
//! The string lists the layout as Civil 3D's group editor sees it: every
//! fixed element with the points that define it, and each group as one
//! object (its method and typed parameters), while the binary records hold
//! the solved pieces. The first piece of a group is a spiral with header
//! type `0x0D` that also stores the group's start point (MIST17); such a
//! spiral is only accepted with this string present (S189).
//!
//! [`Json`] parses and writes the string in Civil 3D's own format (tabs,
//! CRLF, numbers as C `%.17g`); [`compose`] builds it from element
//! records.

use crate::aecc_alignment::{Linked, KIND_FIXED};
use crate::aecc_elements::{arc_through, ArcDef, Element, ARC_THREE_POINT};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutError(pub String);

impl fmt::Display for LayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "layout JSON: {}", self.0)
    }
}

impl std::error::Error for LayoutError {}

fn err(m: impl Into<String>) -> LayoutError {
    LayoutError(m.into())
}

/// A JSON value with object keys in their written order.
#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Num(f64),
    Bool(bool),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    /// Parse a JSON text (the subset Civil 3D writes: no escapes beyond
    /// `\"` and `\\`, no null).
    pub fn parse(text: &str) -> Result<Json, LayoutError> {
        let mut p = Parser {
            s: text.as_bytes(),
            i: 0,
        };
        let v = p.value()?;
        p.ws();
        if p.i != p.s.len() {
            return Err(err(format!("trailing text at {}", p.i)));
        }
        Ok(v)
    }

    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(kv) => kv.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut Json> {
        match self {
            Json::Obj(kv) => kv.iter_mut().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    /// Write in Civil 3D's format: tab indentation, CRLF line ends,
    /// arrays of numbers on one line (`[ 1, 2 ]`), numbers as `%.17g`.
    pub fn to_civil3d(&self) -> String {
        let mut out = String::new();
        self.write(&mut out, 0);
        out
    }

    fn write(&self, out: &mut String, depth: usize) {
        let indent = |out: &mut String, d: usize| out.extend(std::iter::repeat_n('\t', d));
        match self {
            Json::Num(v) => out.push_str(&g17(*v)),
            Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Json::Str(s) => {
                out.push('"');
                for c in s.chars() {
                    if c == '"' || c == '\\' {
                        out.push('\\');
                    }
                    out.push(c);
                }
                out.push('"');
            }
            Json::Arr(items) if items.iter().all(|v| matches!(v, Json::Num(_))) => {
                out.push_str("[ ");
                for (i, v) in items.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    v.write(out, depth);
                }
                out.push_str(" ]");
            }
            Json::Arr(items) => {
                out.push_str("[\r\n");
                for (i, v) in items.iter().enumerate() {
                    indent(out, depth + 1);
                    v.write(out, depth + 1);
                    out.push_str(if i + 1 < items.len() { ",\r\n" } else { "\r\n" });
                }
                indent(out, depth);
                out.push(']');
            }
            Json::Obj(kv) => {
                out.push_str("{\r\n");
                for (i, (k, v)) in kv.iter().enumerate() {
                    indent(out, depth + 1);
                    Json::Str(k.clone()).write(out, depth + 1);
                    out.push_str(": ");
                    v.write(out, depth + 1);
                    out.push_str(if i + 1 < kv.len() { ",\r\n" } else { "\r\n" });
                }
                indent(out, depth);
                out.push('}');
            }
        }
    }
}

/// C's `%.17g`: 17 significant digits, trailing zeros removed, exponent
/// form below 1e-4 and from 1e17.
pub fn g17(v: f64) -> String {
    if v == 0.0 {
        return if v.is_sign_negative() { "-0" } else { "0" }.into();
    }
    let e = format!("{v:.16e}");
    let (mant, exp) = e.split_once('e').unwrap();
    let exp: i32 = exp.parse().unwrap();
    let neg = mant.starts_with('-');
    let digits: String = mant.chars().filter(|c| c.is_ascii_digit()).collect();
    let trim = |s: String| -> String {
        if s.contains('.') {
            s.trim_end_matches('0').trim_end_matches('.').to_string()
        } else {
            s
        }
    };
    let body = if !(-4..17).contains(&exp) {
        let m = trim(format!("{}.{}", &digits[..1], &digits[1..]));
        format!("{m}e{}{:02}", if exp < 0 { '-' } else { '+' }, exp.abs())
    } else if exp >= 0 {
        let k = exp as usize + 1;
        trim(format!("{}.{}", &digits[..k], &digits[k..]))
    } else {
        trim(format!("0.{}{}", "0".repeat((-exp - 1) as usize), digits))
    };
    if neg {
        format!("-{body}")
    } else {
        body
    }
}

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
}

impl Parser<'_> {
    fn ws(&mut self) {
        while self.i < self.s.len() && self.s[self.i].is_ascii_whitespace() {
            self.i += 1;
        }
    }

    fn eat(&mut self, c: u8) -> Result<(), LayoutError> {
        self.ws();
        if self.s.get(self.i) == Some(&c) {
            self.i += 1;
            Ok(())
        } else {
            Err(err(format!("expected '{}' at {}", c as char, self.i)))
        }
    }

    fn string(&mut self) -> Result<String, LayoutError> {
        self.eat(b'"')?;
        let mut out = Vec::new();
        while let Some(&c) = self.s.get(self.i) {
            self.i += 1;
            match c {
                b'"' => return String::from_utf8(out).map_err(|e| err(e.to_string())),
                b'\\' => {
                    out.push(*self.s.get(self.i).ok_or_else(|| err("bad escape"))?);
                    self.i += 1;
                }
                _ => out.push(c),
            }
        }
        Err(err("unterminated string"))
    }

    fn value(&mut self) -> Result<Json, LayoutError> {
        self.ws();
        match self.s.get(self.i) {
            Some(b'{') => {
                self.i += 1;
                let mut kv = Vec::new();
                self.ws();
                if self.s.get(self.i) == Some(&b'}') {
                    self.i += 1;
                    return Ok(Json::Obj(kv));
                }
                loop {
                    let k = self.string()?;
                    self.eat(b':')?;
                    kv.push((k, self.value()?));
                    self.ws();
                    match self.s.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b'}') => {
                            self.i += 1;
                            return Ok(Json::Obj(kv));
                        }
                        _ => return Err(err(format!("bad object at {}", self.i))),
                    }
                }
            }
            Some(b'[') => {
                self.i += 1;
                let mut items = Vec::new();
                self.ws();
                if self.s.get(self.i) == Some(&b']') {
                    self.i += 1;
                    return Ok(Json::Arr(items));
                }
                loop {
                    items.push(self.value()?);
                    self.ws();
                    match self.s.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b']') => {
                            self.i += 1;
                            return Ok(Json::Arr(items));
                        }
                        _ => return Err(err(format!("bad array at {}", self.i))),
                    }
                }
            }
            Some(b'"') => Ok(Json::Str(self.string()?)),
            Some(b't') if self.s[self.i..].starts_with(b"true") => {
                self.i += 4;
                Ok(Json::Bool(true))
            }
            Some(b'f') if self.s[self.i..].starts_with(b"false") => {
                self.i += 5;
                Ok(Json::Bool(false))
            }
            Some(_) => {
                let start = self.i;
                while self
                    .s
                    .get(self.i)
                    .is_some_and(|c| c.is_ascii_digit() || b"+-.eE".contains(c))
                {
                    self.i += 1;
                }
                std::str::from_utf8(&self.s[start..self.i])
                    .ok()
                    .and_then(|t| t.parse().ok())
                    .map(Json::Num)
                    .ok_or_else(|| err(format!("bad number at {start}")))
            }
            None => Err(err("unexpected end")),
        }
    }
}

/// A group method whose pieces Civil 3D keeps apart (S177, S178): piece
/// pattern (`S` spiral, `C` curve), constraint type, parameter names.
type Method = (&'static str, i32, &'static [&'static str]);

const GROUP_METHODS: [Method; 3] = [
    // Free Reverse Spiral-Spiral-Curve-Spiral-Spiral (between two curves).
    (
        "SSCSS",
        2569,
        &[
            "Spiral1Length",
            "Spiral2Length",
            "ArcRadius",
            "Spiral3Length",
            "Spiral4Length",
        ],
    ),
    // Free Reverse Spiral-Curve-Spiral-Curve (from curve, end entity).
    (
        "SSCS",
        2561,
        &[
            "Spiral1Length",
            "Spiral2Length",
            "ArcRadius",
            "Spiral3Length",
        ],
    ),
    // Free Reverse Spiral-Curve-Spiral-Spiral (from entity, end curve).
    (
        "SCSS",
        2565,
        &[
            "Spiral1Length",
            "ArcRadius",
            "Spiral3Length",
            "Spiral4Length",
        ],
    ),
];

/// `ConstraintLocked` of every object: 2 with the parameter lock on, 0
/// with it off (S202: "Lock all parameter constraints" off).
fn constraint_locked(locked: bool) -> f64 {
    if locked {
        2.0
    } else {
        0.0
    }
}

fn num(v: impl Into<f64>) -> Json {
    Json::Num(v.into())
}

fn obj(kv: Vec<(&str, Json)>) -> Json {
    Json::Obj(kv.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
}

fn pt(p: crate::alignment::Point2) -> Json {
    Json::Arr(vec![num(p.x), num(p.y)])
}

/// A group in the layout: one of the sampled methods (by piece pattern
/// `"SSCSS"`, `"SSCS"` or `"SCSS"`), the fixed elements before and after
/// it (0 at an end of the alignment), and its typed parameters in the
/// method's order (Spiral1, Spiral2, ArcRadius, Spiral3, Spiral4, leaving
/// out what the method lacks).
#[derive(Debug, Clone, PartialEq)]
pub struct Group {
    pub method: &'static str,
    pub previous: i32,
    pub next: i32,
    pub params: Vec<f64>,
}

/// The JSON parameter names of `method` (`"SSCSS"`, `"SSCS"`, `"SCSS"`),
/// in order.
pub fn group_parameters(method: &str) -> Option<&'static [&'static str]> {
    GROUP_METHODS
        .iter()
        .find(|(p, _, _)| *p == method)
        .map(|(_, _, n)| *n)
}

/// Build the layout JSON from the fixed elements (fixed lines and
/// three-point curves, as records) and the groups between them. The
/// groups take the IDs after the highest fixed ID, as Civil 3D numbers
/// them; a fixed element's link to a group's neighbour is written as a
/// link to the group. `locked`: the alignment's parameter lock.
pub fn layout(fixed: &[Element], groups: &[Group], locked: bool) -> Result<Json, LayoutError> {
    let top = fixed.iter().map(|e| e.id()).max().unwrap_or(0);
    let group_id = |g: usize| top + 1 + g as i32;
    let mut objects: Vec<(i32, Json)> = Vec::new();
    for e in fixed {
        let (rg, ct, params) = match e {
            Element::Tangent(t) if t.kind == KIND_FIXED => (
                0,
                0,
                vec![
                    ("PassThroughPoint1", pt(t.start)),
                    ("PassThroughPoint2", pt(t.end)),
                ],
            ),
            Element::Arc(a) if a.params.n2 == ARC_THREE_POINT => {
                let ArcDef::ThreePoint {
                    start, mid, end, ..
                } = a.def
                else {
                    return Err(err(format!("curve {} is not by three points", a.id)));
                };
                (
                    1,
                    2,
                    vec![
                        ("PassThroughPoint1", pt(start)),
                        ("PassThroughPoint2", pt(mid)),
                        ("PassThroughPoint3", pt(end)),
                    ],
                )
            }
            _ => {
                return Err(err(format!(
                    "element {}: only fixed lines and three-point curves are known in the JSON",
                    e.id()
                )))
            }
        };
        let next = groups
            .iter()
            .position(|g| g.previous == e.id())
            .map_or(e.after(), group_id);
        let previous = groups
            .iter()
            .position(|g| g.next == e.id())
            .map_or(e.before(), group_id);
        objects.push((
            e.id(),
            obj(vec![
                ("RGObjectType", num(rg)),
                ("Id", num(e.id())),
                ("PreviousId", num(previous)),
                ("NextId", num(next)),
                ("ConstraintLocked", num(constraint_locked(locked))),
                ("HasAnchorPoint", Json::Bool(false)),
                ("ConstraintType", num(ct)),
                ("ConstraintParameters", obj(params)),
            ]),
        ));
    }
    for (g, group) in groups.iter().enumerate() {
        // Civil 3D needs an entity on both sides (owner; S223 → S224: a
        // group at an end is dropped).
        if group.previous == 0 || group.next == 0 {
            return Err(err(format!(
                "group {} at an end of the alignment: Civil 3D needs an element on both sides",
                group.method
            )));
        }
        let (_, ct, names) = GROUP_METHODS
            .iter()
            .find(|(p, _, _)| *p == group.method)
            .ok_or_else(|| err(format!("unknown group method {}", group.method)))?;
        if names.len() != group.params.len() {
            return Err(err(format!(
                "{} takes {} parameters, not {}",
                group.method,
                names.len(),
                group.params.len()
            )));
        }
        let id = group_id(g);
        objects.push((
            id,
            obj(vec![
                ("RGObjectType", num(7)),
                ("Id", num(id)),
                ("PreviousId", num(group.previous)),
                ("NextId", num(group.next)),
                ("ConstraintLocked", num(constraint_locked(locked))),
                ("ConstraintType", num(*ct)),
                ("SpiralType", num(256)),
                (
                    "ConstraintParameters",
                    obj(names
                        .iter()
                        .zip(&group.params)
                        .map(|(n, v)| (*n, num(*v)))
                        .collect()),
                ),
            ]),
        ));
    }
    objects.sort_by_key(|(id, _)| *id);
    let first = match groups.iter().position(|g| g.previous == 0) {
        Some(g) => group_id(g),
        None => fixed
            .iter()
            .find(|e| e.before() == 0)
            .map(|e| e.id())
            .ok_or_else(|| err("no first element"))?,
    };
    Ok(obj(vec![
        ("JsonVersion", num(1)),
        ("FirstId", num(first)),
        (
            "RGObjects",
            Json::Arr(objects.into_iter().map(|(_, o)| o).collect()),
        ),
    ]))
}

/// Build the layout JSON for `els` (element records in any order, linked
/// by before/after IDs): fixed lines and three-point curves as themselves,
/// each run of pieces starting at a group-member spiral (`member` set) as
/// one group with the parameters measured from its pieces ([`layout`]).
/// The pieces' IDs must be higher than the groups' (as Civil 3D numbers
/// them).
pub fn compose(els: &[Element], locked: bool) -> Result<Json, LayoutError> {
    let by_id = |id: i32| els.iter().find(|e| e.id() == id);
    let mut pieces: Vec<i32> = Vec::new();
    let mut groups = Vec::new();
    for start in els
        .iter()
        .filter(|e| matches!(e, Element::Spiral(s) if s.member.is_some()))
    {
        let mut run = vec![start];
        while let Some(n) = by_id(run.last().unwrap().after()) {
            if run.len() == 5 {
                break;
            }
            run.push(n);
        }
        let code: String = run
            .iter()
            .map(|e| match e {
                Element::Spiral(_) => 'S',
                Element::Arc(_) => 'C',
                _ => 'L',
            })
            .collect();
        let method = GROUP_METHODS
            .iter()
            .find(|(p, _, _)| code.starts_with(p))
            .ok_or_else(|| err(format!("pieces {code} after spiral {}", start.id())))?;
        run.truncate(method.0.len());
        let params = run
            .iter()
            .map(|e| match e {
                Element::Spiral(s) => Ok(s.length),
                Element::Arc(a) => match a.def {
                    ArcDef::ThreePoint {
                        start, mid, end, ..
                    } => arc_through(start, mid, end)
                        .map(|g| g.radius)
                        .ok_or_else(|| err(format!("curve {} is straight", a.id))),
                    _ => Err(err(format!("curve {} is not by three points", a.id))),
                },
                _ => Err(err("unexpected piece")),
            })
            .collect::<Result<Vec<_>, _>>()?;
        pieces.extend(run.iter().map(|e| e.id()));
        groups.push(Group {
            method: method.0,
            previous: run[0].before(),
            next: run.last().unwrap().after(),
            params,
        });
    }
    let fixed: Vec<Element> = els
        .iter()
        .filter(|e| !pieces.contains(&e.id()))
        .cloned()
        .collect();
    let top = fixed.iter().map(|e| e.id()).max().unwrap_or(0);
    if let Some(p) = pieces.iter().find(|&&p| p <= top + groups.len() as i32) {
        return Err(err(format!("piece {p} is not numbered after the groups")));
    }
    layout(&fixed, &groups, locked)
}

/// Index of the JSON slot in the string stream: after the class name and
/// the two description copies (S177, S178).
pub const JSON_SLOT: usize = 3;

/// Write `json` as the alignment's layout string: replace the existing
/// one, or insert it at [`JSON_SLOT`] (the strings after it move down one,
/// as in Civil 3D's records). Returns the new bytes and handle-stream size.
pub fn set_layout(
    rec: &crate::raw::RawRecord,
    json: &Json,
) -> Result<(Vec<u8>, usize), LayoutError> {
    use crate::string_stream as ss;
    let text = json.to_civil3d();
    let e = |x: ss::StringStreamError| err(x.to_string());
    let strings = ss::parse(&rec.data, rec.handle_bits).map_err(e)?.strings;
    match strings
        .iter()
        .position(|s| crate::raw::is_layout_json(&s.text))
    {
        Some(i) => ss::replace(&rec.data, rec.handle_bits, i, &text).map_err(e),
        None => {
            let (d, hb) = ss::insert_empty(&rec.data, rec.handle_bits, JSON_SLOT).map_err(e)?;
            ss::replace(&d, hb, JSON_SLOT, &text).map_err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn g17_matches_c() {
        assert_eq!(g17(47.0), "47");
        assert_eq!(g17(1596.536), "1596.5360000000001");
        assert_eq!(g17(1881.277), "1881.277");
        assert_eq!(g17(-14.021), "-14.021000000000001");
        assert_eq!(g17(1000.0000000000001), "1000.0000000000001");
        assert_eq!(g17(0.5), "0.5");
        assert_eq!(g17(1e-5), "1.0000000000000001e-05");
        assert_eq!(g17(1e20), "1e+20");
        assert_eq!(g17(0.001), "0.001");
    }
}
