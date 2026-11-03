//! The compiled-in extension contract (ADR 0034 §1, §4): what an extension crate declares and what
//! the shell draws from it.
//!
//! An extension is an ordinary workspace crate with plain functions and one [`Manifest`] value.
//! The shell's registration module (`curvyo-ui-core/src/extensions.rs`) lists the compiled-in
//! manifests and dispatches by extension id; there is no `Extension` trait until a second
//! extension crate exists (ADR 0034 §3). The contract is `0.x` and unstable (`CHANGELOG.md`).

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod drawing;
mod manifest;
mod notice;
mod param;

pub use drawing::{FixtureDrawing, LineSegment};
pub use manifest::{
    Block, Contribution, FixtureKind, Manifest, Tab, ToolboxCard, ToolboxCommand,
    WorkbenchContribution,
};
pub use notice::NoticeContext;
pub use param::{Param, ParamKind, ParamValue, StoredParams};
