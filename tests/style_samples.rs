//! Checks against samples S18–S20 (alignment style reference).

use civil3drust::aecc_alignment_refs::{alignment_refs, alignment_styles, set_alignment_style};
use civil3drust::raw;

fn align(doc: &acadrust::document::CadDocument) -> raw::RawRecord {
    raw::records(doc, "AECC_ALIGNMENT").remove(0)
}

#[test]
fn s18_s19_style_reference_and_s20_regeneration() {
    let s18 = raw::open("samples/S18_r50_NO_STYLE.dwg").unwrap();
    let s19 = raw::open("samples/S19_r50_TEST_STYLE.dwg").unwrap();
    let s20 = raw::open("samples/S20_S18_TEST_STYLE_generated.dwg").unwrap();
    let styles = alignment_styles(&s18.doc);
    let find = |n: &str| styles.iter().find(|s| s.1 == n).unwrap().0;
    let (no, test) = (find("NO STYLE"), find("TEST STYLE"));
    assert_eq!((no, test), (0x8DE7, 0x8DFE));

    let a18 = align(&s18.doc);
    let a19 = align(&s19.doc);
    let r18 = alignment_refs(&a18).unwrap();
    assert_eq!(r18.style.handle, no);
    assert_eq!(r18.companion.handle, a18.handle - 1);
    assert_eq!(alignment_refs(&a19).unwrap().style.handle, test);
    let ranges = raw::changed_bit_ranges(&a18.data, &a19.data, 0);
    assert!(!ranges.is_empty());
    assert!(ranges
        .iter()
        .all(|&(s, e)| s >= r18.style.bit && e <= r18.style.bit + 24));

    let mut doc = s18.doc;
    assert_eq!(set_alignment_style(&mut doc, a18.handle, test), Ok(no));
    assert_eq!(align(&doc).data, a19.data);
    assert_eq!(align(&s20.doc).data, a19.data);
}

#[test]
fn s21_s22_line_component_flag_and_s23_regeneration() {
    use civil3drust::aecc_style;
    let style = |p: &str| {
        let o = raw::open(p).unwrap();
        raw::records(&o.doc, "AECC_ALIGNMENT_STYLE")
            .into_iter()
            .find(|r| r.handle == 0x8DFE)
            .unwrap()
    };
    let off = style("samples/S21_TEST_STYLE_line_off.dwg");
    let on = style("samples/S22_TEST_STYLE_line_on.dwg");
    let gen = style("samples/S23_S21_line_on_generated.dwg");
    assert_eq!(aecc_style::flags_string(&off.data).unwrap(), "0".repeat(17));
    assert_eq!(
        aecc_style::flags_string(&on.data).unwrap(),
        format!("1{}", "0".repeat(16))
    );
    // Civil 3D changed the flag and the modified-time stamp (bits 801..865).
    let flag = aecc_style::components(&off.data).unwrap()[0].flag_bit;
    for (s, e) in raw::changed_bit_ranges(&off.data, &on.data, 0) {
        assert!(
            (s == flag && e == flag + 1) || (s >= 801 && e <= 865),
            "{s}..{e}"
        );
    }
    // Ours: the flag only.
    let mut d = off.data.clone();
    aecc_style::set_component_visible(&mut d, 0, true).unwrap();
    assert_eq!(d, gen.data);
    assert_eq!(
        raw::changed_bit_ranges(&off.data, &gen.data, 0),
        [(flag, flag + 1)]
    );
}
