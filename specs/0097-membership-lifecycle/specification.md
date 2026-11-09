# Membership lifecycle: admins, leaving, adding back, verified join

Status: Draft (2026-11-05, product owner; customer decisions of 2026-11-05 applied; becomes Ready once ADR 0041 and the UX notes exist, nothing else blocks it)
Priority: Must
Origin: Customer (request of 2026-11-05, translated): "membership management" = slice 2 of `0092` (criteria 41 to 45 there, R-COLLAB-009 to -012): a second admin for other people, leave, add a removed participant back with the choice "full history" or "from now on" (ADR 0010 §9), the verified-fingerprint join (ADR 0010 §7: the invitee shows a fingerprint, an admin confirms it by comparing, the participant is added without an invite link), and two admins who remove each other at the same time (ADR 0010 §5). To apply to shared documents (`0092`) and shared workspaces (`0095`, ADR 0037), and to stay consistent with `0096` (own devices). What is marked Proposal is the product owner's until the customer accepts it.

## User value

As a maker who shares documents and workspaces with other makers I want to hand out admin rights, leave a share, take a person back after a removal and add a person by comparing a fingerprint instead of sending a link, so that sharing does not depend on one person's computer, and so that every screen tells me honestly what leaving, removing and adding back do and do not do.

## Problem today

- `0092` slice 1 and `0095` slice 1 can invite (link), admit and remove with key rotation. An admin can do all of it, but there is exactly one admin in practice: nothing in the UI promotes a second one, and ADR 0010 §13 says the second admin is the backup when an admin loses a computer (R-COLLAB-012, Must).
- A member who wants out can only close the file. R-COLLAB-009 (Should) says leave is a real act. ADR 0010 §8: it is a roster act, not a cryptographic one, because the leaver still holds the current epoch key until an admin rotates.
- A removed participant is gone from the list with no way back except a new invite link; R-COLLAB-011 (Should) wants an ordinary re-add with the history choice of ADR 0010 §9.
- The invite link is the only way to add somebody. It works with a link that anybody who sees it can claim (once). ADR 0010 §7 names the verified flow as the preferred one; it is not built.
- Two admins who remove each other at the same time are handled by ADR 0010 §5 in the crypto crate and covered by tests there (`keyring_forks.rs`), but nothing tells the maker. `0096` found the consequence: the room is then frozen between those two (debt "Mutual removal freezes a device group", Frage 146).
- `0096` made a person's several computers several participants. Role actions on a participant therefore need a rule for a person who has several devices (Frage 155).

## The field

- **Signal groups**: two roles, admin and member; an admin can promote and demote members and other admins. An admin who wants to leave must name a new admin first (Signal's support page and how-to guides; third-party text, not Signal's own wording for every client). A member added later sees only what is sent after joining: there is no "full history" because every message is sealed per sender. Safety numbers: 60 digits in 12 groups of 5 plus a QR code, compared or scanned over a channel other than Signal; the number changes when a contact gets a new phone, which one source calls usually innocent. Lesson: a short digit string is understood, and the "name a successor before you go" rule works. We do better where Signal is silent: our wording at leave says what the leaver can still read.
- **Google Drive and Docs**: owner, editor, commenter, viewer, enforced by the server. The owner moves ownership to another person, who must accept; the old owner then becomes an editor. A person who was removed keeps what they downloaded. Lesson: the server can enforce a viewer because it holds the plaintext; we cannot (ADR 0010 §12).
- **Notion**: owner, admin, member, guest, per page "can view / comment / edit / full access". Leaving a workspace hands ownership to a remaining person automatically in some cases (reports differ). Lesson: do not hand admin rights over silently; we make the leaver choose.
- **Figma**: team owner and admins, per-file edit and view; owners are asked to transfer files before leaving and the system moves them to the team owner otherwise (community answers, not verified in the official help). Lesson: a shared thing must never be left with nobody who can manage it.
- **Obsidian Sync (shared vaults)**: an owner invites collaborators with read and write rights, no read-only role, removal by the owner; a removed collaborator keeps the local vault (from memory of their documentation, not rechecked). Lesson: a "just remove" model is accepted when the wording says what stays; ours adds rotation.

