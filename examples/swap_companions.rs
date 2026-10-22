//! Experiment: swap the parcel companions of two alignments (S34).
//!
//! Usage: cargo run --release --example swap_companions -- in.dwg out.dwg <alignment A> <alignment B>
//!
//! Alignment A's two companion references are pointed at B's companion and
//! vice versa, and each companion's back-pointer is swapped too, so the
//! links stay consistent while each parcel entry holds the other
//! alignment's geometry. Same-size patches only.

use civil3drust::aecc_alignment_refs::alignment_refs;
use civil3drust::{handle_refs, raw};

fn fail(msg: impl std::fmt::Display) -> ! {
    eprintln!("error: {msg}");
    std::process::exit(1);
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() != 5 {
        eprintln!("usage: swap_companions <in.dwg> <out.dwg> <alignment A> <alignment B>");
        std::process::exit(2);
    }
    let mut o = raw::open(&a[1]).unwrap_or_else(|e| fail(e));
    let find = |doc: &acadrust::document::CadDocument, key: &str| {
        raw::records(doc, "AECC_ALIGNMENT")
            .into_iter()
            .find(|r| {
                raw::object_name(&r.data).as_deref() == Some(key)
                    || u64::from_str_radix(key, 16).ok() == Some(r.handle)
            })
            .unwrap_or_else(|| fail(format!("no alignment '{key}'")))
    };
    let (ra, rb) = (find(&o.doc, &a[3]), find(&o.doc, &a[4]));
    let ca = alignment_refs(&ra)
        .unwrap_or_else(|| fail("A: unknown layout"))
        .companion
        .handle;
    let cb = alignment_refs(&rb)
        .unwrap_or_else(|| fail("B: unknown layout"))
        .companion
        .handle;
    if ca == 0 || cb == 0 {
        fail("both alignments need a companion");
    }
    // Alignments: companion refs A->cb, B->ca.
    for (rec, from, to) in [(&ra, ca, cb), (&rb, cb, ca)] {
        let refs = handle_refs::of_record(rec);
        let data = raw::record_data_mut(&mut o.doc, rec.handle).unwrap();
        for r in refs.iter().filter(|r| r.handle == from) {
            if !handle_refs::retarget_any(data, r, rec.handle, to) {
                fail(format!("can't retarget {:X} in {:X}", from, rec.handle));
            }
        }
    }
    // Companions: back-pointer ca->B, cb->A.
    for (comp, from, to) in [(ca, ra.handle, rb.handle), (cb, rb.handle, ra.handle)] {
        let rec = raw::records(&o.doc, "AECC_FEATURE_STYLE")
            .into_iter()
            .find(|r| r.handle == comp)
            .unwrap();
        let refs = handle_refs::of_record(&rec);
        let data = raw::record_data_mut(&mut o.doc, comp).unwrap();
        for r in refs.iter().filter(|r| r.handle == from) {
            if !handle_refs::retarget_any(data, r, comp, to) {
                fail(format!("can't retarget {:X} in {:X}", from, comp));
            }
        }
    }
    println!(
        "[{:X}] now uses companion {:X}; [{:X}] now uses {:X}",
        ra.handle, cb, rb.handle, ca
    );
    acadrust::DwgWriter::write_to_file(&a[2], &o.doc).unwrap_or_else(|e| fail(e));
    let re = raw::open(&a[2]).unwrap_or_else(|e| fail(format!("does not read back: {e}")));
    for rec in raw::records(&re.doc, "AECC_ALIGNMENT") {
        let c = alignment_refs(&rec)
            .map(|r| r.companion.handle)
            .unwrap_or(0);
        let back = raw::records(&re.doc, "AECC_FEATURE_STYLE")
            .into_iter()
            .find(|r| r.handle == c)
            .and_then(|r| handle_refs::of_record(&r).get(1).map(|x| x.handle))
            .unwrap_or(0);
        println!(
            "  [{:X}] {} -> companion {:X} -> back to {:X}",
            rec.handle,
            raw::object_name(&rec.data).unwrap_or_default(),
            c,
            back
        );
    }
}
