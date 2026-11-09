# ADRs for the sharing concept (workspace only)

Architect, 2026-11-09, for DOC-SHARE-REMOVE B (2026-11-06) and the answers to Frage 165 to 173
(2026-11-09, all defaults). **One new ADR (0044, Accepted). No new crate, no new dependency, no
keyring payload or validity rule, no frame, relay protocol 3 unchanged, `.curvyo`
`format_version` and `share.json` version unchanged; the container reader only becomes more
lenient (legacy share members read as local).** Proposals are vetoable at the demo.

## Depends on

- [ADR 0044](../../docs/adr/0044-workspace-only-unit-of-sharing.md) (new): the workspace is the
  only unit; `1.` links refused; legacy containers open local (LEGACY-SHARE A); take-in,
  `Moved`, History choice, `withheld` and the return check gone; "Make a workspace for this
  file"; one Sync switch; people grouped for display only; reachability polled by the client.
- [ADR 0031](../../docs/adr/0031-sync-engine-relay-protocol-and-transport.md): engine, relay,
  envelope and snapshots unchanged; decisions 2, 5 (one document per room), 8 (`1.`) and 9
  (container share members) superseded by 0044.
- [ADR 0037](../../docs/adr/0037-shared-workspace-list-rooms-and-keys.md): the only room people
  share; decision 9 Share is Sync on; decision 10 (take-in) and the `Moved` payload superseded.
- [ADR 0040](../../docs/adr/0040-own-devices-pairing-and-device-group.md): decision 6 On and
  Off are the two halves of the Sync switch; `add_back` stays for re-pairing (decision 5).
- [ADR 0041](../../docs/adr/0041-membership-lifecycle.md): roles, workspace leave and person
  leave, held-back denials and decision 11 unchanged; decisions 2 (document), 5 (History), 6
  (container record, return check) and 7 superseded; the 2026-11-09 amendment's join rule gets
  the two relay checks of `arch-review-join-rule.md`.
- [ADR 0010](../../docs/adr/0010-document-keyring-admins-and-revocation.md): §8/§9 history choice
  superseded (always every held epoch); keyring format and rules unchanged.

## Per criterion