What we do better: no server decides who is an admin (the signed keyring does, ADR 0010), several admins are possible without any account, "leave" and "remove" are worded for what they really do, adding back is a choice about keys and not a hidden default, and the verified join needs no link that can be forwarded.

## Words

- Participant, member, admin, epoch, rotation, removal, keyring: as `0092` "Words" and ADR 0010. A participant is a device (ADR 0010 decision 1); two computers of one maker are two participants (`0096`).
- **Role**: Admin or Member, the only two (ADR 0010 §12).
- **Leave**: a participant writes its own `RemoveMember` (ADR 0010 §4, §8).
- **Rotation pending**: the current epoch key is wrapped to someone who is no longer a participant (they left, or a removal was not followed by a rotation); only an admin can write the `NewEpoch` that ends it.
- **Add back**: an admin writes `AddMember` for an identity that was removed or left, with a history choice. The identity's public keys are already in the keyring, so no invite link and no fingerprint comparison are needed.
- **Verified join**: adding a person who is not in the keyring yet, by comparing a fingerprint (ADR 0010 §7). Slice 2.
- **Own devices**: the `0096` group of one maker. **Shared document**: `0092`. **Shared workspace**: `0095`. Membership of a workspace is one keyring for the whole folder (WS-VISIBLE A); a document inside it has no roles of its own.

## Slicing

The customer asked for all five parts. They are cut by what they need (Proposal; the lead may cut differently). Slice 1 uses only keyring payloads that exist (`SetRole`, `RemoveMember`, `AddMember`, `NewEpoch`, `RewrapEpoch`) and needs no new transport: an admin already holds the public keys of everyone who was ever in the keyring. The verified join needs something that carries the invitee's full public keys to the admin (a fingerprint is a hash, the admin needs the keys to write wraps), which is ADR 0041.

| Slice | What the customer can try | Priority |
|---|---|---|
| 1 | Roles in the participant list; promote and demote; the second-admin nudge; leave with "rotation pending" for the admins; add back with the history choice; the notice when two admins removed each other; each for a shared document and a shared workspace. Three desktop instances (A, B, C) and one server (criteria 1 to 36). | Must |
| 2 | The verified join: the invitee shows a code and a fingerprint, an admin compares and confirms, the invitee is added without a link (criteria 37 to 41, in outline). Also the way an admin admits a maker's new computer in a workspace where that maker is only a member, which `0096` slice 2 names. Needs ADR 0041. | Must (R-COLLAB-007) |
| 3 | Browser instance for slice 1 and 2 (same behaviour as desktop, `0096` slice 1b); an activity list per document (who joined, left, was promoted, with the key epoch) instead of notices only. | Should |

Which spec owns what (Proposal): this spec owns every way a participant enters, leaves or changes role in a keyring; `0096` slice 2 ("workspaces where I am only a member") is delivered here as the verified join in slice 2 and is removed from `0096`'s slice 2 row when this spec is Ready.

## Acceptance criteria (slice 1)

**Amendment (customer feedback on PR #281, 2026-11-09).** These customer decisions override the
criteria below where they conflict:
- Role acts, Remove and Leave are armed buttons with **no sentence**: the confirm texts of
  criteria 4 and 10 and the second-admin line of criteria 7 and 8 are gone. Step down and Leave
  of the only admin are greyed with the reason "You are the only admin" (criteria 5 and 14),
  never refused with a sentence after a press. The Share tab keeps labels, states and real
  errors only.
- The "Removed or left" list and Add back are dropped: criteria 17 to 26 and 33 no longer apply.
  The Removed block shows its heading only.
- 42 (new). Given B was removed from D or W, when an admin makes a new invite and B joins with
  it, then B joins again as a new participant and syncs; the old removal does not block it, and
  B's old link and file are still refused as removed.
- "Leave my devices" on the group's last device is allowed (`0096` criterion 27, ADR 0040
  decision 8).
