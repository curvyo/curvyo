//! The compiled-in extension contract (ADR 0034 §1, §4): what an extension crate declares and what
//! the shell draws from it.
//!
//! An extension is an ordinary workspace crate with plain functions and one [`Manifest`] value.
//! The shell's registration module (`curvyo-ui-core/src/extensions.rs`) lists the compiled-in
//! [`Extension`] values and dispatches by extension id (ADR 0080 §1: the trait exists since the
//! second extension crate). The contract is `0.x` and unstable (`CHANGELOG.md`).

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod drawing;
mod extension;
mod manifest;
mod notice;
mod param;

pub use drawing::LineSegment;
pub use extension::{
    Bound, Clamp, ClosedPath, Cubic, Edit, Extension, GenerateRequest, Generated, Handle, Part,
    ScaleFactor, Seed,
};
pub use manifest::{
    Block, Contribution, FixtureKind, GeneratorKind, Manifest, ParamGroup, Tab, ToolboxCard,
    ToolboxCommand, WorkbenchContribution,
};
pub use notice::NoticeContext;
pub use param::{ChoiceOption, Degrees, Param, ParamKind, ParamValue, ShownWhen, StoredParams};
