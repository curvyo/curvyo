# Architecture review: separation of concerns

**Date:** 2026-11-06  
**Revision reviewed:** `19126071`  
**Scope:** Sampled the workspace boundaries, editor session and frontend, the collaboration boundary, and the file lists of recent sketch-page and membership commits. This is not an exhaustive review or a measurement of PR history. Findings are maintenance risks, not confirmed defects.

## Assessment

The crate split provides useful boundaries: for example, `curvyo-sync-core` has only `curvyo-crypto-core` as a production Curvyo dependency, keeping the document model out of the relay protocol ([manifest](../curvyo-sync-core/Cargo.toml)). The large file counts in commits `3bfde714` (membership) and `991ced70` (sketch pages) also include tests and changes across model, session and UI. An end-to-end feature naturally crosses those boundaries; the counts alone do not prove poor separation. No PR-wide causal analysis was performed.

## Findings

1. **High — editor session is a shared change hub.** [`Session`](../curvyo-editor-wasm/src/session/mod.rs#L423) holds document, tools, viewport, sync, sketch, text, trace and manufacturing state. Its child modules can access the same private fields; for example, [page activation](../curvyo-editor-wasm/src/session/pages.rs#L16) coordinates tiling, manufacturing, sketch, selection and viewport. Some coordination is inherent, but unrelated features can depend on each other's internal state and must be considered when changing a transition. **Proposal:** incrementally move cohesive state and its invariants behind feature-owned methods, starting with one frequently changed transition; keep cross-feature sequencing explicit in the session. Avoid a wholesale rewrite or a new crate solely for this.

2. **Medium — the frontend editor hook combines lifecycle, input, rendering and state projection.** [`useEditorSession`](../frontend/src/hooks/useEditorSession.ts#L939) owns session and canvas lifecycle as well as tool and sketch state; [`syncFromSession`](../frontend/src/hooks/useEditorSession.ts#L1101) refreshes several feature views, while the same hook handles [wheel input and the render loop](../frontend/src/hooks/useEditorSession.ts#L1415). This makes feature changes and lifecycle changes meet in one 3,013-line file, increasing review and regression scope. **Proposal:** extract cohesive input, lifecycle and feature-projection helpers with explicit dependencies, retaining one owner for session lifetime and frame ordering. Preserve existing behavior and test those boundaries as they are extracted.

3. **Medium — JSON view contracts are maintained across languages without a shared checked shape.** Rust builds the [share view](../curvyo-editor-wasm/src/session/sync_view.rs#L38) as JSON; the frontend parses it with a [type assertion](../frontend/src/hooks/useCollaboration.ts#L329), while other consumers read subsets independently ([workspace sync](../frontend/src/hooks/useWorkspaceSync.ts#L124)). Membership notices similarly [cast parsed arrays](../frontend/src/lib/membershipState.ts#L66) without checking element shapes. Rust and TypeScript can therefore disagree without a type error; this review did not establish an actual mismatch. **Proposal:** centralize each frequently used boundary's decoding and add focused contract tests using representative Rust-produced payloads, including optional and ended states. Generate types only if that proves simpler than maintaining the contracts.

## Priority

Start with the session transition and hook areas when a story already touches them; verify the contract boundary with one representative share-view test. Compare future changes by responsibility and reviewability, not by file count alone.