- 43 (new, customer 2026-11-09). Given B is the only participant left of D or W (every other one
  left or was removed), then B's row offers Stop sharing (armed); it ends the share on B without
  writing anything and the file or folder is local again. The Share tab shows W's block, Invite
  and Participants also while no workspace is open in the tree. No key epoch is shown; an invite
  is always valid for 24 hours.

Setup as `0092` "Acceptance criteria": instances A, B and C of the desktop app, each with its own data directory and credential store, so each is a different maker; one `curvyo-sync-server`; the same server address in each. A also has a second computer A2, paired with A as in `0096` (own data directory). D is a shared document with A as admin, B and C as members (joined by invite link). W is a shared workspace (3 documents) with the same people. "Within 2 s" means on a LAN server with both online, as `0092` criterion 6 and `0096` criterion 16. Where the UI is named it is the Share tab; `ux-engineer` fills "UX notes". Every criterion holds for D and for W unless it names one. Criteria marked [ADR] depend on a mechanism ADR 0041 still has to decide.

### The participant list and roles

1. Given D is shared, then the Share tab lists every participant of D with label, fingerprint, role (Admin or Member), "you" on the own row, and the key epoch number of the document. Every participant sees all roles. Given W, the same list is shown in the Workspace block, once for the workspace; a document inside W shows "Shared through workspace W" and no list of its own.
2. Given the list, then an admin sees on every other row the actions Make admin or Make member (as fits), and Remove; a member sees none of them (as `0092` criterion 32). On the own row an admin sees "Step down" (becomes a member) and every participant sees "Leave".
3. Given A chooses Make admin on B, then A writes `SetRole{B, admin}`, and within 2 s B and C show B as Admin; B now sees the admin actions and can invite (link), remove and rotate. B's earlier edits and its connection are not interrupted.
4. Given A chooses Make member on admin B (after a confirm that names B, in one sentence: "B can no longer invite, remove or change roles; B keeps editing"), then within 2 s everybody shows B as Member and B's admin actions disappear. A writes `SetRole{B, member}`. Who may demote whom is Frage 148.
5. Given A is the only admin, when A chooses Step down, then it is refused with the reason "You are the only admin. Make someone else an admin first" and the action Make admin on the other rows is offered. The same holds if A tries to demote the only other admin of a room in which A is not an admin (not offered). Replay is the backstop and already drops a removal or demotion that would empty the admin set (`state.rs`, `admin_set_restored`); the client refuses first and a unit test names the linear case (one admin removes itself, no fork), which has no test today. [ADR]
6. Given A promotes B while C (admin, offline from A) removes B, when both reconnect, then B is not a participant (deny wins, ADR 0010 §5), A's and C's lists show it and nobody sees an error. Given the same with A promoting B and C demoting B, then B is a member.
7. Given the participant list of a document or workspace with exactly one admin device, when the maker opens the share dialog for the first time or later opens the list, then one line says "Make a second person an admin, so this can still be managed if you lose this computer" (Proposal: ADR 0010 §13, R-COLLAB-012), with a Make admin button on each member's row. It never blocks, can be dismissed per document (kept on this computer only), and is not shown once two admin devices exist.
8. Given the only admins are computers of one maker (A and A2, `0096` DEV-ROLE A makes every own device an admin), then the line of criterion 7 changes to "All admins are your own computers. If you lose them all, nobody can add or remove people", counts the admins that are not in the maker's own device group, and is shown until another maker's device is an admin (Proposal; the share dialog knows the own group from `0096`).
9. Given the keyring contains an identity the maker does not recognise as a person (no label sealed, or a label the maker did not see), then the row shows "A device" and its fingerprint, so a device that somebody slipped in is visible and removable (as `0096` decision 8).

### Leaving

