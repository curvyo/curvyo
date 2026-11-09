# Architect review: a removed device joins again by a new invite (PR #294)

Architect, 2026-11-09. Commit `c48e3e1d` on `main` (`curvyo-sync-core/src/relay/admission.rs`,
`client/inbound.rs::removed_here`, `tests/sim_removal.rs`), read against `curvyo-crypto-core`
`keyring/{validity,state,mod}.rs` and `relay/room.rs`.

## Verdict: CHANGES REQUESTED (as a follow-up `fix/curvyo-sync-core` PR, before 0105's PR)

The rule the customer asked for holds for honest clients: no way back without a fresh invite
made by an admin. The relay change leaves two gaps for a removed device whose own old invite is
still valid (invites last 24 hours, `INVITE_VALID_SECONDS`), and the claim "the old link is
still refused as removed" is not what the code does or what the test checks.

## What holds

- **A fresh invite needs an admin.** `AddPendingInvite` is valid only for an admin author
  (`validity.rs`, `is_authorized`).
- **A used invite cannot be claimed again by an honest client.** `ClaimInvite` is valid only
  while the invite is unclaimed in the entry's own ancestors (`invite.claimed_by.is_none()`).
- **A removal is not undone by an old or backdated add.** `membership()` (`state.rs`) counts an
  add only if every `RemoveMember` of that member is its ancestor. A claim built on heads from
  before the removal leaves the device removed in every replay.
- **A reconnect from a cursor is refused at `Proof`** (`since_seq != 0`). A member start from
  cursor 0 sends `Ping` after its proof (`on_challenge`); `genesis_or_claim` answers anything
  but a keyring entry with `Removed`, so it does not hang.
- **Epochs.** The rejoined device is a member with `Admission::Claimed`; `missing_wraps` gives it
  every epoch the rewrapping member holds, including those of its time away. That matches
  WS-HISTORY B and the first join. Because it still holds pre-removal keys, the "expired invite
  and no key yet" skip in `missing_wraps` never applies to it; harmless.
- **Format and protocol.** No frame, refusal reason, keyring payload, validity rule, protocol
  version or `format_version` changes. It is a relay behaviour change: the server must be
  redeployed, and against an old relay a rejoin is refused as `Removed` (fails safe, but the
  client cannot tell an old relay from a refusal).

## Findings

1. **`curvyo-sync-core/src/relay/admission.rs` `genesis_or_claim`: a removed device gets the
   current keyring with its old link.** Rule: ADR 0031 decision 5 / this module's doc comment
   ("nothing reaches a connection that proved neither membership nor an invite"; a removed
   device should be refused as removed). The `Used` check is `claimed_by.is_some_and(|m| m !=
   claimant)`, so the device's own earlier claim passes. Within 24 hours a parentless probe of
   the old link gets `send_keyring`: members, roles, new invites and epoch ids after its
   removal. An honest client's real claim then fails validity and is refused `Protocol` (later
   `Expired`), not `Removed`. **Fix:** when `removed`, refuse `RefusalReason::Removed` if
   `invite.claimed_by.is_some()`, before the probe answer.
2. **Same function: a crafted claim admits a removed device's connection.** Rule: ADR 0031
   decision 5, admission only for a current member, a genesis or a valid claim (the relay
   decides alone). A modified client builds a claim of its own old, unexpired invite on keyring
   heads from before its first claim. That entry is valid against its ancestors and
   `append_entry` stores it; the keyring still says removed ("What holds", third point),
   but `log.admit` then runs anyway: `Welcome`, the whole keyring, the snapshot and every record
   go to it, and `on_push` accepts its pushes under any epoch the keyring names, including the
   pre-removal epochs it holds. `close_removed` ran before `admit`, so the connection stays
   until the next keyring entry. **Fix:** after `append_entry`, admit only if
   `log.keyring().role(&claimant.member).is_some()`, else `Refused(Removed)` and close (better:
   decide on the replayed candidate keyring before the entry is stored, so no such entry stays
   in the log). Finding 1's check also stops this path; keep both (the second is the general
   rule).
3. **`curvyo-sync-core/tests/sim_removal.rs`: the test does not cover the old link.** Rule:
   Definition of Done (every criterion covered). `a_removed_device_joins_again_by_a_new_invite`
   checks a reconnect from a cursor only. **Fix:** add relay tests in
   `curvyo-sync-core/src/relay` tests or `tests/`: (a) removed device, cursor 0, its old secret
   within 24 hours: `Refused(Removed)` and no `Keyring` frame sent; (b) a backdated claim of its
   own invite: not admitted, no `Welcome`, nothing stored; (c) the existing rejoin by a fresh
   invite still passes.
Accepted risk (no change): an invite link is a bearer token; a removed device that obtains a
fresh link meant for someone else can claim it first. That was true for any outsider before and
is the customer's model (COOP-JOIN A).

## Dead code left by the change

`add_back` is **not** dead: `curvyo-editor-wasm/src/device_group_glue.rs::admit` re-pairs a
removed own device with it (ADR 0040). What is dead is listed in `adrs.md`, "Deletion list".
