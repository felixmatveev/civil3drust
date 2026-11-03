//! The string count of an alignment (D-056): Civil 3D's re-save of the
//! generated S184 (S186) writes exactly `required_string_count` for every
//! alignment, and the regenerated S188–S190 follow it.

use civil3drust::aecc_element_edit::required_string_count;
use civil3drust::{raw, string_stream};

fn check(file: &str, min: usize) {
    let d = raw::open(format!("samples/{file}")).unwrap();
    let mut n = 0;
    for r in raw::records(&d.doc, "AECC_ALIGNMENT") {
        // Civil 3D kept garbage strings read past the stream in a few
        // (S186 100D, 1010); those don't parse.
        let Ok(ss) = string_stream::parse(&r.data, r.handle_bits) else {
            continue;
        };
        let want = required_string_count(&r).unwrap();
        assert_eq!(ss.strings.len(), want, "{file} {:X}", r.handle);
        n += 1;
    }
    assert!(n >= min, "{file}: {n} alignments checked");
}

#[test]
fn civil3d_resave_follows_the_rule() {
    check("S186_S184_civ3d_recovered.dwg", 29);
}

#[test]
fn generated_alignments_follow_the_rule() {
    check("S188_S24_generated_chains.dwg", 44);
    check("S189_S24_generated_group_members.dwg", 7);
    check("S190_S24_generated_plain_members.dwg", 7);
}