10. Given B (member of D), when B chooses Leave, then B confirms a text that says in two short sentences what happens: "Leaving stops syncing this document on this computer and the copy stays here. Until an admin changes the keys, this computer still holds the key to what was shared so far." (wording is Frage 150). B's client writes its own `RemoveMember` and shows "Leaving…" until the server has confirmed the entry (ADR 0041 decision 11, Proposal); only then the file becomes an ordinary editable local file, no longer shared, and B sees "You left D. Your copy stays on this computer." A leave that was not sent (offline, closed) goes out on the next open and the file is shared until then. and no text anywhere says B "lost access" (a test searches the user texts of the share flow for "revoked" and "lost access" and finds none).
11. Given B left, then A and C show within 2 s that B is no longer a participant and one notice, "B left. The keys have not been changed yet." with a button Change keys now (admins) or "Waiting for an admin to change the keys" (members). The notice is derived from the keyring (the current epoch key is wrapped to a non-participant) and so survives restarting the app; it is shown to every admin, and the first admin who chooses Change keys now clears it for all. Exception to check [ADR]: in a workspace that an admin has switched on for the admin's own devices, `reconcile` rotates by itself when a current epoch is wrapped to a non-member (ADR 0040 decision 5), so criteria 11 and 12 then show "Keys changed" with no button; ADR 0041 says whether that rule applies only to the own group's devices or to any leaver.
12. Given an admin chooses Change keys now, then the client writes a `NewEpoch` wrapped to every remaining participant (not to B) and a fresh signed snapshot under it, as `0092` criterion 32 and ADR 0031 decision 6a; the notice disappears; C, online, continues across the rotation without action and loses no edit (`0092` criterion 34).
13. Given B's leave was confirmed by the server, then B's client makes no further connection attempt for D by itself and shows "You left D" (the room refuses a non-participant, ADR 0010 §10); how B comes back is criteria 20 and 25 and Frage 158.
14. Given A is the only admin (counted per device for the Leave of a document or of a workspace not synced to own devices, ADR 0041 decision 1; per person for the leave of a synced workspace, criterion 16) (Proposal) and other participants exist, when A chooses Leave, then it is refused as in criterion 5 ("Make someone else an admin first"). Given A is the only participant of D, then Leave is not offered; "Stop sharing" of a document or workspace is out of scope (Frage 149 for the option of allowing a freeze).
15. Given W and B leaves, then B keeps the folder with its documents as ordinary local files, the marker's share files are removed (ADR 0037 decision 9), the other participants see criterion 11, and no document of W is deleted anywhere.
16. Given B's computer is in a `0096` device group (B2, B3 are other computers of B's and "Sync to my devices" is on for W), then Leave means the person leaves (DEVICE-OR-PERSON A): B is told "W is synced to your other computers. Leaving removes all of them from W". The leave is refused unless an admin exists outside B's own devices (the last-admin rule of criteria 5 and 14 is counted per person, not per device: B's own computers together are one admin). On Leave, B's admin devices first remove each other, then each removes itself from W (a self-removal, so no admin of another maker is needed); a device of B that is only a member removes itself. B sets a `left` mark for W in the devices list, so a sibling's `reconcile` does not add B's devices again; the mark is cleared by "Sync again" or by switching W on for the devices. A per-computer leave is not chosen. [ADR]

### Adding back

