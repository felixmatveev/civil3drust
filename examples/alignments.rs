//! List the alignments in a DWG with their tangent and arc elements,
//! decoded from the element records (see docs/alignment-object.md).
//!
//! Usage: cargo run --release --example alignments -- file.dwg

use civil3drust::aecc_alignment::{record_name, Linked};
use civil3drust::aecc_elements::{decode_elements, Element, Segment};
use civil3drust::raw;

fn main() {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: alignments <file.dwg>");
        std::process::exit(2);
    };
    let opened = raw::open(&path).unwrap_or_else(|e| {
        eprintln!("failed to read {path}: {e}");
        std::process::exit(1);
    });
    if let Some(r) = &opened.failsafe_reason {
        eprintln!("note: read in failsafe mode; some records were dropped ({r})");
    }
    let recs = raw::records(&opened.doc, "AECC_ALIGNMENT");
    println!("{path}: {} AECC_ALIGNMENT record(s)", recs.len());
    for rec in &recs {
        let d = match decode_elements(rec) {
            Ok(d) => d,
            Err(e) => {
                println!(
                    "\n[{:X}] {}  not decoded: {e}",
                    rec.handle,
                    record_name(&rec.data).unwrap_or_else(|| "?".into())
                );
                continue;
            }
        };
        let start = d.stored_start_station.unwrap_or(0.0);
        let count = |f: fn(&Segment) -> bool| d.elements.iter().filter(|e| f(&e.segment)).count();
        let arcs = count(|s| matches!(s, Segment::Arc { .. }));
        let spirals = count(|s| matches!(s, Segment::Spiral { .. }));
        println!(
            "\n[{:X}] {}  {} element(s) ({} tangent, {} arc, {} spiral), length {:.4}, stations {:.2}-{:.2}{}",
            d.handle,
            d.name.as_deref().unwrap_or("?"),
            d.elements.len(),
            d.elements.len() - arcs - spirals,
            arcs,
            spirals,
            d.length(),
            start,
            start + d.length(),
            if d.stored_start_station.is_none() {
                " (start station not found; shown from 0)"
            } else {
                ""
            }
        );
        // Offset alignments (S255): the parameters and the parent.
        if let Ok(Some(o)) = civil3drust::aecc_offset::offset_info(rec) {
            let parent = civil3drust::aecc_alignment_refs::offset_parent(&opened.doc, rec)
                .map(|p| format!("{p:X}"))
                .unwrap_or_else(|| "?".into());
            println!(
                "  offset alignment of [{parent}]: {:.3} {:?}, parent stations {:.2}-{:.2}, own {:.2}-{:.2}",
                o.offset.abs(),
                o.side,
                o.parent_start,
                o.parent_end,
                o.start,
                o.end
            );
        }
        // Station equations (S243, S247): stations below are shown as
        // Civil 3D shows them, raw ones in brackets.
        let eqs = civil3drust::aecc_station_refs::station_equations(rec).unwrap_or_default();
        let show = |raw: f64| civil3drust::aecc_station_refs::display_station(raw, &eqs);
        let fmt = |raw: f64| {
            if eqs.is_empty() {
                format!("{raw:.2}")
            } else {
                format!("{:.2} [raw {raw:.2}]", show(raw))
            }
        };
        for (i, e) in eqs.iter().enumerate() {
            println!(
                "  station equation {}: raw back {:.2}, back {:.2}, ahead {:.2}, {}",
                i + 1,
                e.raw_station,
                civil3drust::aecc_station_refs::station_back(i, &eqs).unwrap_or(e.back),
                e.ahead,
                if e.increasing {
                    "increasing"
                } else {
                    "decreasing"
                }
            );
        }
        if !eqs.is_empty() {
            println!(
                "  stations with equations: {} to {}",
                fmt(start),
                fmt(start + d.length())
            );
        }
        let mut sta = start;
        for e in &d.elements {
            let len = e.segment.length();
            let (s, t) = (e.segment.start(), e.segment.end());
            let what = match (&e.segment, &e.record) {
                (Segment::Line { .. }, Element::LineSpiral(_)) => {
                    "tangent (line-spiral group, line)".to_string()
                }
                (Segment::Line { .. }, _) => "tangent".to_string(),
                (Segment::Spiral { geometry: g, .. }, rec) => format!(
                    "spiral ({}) R {:.4} -> {:.4} {}",
                    match rec {
                        Element::Spiral(sp) => match sp.kind {
                            0 => "inward",
                            1 => "outward",
                            2 => "compound",
                            5 => "offset",
                            6 | 8 => "placed",
                            _ => "?",
                        },
                        Element::FreeSpiral(sp) => {
                            if sp.code == civil3drust::aecc_elements::FREE_SPIRAL_INCURVE {
                                "free, incurve"
                            } else {
                                "free, outcurve"
                            }
                        }
                        Element::Group(_) => "group",
                        _ => "line-spiral group",
                    },
                    g.start_radius,
                    g.radius,
                    if g.counter_clockwise { "left" } else { "right" }
                ),
                (Segment::Arc { geometry: g, .. }, rec) => {
                    let kind = match rec {
                        Element::Arc(a) => a.def.describe(),
                        _ => "?",
                    };
                    format!(
                        "arc ({kind}) R={:.4} {} delta={:.4}° center ({:.4}, {:.4})",
                        g.radius,
                        if g.counter_clockwise { "left" } else { "right" },
                        g.sweep.to_degrees(),
                        g.center.x,
                        g.center.y
                    )
                }
            };
            println!(
                "  id {:>2} (before {}, after {})  {}  ({:.4}, {:.4}) -> ({:.4}, {:.4})  L={:.4}  sta {}-{}",
                e.id(),
                e.before(),
                e.after(),
                what,
                s.x,
                s.y,
                t.x,
                t.y,
                len,
                fmt(sta),
                fmt(sta + len)
            );
            sta += len;
        }
        for g in &d.layout_groups {
            println!(
                "  JSON group {} ({}; type {}, between {} and {}): {}",
                g.id,
                if g.solved {
                    "solved by civil3drust, its pieces are listed above"
                } else {
                    "not solved, stations stop before it"
                },
                g.constraint_type,
                g.previous,
                g.next,
                g.params
                    .iter()
                    .map(|(k, v)| format!("{k} {v}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            for e in &g.following {
                let (s, t) = (e.segment.start(), e.segment.end());
                println!(
                    "    id {:>2} (before {}, after {})  {}  ({:.4}, {:.4}) -> ({:.4}, {:.4})  L={:.4}  (stations unknown)",
                    e.id(),
                    e.before(),
                    e.after(),
                    if matches!(e.segment, Segment::Line { .. }) { "tangent" } else { "arc" },
                    s.x,
                    s.y,
                    t.x,
                    t.y,
                    e.segment.length()
                );
            }
        }
        for (k, piece) in d.detached.iter().enumerate() {
            println!(
                "  detached piece {} (ignored by Civil 3D, no stations):",
                k + 1
            );
            for e in piece {
                let (s, t) = (e.segment.start(), e.segment.end());
                println!(
                    "    id {:>2} (before {}, after {})  {}  ({:.4}, {:.4}) -> ({:.4}, {:.4})  L={:.4}",
                    e.id(),
                    e.before(),
                    e.after(),
                    if matches!(e.segment, Segment::Line { .. }) { "tangent" } else { "arc" },
                    s.x,
                    s.y,
                    t.x,
                    t.y,
                    e.segment.length()
                );
            }
        }
    }
}
