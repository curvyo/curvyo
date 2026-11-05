# ADRs for the membership lifecycle

Architect, 2026-11-05, for the customer decisions of 2026-11-05 (ROLES, WHO-PROMOTES, LAST-ADMIN,
LEAVE-WORDING, READD-DEFAULT A, WS-HISTORY B, FP-FLOW A, FP-LINK A, DEVICE-OR-PERSON A, NOTIFY A,
REJOIN-PATH default B). Covers slice 1 and the transport of slice 2. Proposals are vetoable at
the demo.

**One ADR (0041, Proposed, needs-customer: Frage 160 to 162). No new keyring payload, no keyring
validity change, relay protocol 3 unchanged with no new frame, no new crate, no new dependency,
`.curvyo` `format_version` and `share.json` versions unchanged. New formats: `share.ended.json`
v1 (container member and marker file), join messages v1 (`curvyo-join-v1`), an optional `left`
field in the devices list (no version change).** Amended 2026-11-05 after the tester's
findings: own keyring entries are confirmed by a `Ping`/`Pong` barrier and resent after the
catch-up (0041 decision 11), still with no frame, protocol version, format or server change.

## Depends on

- [ADR 0041](../../docs/adr/0041-membership-lifecycle.md) (new): roles and the per-person
  last-admin rule, leave, person leave of a synced workspace, held-back denials, add back, the
  ended record and the return check, repair that respects a withheld epoch, the verified join,
  notices; decision 11 (amendment): confirmation and resend of own keyring entries.
- [ADR 0010](../../docs/adr/0010-document-keyring-admins-and-revocation.md): every entry used
  exists (§3); §4 self-removal is Leave; §5 replay unchanged; §7 verified flow realised by
  0041 decision 8 (wording amended on acceptance); §9 history choice, refined client side by
  0041 decision 7.
- [ADR 0031](../../docs/adr/0031-sync-engine-relay-protocol-and-transport.md): `Hello`/`Proof`
  admission and `Refused{removed}` carry the return check; 6a rotation and repair; 6a's removed
  device keeps an ended record (amended on acceptance).
- [ADR 0037](../../docs/adr/0037-shared-workspace-list-rooms-and-keys.md): decision 9 removal
  keeps the marker's `share.ended.json` and the documents' `share.json` v2 (amended on
  acceptance).
- [ADR 0040](../../docs/adr/0040-own-devices-pairing-and-device-group.md): pairing connection
  and CPace reused for the join (decisions 3, 4); `reconcile` gains the `left` rule and adds as
  member in rooms with held-back denials (decisions 5, 8 amended on acceptance).

## Per criterion

- **1, 9**: list from `Keyring::participants()`; unnamed identity shows "A device".
- **2 to 4**: `SetRole`; any admin, any participant (0041 decision 1).
- **5, 14**: client refuses when no admin would remain outside the acted-on devices; replay's
  linear restore is the backstop and gets its own unit test (decision 1).
- **6**: ADR 0010 §5 unchanged.
- **7, 8**: admin count per device, own group from `0096`; dismissal kept on this computer.
- **10, 13**: own `RemoveMember`, `share.ended.json` with `ended: "left"` once the relay
  confirmed the leave or refused the device "removed", room keys kept, no further connection
  (decisions 2, 6, 11); "Leaving…" until then.
- **11, 12**: `Keyring::rotation_pending()`; Change keys now = 6a rotation. In a workspace the
  maker syncs, `reconcile` rotates for any leaver by itself and the notice reads "Keys changed"
  (decision 2, Proposal).
- **15**: marker keeps `share.ended.json`; documents keep `share.json` v2 (decision 6).
- **16**: `left: true` on the devices list entry, cleared by Sync again or switch-on; each device
  runs the `left` plan (admin:
  remove siblings still in, rotate, then itself; member: itself); refused unless an admin exists
  outside the maker's group (decisions 1, 3). Diverges from the criterion's "no device of B
  removes another": admin siblings do remove each other, which heals a sibling's re-add race and
  removes a sibling that stays offline; with an outside admin all those removals apply. PO to
  adjust the sentence.
- **17**: `Keyring::former_participants()`; last label from the existing label cache, no longer
  dropped on removal (decision 5).
- **18, 19, 22**: Full history = every epoch the admin holds; From now on = current epoch plus a
  fresh signed snapshot; other members do not repair withheld epochs (decision 7, Frage 160 A).
  The dialog says "the app does not give B the keys", never "B cannot read". Test for 22 also
  checks that no member writes `RewrapEpoch` for epoch k.
- **20, 25, 26, 33**: ended record plus the return check: `Hello{since_seq: u64::MAX}`, `Proof`,
  `Ping`, keyring until `Pong`, own replay, close; sends nothing else (decision 6). `heads` are
  the confirmed heads (decision 11). 26: the check does not retry; A's list shows the add back
  as pending until the relay holds it, so "Added" on A's screen means B's check finds it.
- **21**: Sync again rebuilds `share.json` and `keyring.log` and resumes; Keep as it is is Frage
  162 (default A: one own leave entry). PO to adjust "nothing is sent" if A stands.
- **23**: every epoch, no choice (WS-HISTORY B); a first add keeps `0095`.
- **24**: Add back is per device; `reconcile` keeps `may_readmit` for siblings.
- **27 to 29**: through one relay a mutual removal removes the admin whose own removal of
  the other arrived second (its connection is closed first); crossed demotions, or a removal arriving after the
  remover's own demotion, are both stored and held back: that is the frozen room (decision 4,
  by code reading, test to pin). PO to reword. `Keyring::held_back()`; mutual versus linear told apart by authors and subjects;
  Make admin and pairing into the group room warn; `reconcile` adds group devices as members
  while denials are held back, so no automatic write releases them (decision 4).
- **30**: `membership_notices` in `curvyo-sync-core::membership`, last seen admin set per data
  directory (decision 9, Proposal).
- **31, 32**: nothing new on the server beyond public keyring entries and one connection per
  check (decisions 6, 10).
- **34 to 36**: texts in `curvyo-ui-core`; no mechanism.
- **37 to 41 (slice 2)**: admin shows the code, invitee types server and code (UX
  MEM-VERIFIED-DIRECTION confirmed; criterion 37 to be turned round by the PO); CPace with
  `curvyo-join-v1`; check words; `AddMember{X, member}`; sealed room pointer; a swapping relay
  fails CPace (39); link stays (40); any admin adds a maker's new computer (41) (decision 8).

## Notes

- 2026-11-05: `remove_participant` must stop dropping the removed member's label; the kept label
  is what the Removed-or-left row shows.
- 2026-11-05: debts "From now on is a client rule" (new) and "Mutual removal freezes a device
  group" (now detected, still unsettled) in `docs/technical-debt.md`.
- 2026-11-05 (tester findings): every own keyring act (role change, removal and rotation, add
  back, leave, rewrap) is confirmed by the `Pong` after it and resent after the next catch-up
  if not; no protocol 4, because an echo or `KeyringAck` frame would still need the `Ping` to
  see a dropped entry. Debt "Return check heads" resolved; "A mutual removal cannot arise
  through one relay" corrected to the demotion case.