17. Given A (admin) and B was removed from D or left, then A's list has a section "Removed or left" with one row per such identity: last known label (or "A device"), fingerprint and Add back. The identity comes from the keyring, which keeps the public keys of everyone ever added; the last known label is kept on this computer and may be missing. No other participant sees the section. [ADR] for where the label is kept.
18. Given A chooses Add back on B, then a dialog offers two choices with their consequence in observable terms ("Full history" is preselected, as ADR 0010 §9): "Full history: B receives the keys of every key period of this document that A holds", and "From now on: B receives the current key only. Key periods that began while B was away stay closed to B. B still holds the keys B had before, and can still read what was sealed with them, and the snapshot B starts from carries the document's change history." Keys only; nothing trims the history. Neither choice rotates keys, changes anyone else's access or touches B's role (B returns as Member, Proposal; Make admin is a separate action).
19. Given A confirms, then A writes `AddMember{B, member}` with the wraps of the choice and, for "from now on", a fresh signed snapshot under the current epoch for B to start from (ADR 0010 §9), within 5 s on a LAN server. A and C show B as Member within 2 s.
20. Given B's app is closed or offline when A adds B back, then A needs nothing more once "Added" shows on A's screen, which it does only when the relay holds the entry (ADR 0041 decision 11, Proposal), so B's check finds it; the check does not retry. How B learns of it is Frage 158; the default (B) sends nothing until B acts: the removed or left file shows "Check whether I was added back" (B presses it after being told by A), the client makes one connection attempt to the room, and if B is a participant again it shows "A added you back to D. Sync again?" with Sync again and Keep as it is. Nothing syncs from B before it chooses. The removed or left file keeps a small `share.ended.json` (server and room, no keys) in place of `share.json` and `keyring.log` (in a workspace, in the marker), and every build treats the file as local (Frage 161 default A); this changes `0092` criterion 33 and ADR 0037 decision 9 (the removed device dropped its share record). The check decides from the client's own replay of the keyring it fetches, not from the server's answer. [ADR]
21. Given B chooses Sync again and B edited the local copy while out, then B's edits merge as an offline edit would (`0092` criteria 17 to 19) and the dialog said so before ("Your changes made while you were away will be merged in"). Given B chooses Keep as it is, then B's device sends one leave entry (its own `RemoveMember`, as Leave, Frage 162 default A): the file stays local, the admins see "B left" and that the keys should change (criterion 11), and the list stays true; Add back can be shown again on the next check only if A adds B back again.
22. Given "full history" in D, then B can open every record the server still holds that is sealed under any epoch A holds, including epochs created while B was away (test: B's replica receives and opens a record sealed under the intermediate epoch k). Given "from now on", then A's client does not wrap epoch k (created while B was away, not the current one) for B and the repair step (`RewrapEpoch`) does not fill it in, so B's replica cannot open a record sealed under k and can open everything from the current epoch (a client rule in the re-wrap repair, Frage 160 default A; not a keyring rule, so a member or an older build can still pass the key or the content on, and the dialog says "the app does not give B the keys", never that B cannot read it). [ADR]
23. Given W, then Add back offers no history choice: a returning member always receives the keys of every key period the admin holds (WS-HISTORY B); `0095` criterion 46 is unchanged for first-time members. The dialog says what the returning member still holds (as criterion 18). B's kept folder with the same workspace id is offered "Connect this folder" as `0096` criterion 14, and the merge loses nothing on either side.
24. Given B was removed by an admin of another maker (A) and B has a second computer B2 that was in W through `0096`, when A adds back B, then exactly that device is added. B2 and any other device of B stay in "Removed or left" as their own rows, and `reconcile` on B's own admin device may add them only where `may_readmit` allows (ADR 0040 decision 5); A's Add back is never limited by it (ADR 0010 §8 allows it). Add back and Make admin act on one device only (DEVICE-OR-PERSON A).
25. Given B left on purpose (criterion 10) and A adds B back, then criterion 20 applies unchanged: B is asked, B decides; an admin cannot put a leaver back to work silently.
26. Given B was removed, A's screen shows "Added" for B (the relay holds the entry) and B pressed "Check whether I was added back", then B shows the prompt of criterion 20 within 5 s on a LAN server; after Sync again B has the content of the chosen history within 30 s for a document of 1,000 objects and for a workspace of 20 documents of 1,000 objects (as `0096` criterion 10).

### Two admins who remove each other

27. Given D has the admins A and C (and a member B), and A and C demote each other at the same time (or C demotes A and A's removal of C arrives after it), then both are still admins, both lists show a notice "A and C demoted each other at the same time. Nobody was demoted.", and the notice stays until the maker acknowledges it (Proposal, ADR 0041 decision 11). Given a mutual removal through the server, then the admin whose own removal arrived second is removed (the server confirms entries in order) and that admin sees "You were removed". The rule is ADR 0010 §5 in the crypto crate (the test `the_admin_set_never_becomes_empty` and the exhaustive ordering checks); this criterion is about what the maker sees.
28. Given the notice of criterion 27, then while A and C are the only admins, every further removal or demotion between them is dropped by replay and the list shows that ("Removal of C was not applied: you are the only admins"), not an error; with a third admin the dropped removals take effect, which would remove A and C both (`0096` debt "Mutual removal freezes a device group", Frage 146 default A, not asked again). Therefore Make admin on a third participant while such a notice stands warns inside the same confirm (UX MEM-WARN-ONE, no second dialog): "A and C demoted each other earlier. If you add another admin, both demotions apply and both lose the role." (Same trigger as criterion 27; the rule is unchanged.) A frozen room is visible to the client (`Keyring::held_back()`), and `reconcile` adds the maker's own new devices there only as members, never as admins (amends ADR 0040 decision 8), so pairing a new own computer or switching sync on cannot trigger the removals. [ADR]
29. Given A and C do the same in W (trigger as criterion 27), then the same notice appears in the Workspace block; B stays a participant of the workspace.
29a. Given any keyring act (role change, removal, add back, leave) is written while the writer is offline, then it reaches the room when the connection is back, in order, without any action (ADR 0041 decision 11, Proposal). Until the server confirmed it, the admin's list marks the row pending ("Adding…", "Removing…", "Leaving…"); after 10 s unconfirmed the row says "goes out when the connection is back". The success line appears only after confirmation. A test writes each act with the network off, restores it and finds the entry in the room.

### Notices, what the server sees, safety

30. Given the admin set changes (promotion, demotion, leave of an admin, removal of an admin), then every participant sees within 2 s one line in the Share tab and a mark on the Share tab icon until read: "B is now an admin", "B is no longer an admin", "B left". The notices are in the app only; no email, no OS notification, nothing sent anywhere (R-SYS-007). Where they appear is Frage 156.
31. Given the server's stored data after criteria 1 to 29, then it holds nothing new that is readable: the `SetRole`, leave, add-back and `NewEpoch` entries are public keyring entries (public keys, roles, wraps) as in ADR 0010 §10, the server can see that roles changed and when; labels, document and workspace names, "removed or left" labels and the dismissed state of criterion 7 are not on the server. This is stated in the privacy statement (one sentence: "The server can see who is an admin and when roles change, not names").
32. Given a document D or workspace W with a single participant admin and nobody else, then no criterion above makes any network request the file did not already make; an unshared file sends nothing (R-SYS-002, R-SYS-007).
33. Given a removed or left document, then it makes no network request until B presses "Check whether I was added back" (Frage 158 default B; with option A the background check is the only exception and has a "Don't check again" switch). Given Frage 158 C, no room reference is kept and Add back is a fresh invite link with the history choice.

### Wording and platforms

34. Given every user-visible text of the flows above, then it never says that access was "revoked" or that the person "can no longer read" without saying that what they already hold stays (ADR 0010 consequences; checked by a test over the texts of `ui-core`), and the removal and leave sentences are the ones of criteria 10 and 18 and `0092` criterion 32.
35. Given a document inside W, then the Share tab shows "Shared through workspace W", offers no Leave of its own, and the Leave of W applies to the whole folder (WS-VISIBLE A).
36. Given three instances on Linux, then the criteria are verified on Linux in the test; Windows and macOS run the same code and are verified in the nightly (`CLAUDE.md` §8).

## Acceptance criteria (slice 2, outline)

Not final until ADR 0041 exists (FP-FLOW A and FP-LINK A are decided); they are listed so that slice 1 does not rule them out.

37. Slice 2: Given D (or W) with admin A and a maker X who is not in the keyring, when X chooses "Join with a fingerprint" and enters the server address and D's join id (no link), then the admin's device shows the code and X types it, as in `0096` (the join reuses the ADR 0040 pairing connection, tagged `curvyo-join-v1`). X's client generates its identity if it has none, both screens show the fingerprint and the four check words, and X shows "Waiting for an admin". X sees no content. [ADR]
38. Slice 2: Given A is online, then A sees "X wants to join D" with X's name, fingerprint and the words, and Confirm and Decline; the comparison happens by voice or chat between the people. On Confirm A writes `AddMember{X, member}` with the wraps of a history choice (as criterion 18) and X joins within 10 s; no invite link existed and no secret travelled in a channel (ADR 0010 §7). On Decline nothing is written and X is told. [ADR]
39. Slice 2: Given a relay that swaps X's public keys on the way, then the check words or digits on A and X differ and A's Confirm is not given by a maker who compares them; a test with a swapping relay shows the difference in the shown phrase. Skipping the comparison is the maker's choice and the dialog says what the comparison protects. [ADR]
40. Slice 2: Given the link flow of ADR 0010 §7, then it stays (COOP-JOIN A): the share dialog offers both, and the verified join is shown first with one line: "Verified: you compare a fingerprint, no link can be forwarded" (Frage 154).
41. Slice 2: Given a maker M whose computer M2 is not yet in W where M is only a member, then an admin of M's own group cannot add it (M is not admin); an admin of W sees "M's new computer wants to join W" and adds it by the verified join, which delivers the `0096` slice 2 row "workspaces where I am only a member". [ADR]

## Out of scope

- Read-only or viewer roles, per-document roles in a workspace: cannot be enforced (ADR 0010 §12), WS-VISIBLE A. A viewer would be an export or a non-member viewer under its own ADR (Frage 147).
- Transferring "ownership" of a document: there is no owner in the keyring, only admins.
- Stop sharing a document or workspace whose last participant is its admin, and dissolving a share for everybody: later story.
- Merging two device groups, a person model that groups devices under one identity, and accounts (ADR 0010 answer 2, ADR 0007).
- A recovery path when all admins lost their keys: ADR 0010 §13 names it as a hard limit (R-COLLAB-013); this spec only makes the second admin easy to add.
- Forward secrecy and making already-received content unreadable: not possible, stated in the wording.
- Share by name or e-mail, a contact list, "people you shared with".
- Browser behaviour: slice 3; nothing here stops it.
- Notifications outside the app, an audit log kept on the server, activity export.
- Changing the keyring payloads or rules of ADR 0010. If slice 1 needs a new payload, the architect says so in ADR 0041 and the customer decides.

## UX notes

(to be filled in by ux-engineer before Ready)

Hints from the product owner, not decisions: the participant list is one table, one row per device, role as a small badge, the actions in a row menu; "Removed or left" is a collapsed section under it, visible to admins only; the leave and add-back dialogs are one-sentence-per-consequence, no checkbox maze; the second-admin line is quiet text with a button in the row, not a modal; notices live in the Share tab header; the two-admins notice stays until acknowledged because it says something unusual happened.

## Customer decisions

- 2026-11-05: membership management is next (NEXT A): second admin, leave, add back with history choice, verified-fingerprint join, concurrent removal; for documents and workspaces; consistent with `0096`.
### Customer decisions (2026-11-05)

- READD-DEFAULT: A. Adding someone back to a shared document preselects "Full history", as ADR 0010 §9 (criterion 18).
- WS-HISTORY: B. A shared workspace gets no history choice: a returning member always gets everything (all key periods the admin holds). `0095` criterion 46 stays as built; this spec no longer amends it (criterion 23).
- HIST-MEANING: obsolete. ADR 0010 §9 semantics stay, keys only; no shallow-snapshot follow-up. The dialog keeps the honest wording of what a returning person still holds (criteria 18 and 22).
- ADR-VORSCHLAEGE A: all Proposals for the questions on roles (ROLES, Frage 147: two roles, no viewer), who promotes (WHO-PROMOTES, 148: any admin promotes and demotes any participant, with a confirm), last admin (LAST-ADMIN, 149: refused until another admin exists), leave wording (LEAVE-WORDING, 150: A for documents, folder wording for workspaces), the verified join (FP-FLOW, 153: reuse the `0096` code and four check words; FP-LINK, 154: complements the invite link), one device or person (DEVICE-OR-PERSON, 155: per device, leave of a synced workspace per person) and notices (NOTIFY, 156: one in-app line and a mark on the Share tab icon) are accepted.
- REJOIN-PATH (158): not answered, default B applied (no background traffic; "Check whether I was added back").

### Carried over

- ADR 0010 (two roles, no read-only; leave is a roster act; deny wins; admin set never empty; full history or from now on); COOP-JOIN A (a link join is completed by any online member); WS-VISIBLE A; DEV-ROLE A (a device inherits the admitting device's role); Frage 146 GROUP-DEADLOCK, default A.

## Open questions

Four questions are open, each with its default applied: Frage 158 (the customer did not answer it on 2026-11-05) and Frage 160 to 162 (raised by ADR 0041). They change accepted behaviour or formats, so the customer may still override them at the demo. Every other question of the first draft (Frage 147 to 157) is answered, see "Customer decisions (2026-11-05)".

**Frage 158 (REJOIN-PATH): How does a removed or left person find out they were added back?** (Changes accepted behaviour: `0092` criterion 33 and ADR 0037 decision 9 drop the share record on removal; and data leaves the machine.)
- A: The file checks in the background (on start, on opening, every 10 minutes) and prompts.
- B: No background traffic. The file keeps its room reference and shows "Check whether I was added back"; B presses it after A tells B outside the app.
- C: No retained reference. Add back takes effect through a fresh invite link that carries the history choice.
- Recommendation: B (sends nothing until B acts; no accepted behaviour changes beyond keeping the reference). Default applied: B (criteria 20, 26, 33). [ADR]

**Frage 160 (FROM-NOW-ON): What may "From now on" promise when someone is added back?** (ADR 0041)
- A: The app does not give the person the keys of the time they were away; anyone still in the share could pass them on outside the app. A client rule, no keyring change, ships in slice 1.
- B: The keyring refuses those keys to the person (changes ADR 0010's rules; server and older builds disagree about valid entries until updated).
- C: No choice for documents; adding back always gives full history.
- Recommendation: A. Default if unanswered: A (applied in criteria 18 and 22).

**Frage 161 (ENDED-RECORD): Where does a removed or left file keep the way back?** (ADR 0041)
- A: In the file, as a small new part `share.ended.json` (server and room, no keys); older builds ignore it, and if they save the file the way back is an invite link again.
- B: The file stays "shared" and a list on this computer says "do not connect"; an older build or a lost list reconnects unasked.
- C: Only on this computer, by room; a removed document cannot be found again, only workspaces.
- Recommendation: A. Default if unanswered: A (applied in criterion 20).

**Frage 162 (DECLINE): What does "Keep as it is" do after someone was added back?** (ADR 0041)
- A: The device leaves again (one keyring entry, as Leave); the admins see "B left" and that keys should change.
- B: Nothing is sent; the person stays listed as a member and holds the current key though they never sync.
- Recommendation: A. Default if unanswered: A (applied in criterion 21).

Not asked, because a safe Proposal is marked: the Removed-or-left section for admins only (17), Add back returns a Member (18), the two-tier second-admin line (7, 8), the warning before promoting a third admin (28), the choice for a leaver to come back is the leaver's (25).

## Links

Requirements: R-COLLAB-007 (verified join), R-COLLAB-009 to R-COLLAB-013, R-SYS-002, R-SYS-007 in `docs/requirements.md`; ADR 0010 §4, §5, §7, §8, §9, §12 and §13, ADR 0031 decisions 6a and 8, ADR 0037 decision 9, ADR 0040 decisions 5 and 8 in `docs/adr/`; ADR 0041 (reserved for this story, not written); `specs/0092-cooperative-editing/` (criteria 41 to 45 move here), `specs/0095-workspace/`, `specs/0096-own-devices-sync/`; `docs/technical-debt.md` ("Mutual removal freezes a device group", "Joining needs a member online", "Losing every admin key freezes a document's membership"); crypto tests in `curvyo-crypto-core/tests/keyring_forks.rs`.
Field sources: [Signal support, groups](https://support.signal.org/hc/articles/360007060712); [EFF, how to use Signal (safety numbers)](https://ssd.eff.org/module/how-to-use-signal); Google Drive "Transfer file ownership"; Figma and Notion community answers (not verified in the official help).
PR: none yet.
