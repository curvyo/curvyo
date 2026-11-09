# ADR 0044: The workspace is the only unit of sharing

**Status:** Accepted 2026-11-09 (customer). Was `needs-customer` (persistence/sync, document
model, a file format we write). Customer decision DOC-SHARE-REMOVE B (2026-11-06: "wir gehen
immer auf Workspace"); answers to Frage 165 to 173 on 2026-11-09, all defaults: SHARE-UNIT A,
ONE-SWITCH A, PEOPLE-VIEW A, MEMBER-DEVICES C, JOIN-FLOW A, STOP-SHARING B, DEVICE-WORD A,
BROWSER-PEOPLE (the browser takes part only as one of the maker's own devices), LEGACY-SHARE A.
Asked by `specs/0105-sharing-concept`. **Supersedes in part:** ADR 0031 decisions 2, 5 ("one
document per room"), 8 (version `1.`) and 9 (container `share.json` v1 and `keyring.log`); ADR
0037 decision 10 (take-in), the take-in and `0092` room sentences of 11, the `Moved` payload in
12, and WS-SEPARATE B; ADR 0041 decision 2 (the document leave), 5 (the History choice), 6 (the
container ended record and the return check) and 7 (withheld epochs); ADR 0010 §8/§9 (the
history choice). Choices marked (Proposal) are the architect's, vetoable at the demo.

## Context

Since `0092` a document can be shared on its own (its own room, keyring and `1.` link), and since
`0095` a workspace can be shared as one room (ADR 0037). `0097` built the membership lifecycle
twice, once per kind, and `0095` needed a take-in with a `Moved` record to move a document from
its own room into a workspace. The customer wants one answer to "what leaves my computer and who
sees it": the workspace. The engine (`curvyo-sync-core` frames, client, relay, keyring) is the
same for both kinds; only the entry points, the container's share members and a few client rules
are document-only.

Files written by the `0092` and `0097` builds exist on disk: containers with `share.json` v1 plus
`keyring.log`, and containers with `share.ended.json`. `pack` writes only members the document
models, so keeping them on save would need new pass-through code.

## Options

**A. Remove single-document sharing; the workspace is the only unit (chosen, DOC-SHARE-REMOVE B).**
One membership model, one room kind for people, less code (roughly 2 to 3 thousand lines).

**B. Keep document rooms in the engine, hidden behind the workspace in the UI.** Lost: both
lifecycles, the take-in and the return paths stay to be tested and kept consistent, for a feature
nobody can reach.

**C. Turn every shared document into a one-document workspace on open.** Lost: a silent folder
creation and file move the maker did not ask for, a new room per file, and a migration in a
format we write.

For files already shared on their own:

**A. Open as a plain local document; the next save drops the old members (chosen, LEGACY-SHARE A).**
**B. Keep the old members on save.** Lost: pass-through code in a format we write, and an older
build would reconnect such a file and upload edits made in the new build without the new build
ever showing it (R-SYS-007).

## Decision

1. **One unit.** People share and devices sync only workspace rooms (ADR 0037); the device group
   room (ADR 0040) stays. No code path creates, joins or resumes a document room. One connection
   per synced workspace replaces ADR 0031 decision 2's "per open shared document". Frames,
   envelope, relay, snapshots and keyring are unchanged; the relay keeps serving protocols 1 to 3
   (the device group uses single snapshots; old document rooms stay on disk, ADR 0031 has no room
   deletion).
2. **Invite links.** Only `2.` links are created and accepted. A `1.` link is parsed and refused
   on the device with its own text; nothing is sent. `InviteKind::Document` stays only for that
   refusal. The browser's Join field accepts a device code only; a `2.` link there is refused
   with "Open this link in the desktop app." (BROWSER-PEOPLE).
3. **Old shared files (LEGACY-SHARE A).** Read rules of the new build per container:

   | Container holds | Opens as | Next Save writes |
   |---|---|---|
   | `share.json` v2, no `keyring.log` | workspace document (unchanged) | unchanged |
   | `share.json` v2 and `keyring.log` | `Damaged` (unchanged; no build writes it) | – |
   | `share.json` v1 with or without `keyring.log`, or `keyring.log` alone | plain local document, no connection, no key read | neither member |
   | `share.ended.json` | plain local document | no ended record |
   | `share.json` with `format_version` > 2 | `FormatTooNew` (unchanged) | – |

   Opening writes nothing. `unpack` reports a read-only "legacy share members present" flag for
   one notice; it is not a new member. The container `format_version` and the `share.json`
   reader (version 2, and the v1 shape the workspace marker uses, ADR 0037 decision 1) do not
   change; the reader only becomes more lenient. The golden files `shared_v34.curvyo` and
   `ended_v36.curvyo` become legacy inputs; the marker goldens stay as they are. A `0092` file in
   a workspace folder is an ordinary workspace document: Share or Add to workspace writes its
   `share.json` v2 and drops the v1 members, with no membership check and no `Moved`.
4. **Gone with the document rooms.** The take-in and the `Moved` codec (ADR 0037 decision 10);
   the History choice: an add back always wraps every epoch the adding device holds,
   `add_back(member)` (WS-HISTORY B); `Keyring::withheld`, so any member repairs any epoch a
   current member lacks; the document leave, ended record and return check (ADR 0041 decisions 2,
   6, 7 for documents). No keyring payload, validity rule, frame, protocol version or
   `format_version` changes. `add_back` itself stays: the device group re-pairs a removed own
   device with it (ADR 0040).
5. **The way back is a new invite** (already built in PR #294, ADR 0041 amendment of
   2026-11-09). The relay lets a removed identity that starts at `since_seq` 0 offer a claim of
   an invite; the keyring counts an add only after every removal (ADR 0010 §8). Two rules close
   what that change left open (`specs/0105-sharing-concept/arch-review-join-rule.md`): a removed
   claimant whose invite is already claimed is refused as removed before anything is sent to it,
   and a claim admits a connection only if the keyring then lists the claimant. The return
   check (`CheckOutcome`, `Keyring::admitted_after`) is retired. `EndedRecord.heads` stays in the
   marker's `share.ended.json` (a format we write), unread.
6. **"Make a workspace for this file"** (SHARE-UNIT A, desktop only). One command in
   `curvyo-app` over `curvyo-storage-io`: create `<parent>/<name>/` (" 2", " 3" on a clash, never
   the current parent), save first if dirty, rename the file into it (a sibling folder is on the
   same file system; a failed rename leaves the file where it was), write `workspace.toml` and
   open the folder as a Local workspace. No format, crate, dependency or protocol change.
7. **One Sync switch** (ONE-SWITCH A). On is ADR 0037 decision 9 Share followed by ADR 0040
   decision 6 On; Invite person runs both in its one confirm. Off is offered only while every
   participant is one of the maker's devices (in the device group's list): ADR 0040 decision 6
   Off, then this device ends the link without a keyring entry, as Stop sharing does for the
   only participant left (PR #294), and the marker drops its share files (Proposal). The room
   stays on the relay.
8. **People view** (PEOPLE-VIEW A). Rows are grouped by the label each device sealed, for display
   only. Roles, Remove and Leave stay per device (ADR 0010 §1, ADR 0041 DEVICE-OR-PERSON A). No
   keyring change; the grouping is unverified, a device can claim any label.
9. **Server reachability is polled by the client.** While the Share tab is shown, a server
   address is set and the workspace has no link (a live or reconnecting link shows its own
   state), the page opens a WebSocket to the server address and closes it as soon as it opens:
   no frame, no identity, no room id. It repeats every 5 s, one probe at a time, each given up
   after 4 s; it stops when the tab is hidden or the address is cleared. The server learns the IP
   address and the times, nothing else. It runs in the frontend, where the transport lives (ADR
   0031 decision 2); no core crate is involved. This is the only request a Local workspace or a
   local document causes, and only while the Share tab is shown.
10. **Kept unchanged:** `ClientSession`'s single-snapshot path (device group), `EndedRecord` and
    `share.ended.json` for the workspace marker, `share.json` v2 and `CURRENT_SHARE_FORMAT_VERSION`
    = 2, the marker's `share.json` v1 shape, `device_keys` by room, the relay and its tests,
    MEMBER-DEVICES C (`0096` criterion 17 as built).

## Consequences

- Old document rooms stay on the relay and their epoch keys stay in the credential store or key
  file, unused (`docs/technical-debt.md`).
- A file shared on its own and saved by the new build opens as a local file in an older build
  too; its old link stops working.
- The browser works with other people only as one of the maker's own devices until a story says
  otherwise.
- The workspace path reuses the document session (link, pump, presence, undo, label cache):
  tests of kept behaviour are ported to a workspace pair, never deleted.
- `withheld` going changes which `RewrapEpoch` entries honest clients write; a test pins that a
  returning workspace member gets an epoch only a third member held.
- The reachability probe needs the exception in `0105` criterion 24 ("no network request").
- The deletion list for the implementer is in `specs/0105-sharing-concept/adrs.md`.
