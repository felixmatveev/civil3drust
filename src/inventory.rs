//! Inventory of Civil 3D (AECC) content in a DWG.
//!
//! Civil 3D objects are custom classes registered in the drawing's class
//! table. acadrust reads that table, so before decoding any object data we
//! can already see which AECC classes a drawing uses and how many instances
//! of each it reports. That is the first step in the cleanroom process:
//! pick a class, collect sample files that exercise it, then study the
//! object data.

use acadrust::document::CadDocument;
use std::path::Path;

/// One AECC entry from a drawing's class table.
#[derive(Debug, Clone, PartialEq)]
pub struct AeccClassInfo {
    pub dxf_name: String,
    pub cpp_class_name: String,
    pub application_name: String,
    pub class_number: i16,
    pub instance_count: i32,
    pub is_entity: bool,
}

/// True if a class-table entry belongs to Civil 3D.
///
/// Civil 3D DXF class names start with `AECC_`; the C++ class names start
/// with `AeccDb`. Checking both catches either convention.
pub fn is_aecc(dxf_name: &str, cpp_class_name: &str) -> bool {
    dxf_name.to_ascii_uppercase().starts_with("AECC") || cpp_class_name.starts_with("AeccDb")
}

/// List the AECC classes in an already-loaded document, sorted by DXF name.
pub fn aecc_classes(doc: &CadDocument) -> Vec<AeccClassInfo> {
    let mut out: Vec<AeccClassInfo> = doc
        .classes
        .iter()
        .filter(|c| is_aecc(&c.dxf_name, &c.cpp_class_name))
        .map(|c| AeccClassInfo {
            dxf_name: c.dxf_name.clone(),
            cpp_class_name: c.cpp_class_name.clone(),
            application_name: c.application_name.clone(),
            class_number: c.class_number,
            instance_count: c.instance_count,
            is_entity: c.is_an_entity,
        })
        .collect();
    out.sort_by(|a, b| a.dxf_name.cmp(&b.dxf_name));
    out
}

/// Open a DWG file and list its AECC classes.
pub fn aecc_classes_in_file(
    path: impl AsRef<Path>,
) -> Result<Vec<AeccClassInfo>, acadrust::DxfError> {
    let doc = acadrust::DwgReader::from_file(path)?.read()?;
    Ok(aecc_classes(&doc))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_aecc_names() {
        assert!(is_aecc("AECC_ALIGNMENT", "AeccDbAlignment"));
        assert!(is_aecc("aecc_alignment", ""));
        assert!(is_aecc("", "AeccDbSurfaceTin"));
        assert!(!is_aecc("LWPOLYLINE", "AcDbPolyline"));
        assert!(!is_aecc("AcDbAlignment", "AcDbSomething"));
    }

    #[test]
    fn empty_document_has_no_aecc_classes() {
        let doc = CadDocument::new();
        assert!(aecc_classes(&doc).is_empty());
    }
}
