//! Acceptance (D-049) for generated alignments: Civil 3D opened S188 and
//! S190 without RECOVER, AUDIT clean, and its re-saves S195 and S196 read
//! back to the same model — every element with the same ID, links and
//! geometry. Civil 3D only sets the "solved" flag and rounds in the last
//! bits.

use civil3drust::aecc_alignment::Linked;
use civil3drust::aecc_elements::decode_elements;
use civil3drust::raw;

fn same_model(written: &str, resaved: &str, alignments: usize) {
    let a = raw::open(format!("samples/{written}")).unwrap();
    let b = raw::open(format!("samples/{resaved}")).unwrap();
    let rb = raw::records(&b.doc, "AECC_ALIGNMENT");
    let mut n = 0;
    for r in raw::records(&a.doc, "AECC_ALIGNMENT") {
        let s = rb
            .iter()
            .find(|q| q.handle == r.handle)
            .unwrap_or_else(|| panic!("{resaved}: {:X} missing", r.handle));
        let (da, db) = (decode_elements(&r).unwrap(), decode_elements(s).unwrap());
        assert_eq!(da.elements.len(), db.elements.len(), "{:X}", r.handle);
        for (x, y) in da.elements.iter().zip(&db.elements) {
            assert_eq!(
                (x.id(), x.before(), x.after()),
                (y.id(), y.before(), y.after()),
                "{:X}",
                r.handle
            );
            let (lx, ly) = (x.segment.length(), y.segment.length());
            assert!(
                (lx - ly).abs() < 1e-9 * (1.0 + lx),
                "{:X}: {lx} vs {ly}",
                r.handle
            );
            let (sx, sy) = (x.segment.start(), y.segment.start());
            assert!(
                (sx.x - sy.x).abs() + (sx.y - sy.y).abs() < 1e-6,
                "{:X}",
                r.handle
            );
        }
        n += 1;
    }
    assert_eq!(n, alignments);
}

#[test]
fn s188_reads_back_after_civil3d_save() {
    same_model(
        "S188_S24_generated_chains.dwg",
        "S195_S188_civ3d_resaved.dwg",
        44,
    );
}

#[test]
fn s190_reads_back_after_civil3d_save() {
    same_model(
        "S190_S24_generated_plain_members.dwg",
        "S196_S190_civ3d_resaved.dwg",
        7,
    );
}