- **1, 13**: ADR 0044 decision 1; the page loses the document entry points (M2).
- **2**: workspace documents keep `share.json` v2 (ADR 0037 decision 3).
- **3**: ADR 0044 decision 2; `InviteKind::Document` parsed only to refuse it.
- **4 to 6**: ADR 0044 decision 3 (read table, legacy flag, goldens retargeted).
- **7 to 9**: ADR 0044 decision 6, one `curvyo-app` command over `curvyo-storage-io`.
- **10 to 12**: ADR 0044 decision 7 (0037 decision 9 + 0040 decision 6).
- **14, 15**: ADR 0044 decision 8; Stop sharing as built (PR #294).
- **16, 17**: JOIN-FLOW A on the existing link and pairing paths; MEMBER-DEVICES C as built.
- **19**: ADR 0044 decision 2 (browser refuses `2.`).
- **21**: ADR 0044 decision 9. **Conflict with criterion 24** ("no network request" for a local
  document or a Local workspace): the probe is a request. Question for the PO, below.
- **23**: tests of kept behaviour are ported to a workspace pair, never deleted (ADR 0044
  consequences).

## Question for the PO (not the customer: wording of an accepted decision)

Criterion 24 vs 21. A: criterion 24 gets "except the reachability check while the Share tab is
shown (criterion 21)". B: the check runs only for a Synced or Shared workspace. Recommendation
A (the maker needs to see the server before switching Sync on). Default A.

## Removal milestones (one story PR)

M2 frontend and journeys, M3 `curvyo-editor-wasm` and `curvyo-ui-core`, M4 `curvyo-sync-core`
and `curvyo-crypto-core`, M5 `curvyo-document-core` and `curvyo-storage-io` (read rules,
goldens). Callers before callees; every commit builds and passes its crates' gate. Before M2: the
`fix/curvyo-sync-core` PR for the join rule (`arch-review-join-rule.md` findings 1 to 3).

## Deletion list (state of `main` at `c48e3e1d`)

Already gone in PR #294: the Add back dialog (`FormerList.tsx`), the History choice in the page,
the return-check glue in `wasm_membership.rs` and its texts.

`curvyo-sync-core` (M4)
- `src/client/check.rs` whole, `mod check;`.
- `src/client/api.rs`: `SessionStart::ReturnCheck`, `CheckOutcome`, `SessionEvent::CheckResult`,
  `History`.
- `src/client/mod.rs`: `Start::Check`, its construction (~line 146), `is_check` and its use in
  the link-gone path (~line 339 to 346).
- `src/client/inbound.rs`: `Start::Check` in `on_challenge`'s ping match; the check-stage
  frame handler (`check_refused`, `check_caught_up` callers).
- `src/client/outgoing.rs`: `add_back(member, history)` becomes `add_back(member)`; the
  `FromNow` branch and its snapshot request.
- `src/client/keys.rs`: `write_current_addition`.
- `src/workspace_stream.rs`: `Moved`, `encode_moved`, `decode_moved`, `MOVED_MAGIC`.
- `src/lib.rs`: re-exports of `CheckOutcome`, `History`, the `Moved` codec.
- **Keep:** `add_back` (used by `device_group_glue.rs::admit`), `former_participants`,
  `removed_here`, `devices.rs`'s `withheld` (devices not admitted in a room, a different thing).

`curvyo-crypto-core` (M4, own commit)
- `src/keyring/mod.rs`: `Keyring::admitted_after`; `Keyring::withheld` and its use in
  `missing_wraps` (and the doc comment's "withheld" clause).
- Tests: `tests/acceptance_0097_tester.rs` and `tests/keyring_lifecycle.rs` cases for
  `withheld` and `admitted_after`; new test: a returning member gets an epoch only a third member
  held.

`curvyo-editor-wasm` (M3)
- `src/session/sync_return.rs` whole (`check_answered`).
- `src/session/sync.rs`: `SyncLink.check`, `SyncLink.returning`.
- `src/session/sync_events.rs`: the `SessionEvent::CheckResult` arm.
- `src/session/sync_view.rs`: `return_view`, its caller, the `CheckOutcome` import.
- `src/device_group_glue.rs`: `add_back(identity.member, History::Full)` becomes
  `add_back(identity.member)`.
- The document entry points of the removal plan: `sync_start.rs` (`share_document`,
  `join_document`, `resume_shared`, `sync_config`), `workspace_take_in.rs` and
  `wasm_workspace_take_in.rs`, the take-in branch of `workspace_share_builder.rs`,
  `SyncEnd::Moved`, the document branch of the end (`set_ended_share`), the `wasm_sync.rs`
  document exports. Port `sync_test_support.rs::shared_pair` to `share_workspace`/`join_workspace`
  first.

`curvyo-document-core` and `curvyo-storage-io` (M5)
- `container_share.rs`: `ShareRecord`, `Sharing::Room`, `Sharing::Ended`, `Document::share`,
  `set_share`, `read_room_share`, the container `KEYRING_MEMBER`; keep `ShareJson`,
  `ROOM_SHARE_VERSION`, `WorkspaceShareRecord`, `WorkspaceRoomRecord`.
- `container_ended.rs`: `Document::ended_share`, `set_ended_share`; keep `EndedRecord` with
  `heads` (marker format).
- `curvyo-storage-io/src/workspace_documents.rs`: `ShareMembers.keyring`, `KEYRING_MEMBER`.

Frontend (M2)
- `src/components/panel/ShareTab.tsx`: `endedAsks` and the `ended.check === "addedBack"` branch
  (~lines 190, 263); the `!ended` gate on `checkServer` becomes the 5 s poll of ADR 0044
  decision 9.
- `src/hooks/useCollaboration.ts`: the return-check field (~line 71), `resume`, `startShare`,
  the `1.` join; `src/lib/movedPush.ts`; take-in calls in `workspaceShare.ts`,
  `useSharedTree.ts`, `useWorkspaceSync.ts`.

Tests that pin removed behaviour: `curvyo-sync-core/tests/{acceptance_0097_tester,sim_membership,
sim_confirmation,acceptance_0095_tester}.rs` (History, return check, `Moved`),
`curvyo-sync-server/tests/acceptance_0097_tester.rs` (FromNow). Not membership: the `History`
matches in `curvyo-editor-wasm/tests/acceptance_0023_tester.rs`, `acceptance_0052_tester.rs` and
the `edit_polish` tests are undo history; leave them.

Debt to close in the same PR: "From now on is a client rule", "Return check heads", "Changes
pushed to an old room after `Moved` stay local"; reword "A taken-in document's old room stays on
the relay" to rooms of documents shared on their own.

## Notes

- 2026-11-09: the join rule of PR #294 is reviewed in `arch-review-join-rule.md` (CHANGES
  REQUESTED, a `fix/` PR).
- 2026-11-09 (customer feedback on PR #298): no Sync switch per workspace. ADR 0044 decision 7
  holds with Invite person as the On (Share in its one confirm, with my devices in a device group)
  and Stop sharing or "Sync to my devices" off as the Off; Invite person is greyed with its reason
  (`curvyo-ui-core::sharing_text::invite_blocker`). No format, frame or protocol change.
