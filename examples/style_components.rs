//! List alignment styles' display-component visibility, or turn one
//! component on/off and save.
//!
//! Usage:
//!   cargo run --release --example style_components -- in.dwg
//!   cargo run --release --example style_components -- in.dwg out.dwg <style> <component> on|off
//!
//! `<style>` is a style name or hex handle; `<component>` is `view:name`
//! (`plan:line`, `plan:curve`, `model:spiral`, `section`, …) or the 0-based
//! index (0–7 plan, 8–15 model, 16 section).

use civil3drust::aecc_alignment_refs::alignment_styles;
use civil3drust::{aecc_style, raw};

fn fail(msg: impl std::fmt::Display) -> ! {
    eprintln!("error: {msg}");
    std::process::exit(1);
}

fn list(doc: &acadrust::document::CadDocument) {
    for (h, name) in alignment_styles(doc) {
        let rec = raw::records(doc, "AECC_ALIGNMENT_STYLE")
            .into_iter()
            .find(|r| r.handle == h)
            .unwrap();
        let flags = aecc_style::flags_string(&rec.data).unwrap_or_else(|| "?".into());
        let grouped = if flags.len() == 17 {
            format!(
                "plan {} model {} section {}",
                &flags[..8],
                &flags[8..16],
                &flags[16..]
            )
        } else {
            flags
        };
        println!("  [{h:X}] {name:<24} {grouped}");
    }
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() != 2 && a.len() != 6 {
        eprintln!("usage: style_components <in.dwg> [<out.dwg> <style> <component> on|off]");
        std::process::exit(2);
    }
    let mut o = raw::open(&a[1]).unwrap_or_else(|e| fail(e));
    if let Some(r) = &o.failsafe_reason {
        eprintln!(
            "WARNING: acadrust read this file in failsafe mode; records it could not parse\n\
             will be MISSING from the output. ({r})"
        );
    }
    println!("visible flags: Line, Curve, Spiral, Arrow, Line/Curve/Tangent Extensions, Warning Symbol per view:");
    if a.len() == 2 {
        list(&o.doc);
        return;
    }
    let style = alignment_styles(&o.doc)
        .into_iter()
        .find(|(h, n)| {
            n == &a[3] || Some(*h) == u64::from_str_radix(a[3].trim_start_matches("0x"), 16).ok()
        })
        .map(|s| s.0)
        .unwrap_or_else(|| fail(format!("no alignment style '{}'", a[3])));
    let idx = aecc_style::component_index(&a[4])
        .unwrap_or_else(|| fail("component must be view:name (e.g. plan:curve) or 0-16"));
    let on = match a[5].as_str() {
        "on" => true,
        "off" => false,
        _ => fail("last argument must be on or off"),
    };
    let data = raw::record_data_mut(&mut o.doc, style).unwrap();
    let was = aecc_style::set_component_visible(data, idx, on)
        .unwrap_or_else(|| fail("can't locate display components in this style"));
    let (view, name) = aecc_style::component_name(idx).unwrap();
    println!("[{style:X}] {view:?} {name} (component {idx}): {was} -> {on}");
    acadrust::DwgWriter::write_to_file(&a[2], &o.doc).unwrap_or_else(|e| fail(e));
    let re =
        raw::open(&a[2]).unwrap_or_else(|e| fail(format!("written file does not read back: {e}")));
    println!("verify {}:", a[2]);
    list(&re.doc);
}
