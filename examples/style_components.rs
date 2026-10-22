//! List alignment styles' display-component visibility, or turn one
//! component on/off and save.
//!
//! Usage:
//!   cargo run --release --example style_components -- in.dwg
//!   cargo run --release --example style_components -- in.dwg out.dwg <style> <component> on|off
//!
//! `<style>` is a style name or hex handle; `<component>` is the 0-based
//! component index (0 = line, confirmed by S21/S22).

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
        println!("  [{h:X}] {name:<24} {flags}");
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
    println!("visible flags, component 0 (line) first:");
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
    let idx: usize = a[4]
        .parse()
        .unwrap_or_else(|_| fail("component must be a number"));
    let on = match a[5].as_str() {
        "on" => true,
        "off" => false,
        _ => fail("last argument must be on or off"),
    };
    let data = raw::record_data_mut(&mut o.doc, style).unwrap();
    let was = aecc_style::set_component_visible(data, idx, on)
        .unwrap_or_else(|| fail("can't locate display components in this style"));
    println!("[{style:X}] component {idx}: {was} -> {on}");
    acadrust::DwgWriter::write_to_file(&a[2], &o.doc).unwrap_or_else(|e| fail(e));
    let re =
        raw::open(&a[2]).unwrap_or_else(|e| fail(format!("written file does not read back: {e}")));
    println!("verify {}:", a[2]);
    list(&re.doc);
}
