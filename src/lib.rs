//! # civil3drust
//!
//! Cleanroom implementation of Civil 3D (AECC) DWG objects, built on top of
//! [`acadrust`].
//!
//! ## Status
//!
//! Early skeleton. Current scope is **tangent-only horizontal alignments**
//! (no curves, no spirals). The first milestone is:
//!
//! > Read a Civil 3D DWG, change the length of one alignment segment,
//! > save it back, and open the result in Civil 3D without errors.
//!
//! ## Modules
//!
//! - [`alignment`] — format-independent alignment geometry (stationing,
//!   segment editing). No DWG knowledge lives here.
//! - [`inventory`] — reports which AECC classes and objects a DWG contains,
//!   using acadrust's class table. This is the starting point for
//!   reverse-engineering object layouts from sample files.
//! - [`aecc_alignment`] — decodes tangent-only `AECC_ALIGNMENT` records
//!   into [`Alignment`] geometry.
//! - [`raw`] — the stored bytes of AECC records (kept opaque by acadrust)
//!   and bit-level helpers for comparing sample files.

pub mod aecc_alignment;
pub mod alignment;
pub mod inventory;
pub mod raw;

pub use alignment::{Alignment, AlignmentError, Point2, Tangent};
pub use inventory::{aecc_classes, AeccClassInfo};
