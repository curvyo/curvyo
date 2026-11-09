# Sync across the maker's own computers

Status: Done (PR #270, PR #298): slice 1 desktop and browser; the switch and Share workspace merged into one Sync switch by `0105`
Priority: Must
Origin: Customer (request of 2026-11-04, translated): sync the workspace across my own computers, slice 2 of `0095`. The maker pairs a second computer once with the first; afterwards the workspaces sync automatically through the server without an invite link, still end to end encrypted, and the server sees nothing but ciphertext (WS-META of `0095`). Standing requirements: R-SYS-004, R-COLLAB-003 (always available, not conditional on a sync folder), R-COLLAB-006. What is marked Proposal is the product owner's until the customer accepts it.

## User value

As a maker who works on a desktop in the workshop and a laptop at home I want to pair my second computer with my first once, so that my workspaces appear on it and stay in sync, online and offline, without invite links, accounts or passwords, and so that a lost or sold computer can be cut off again in one action.

## Problem today

- `0092` shares one document with another maker; `0095` shares a whole workspace with another maker. Both go through an invite link and an online member who admits the joiner (COOP-JOIN A). Used for my own second computer this is a ceremony per workspace, and it makes me invite myself.
- R-SYS-004 and R-COLLAB-003 (both Must) say cloud sync of my own computers is a standing capability. Nothing of it exists: a project on the desktop is not on the laptop unless I copy the folder by hand.
- ADR 0010 decision 1 made every device a participant, not a person, and has no link between two devices of one maker. The customer accepted that on 2026-10-02 and said to revisit it "only if work across more than two or three machines makes the device model awkward". DEV-IDENTITY A keeps the device model and links devices by pairing.
- A workspace that is not shared has no room, so "sync it to my devices" also means making it a shared workspace whose only members are my own computers.
- Known debts that apply: joining needs a member online (`docs/technical-debt.md`, "Joining needs a member online"); the browser has no key custody ("Browser key custody is undecided") and no folders (`0095` Out of scope), yet the customer wants the browser to be a device (DEV-BROWSER B).

## The field

- **Obsidian Sync**: a paid account; per remote vault, each device picks what to sync (folders, file types); end-to-end encryption needs a separate encryption password per vault, and a forgotten password loses the data. No device approval: whoever has the account and the password is in. Lesson: selective sync per vault is simple and understood; a password per vault is a trap. We do better: no account, no password, and a device can be removed.
- **1Password**: account plus a Secret Key that exists only on the maker's devices; a new device is set up by scanning a QR code from a signed-in device, which hands the Secret Key over. Lesson: a code shown on the old device is the accepted way, and the key is copied. We copy the code flow but not the key: each computer makes its own keys.
- **Bitwarden "Log in with device"**: the new device asks, the signed-in device approves and both show the same fingerprint phrase to compare. Lesson: the phrase comparison is what defeats a swapping server; we show one too.
- **Signal linked devices**: the desktop shows a QR code, the phone scans it; every device has its own identity keys; the phone can unlink any device. Lesson: per-device keys linked once, unlink from the other device. Closest to our model (ADR 0010 decision 1).
- **Syncthing**: no server; devices exchange device ids and both sides must accept; each folder is shared per device explicitly. Lesson: explicit per-folder choice, explicit accept on both ends. We add the relay (works across networks and when one computer is off) and the sealed state on it.

What we do better: nothing to sign up for or remember; the server holds only ciphertext (customer, WS-META); one lost computer is removed with key rotation while the others keep working; the unit synced is the workspace of `0095`, whose files stay ordinary `.curvyo` files usable without Curvyo (R-SYS-002).

## Words

- **My devices**: the computers a maker paired with each other. A device is a participant with its own keys (ADR 0010 decision 1) and a label the maker chose.
- **Pairing**: the one-time act of linking a new computer to a paired one, with a one-time code (Frage 136).
- **Synced workspace**: a workspace with the switch "Sync to my devices" on. It is a shared workspace (`0095`, workspace room, keyring, epoch keys) in which every one of my devices is a member.
- **Synced workspaces folder**: the folder on a desktop computer under which workspaces that arrive by sync are created, one subfolder each. Chosen once per computer at pairing (Proposal).
- **Browser device**: the app in a browser tab, paired like a computer. Its device keys, epoch keys and synced workspaces live in the browser's site storage for that origin (Proposal, ADR 0040 decides); clearing site data destroys them, which is a lost device (criterion 36).
- Participant, member, admin, epoch, rotation, removal: as `0092` and ADR 0010.

## Slicing

| Slice | What the customer can try | Priority |
|---|---|---|
| 1 | Pair B with A with a code; A switches "Sync to my devices" on for chosen workspaces; they appear on B and stay in sync live and across offline edits; a third computer can pair through B; Remove a computer (also a lost one) with key rotation; a computer can leave on its own. Desktop to desktop, two or three instances, one server (criteria 1 to 32). | Must |
| 1b | The browser as a paired device (customer, DEV-BROWSER B): a browser pairs from a desktop and a desktop from a browser, keys in the browser's storage, synced workspaces kept in browser storage, same sync and removal (criteria 33 to 40). Needs ADR 0040 (key custody, maybe accounts, see "Open dependency") and a browser workspace without folders. Proposal: built after slice 1 as its own PR; the lead may cut differently. | Must (customer) |
| 2 | Workspaces in which the maker is only a member (an admin elsewhere has to admit the new computer); documents opened as single files that are shared on their own (`0092`); recovery code (ADR 0008 decision 9). | Must (R-SYS-004) |
| 3 | The material library syncs the same way (R-MAT-005, R-SYS-004); needs `0028` and slice 3 of `0095`. | Should |
| 4 | The mobile companion. | Could |

## Acceptance criteria (slice 1)

Criteria 1 to 32: desktop app only; the browser device has its own criteria 33 to 40. Setup as `0092` "Acceptance criteria": instances A, B and C of the desktop app, each with its own data directory and credential store (so each behaves as a different computer), one `curvyo-sync-server` started with one command, the same server address setting in each. Unless a criterion says otherwise, A holds two local workspaces W1 (3 documents) and W2 (20 documents of 1,000 objects) that were never shared. Criteria marked [ADR] depend on the mechanism an ADR still has to decide (the architect writes it before Ready). Where the UI is named it is the Share tab (DEV-UI A); `ux-engineer` fills "UX notes". Criteria 12, 13, 16 and 17 apply SYNC-SCOPE B and DEV-ROLE A.

### Pairing

1. Given A has a server address set and no paired device, when the maker chooses "Add my computer" in the "My devices" block of the Share tab, then A shows a one-time pairing code (Proposal: 8 characters in two groups of four, from an alphabet without look-alike characters), the server address, a countdown of 5 minutes (Proposal), and one sentence: "The computer that enters this code can sync the workspaces you choose, and you can remove it again." [ADR]
2. Given the code, when the maker chooses "Join my devices" in the Share tab of B (same server address) and enters it, then B asks for a computer name if none is set (default: the host name), asks for the Synced workspaces folder (default `~/Curvyo`, created if absent, never inside a workspace), shows "Waiting for <A's name> to confirm", and shows a check phrase of four words. A shows B's name, B's fingerprint and the same four words, with Pair and Cancel. B sees no content before A presses Pair. [ADR]
3. Given A presses Pair, then within 10 s on a LAN server both show each other in My devices with name, fingerprint and "this computer" on their own row; B has generated its own device keys on B and no private key was sent to A or the server (a test searches captured traffic and server data for B's private key bytes and finds nothing). [ADR]
4. Given a code that was used once, expired after 5 minutes or entered wrongly 5 times in a row, when C enters it, then C is refused with the reason (used, expired, locked) and sees no content; the server's stored data gained nothing readable from the attempt.
5. Given no reachable server, then "Add my computer" and "Join my devices" are unavailable with the reason named; the app stays fully usable offline (R-SYS-002). Given A is closed when B enters the code, B keeps waiting and does not pair on its own; pairing needs both computers online once (debt "Joining needs a member online" stays).
6. Given A and B are paired, when the maker chooses "Add my computer" on B and C enters the code, then A and B both list C within 30 s of both being online; C is paired with all devices, not only with B. [ADR]
7. Given a pairing, then no account, email address or password is asked for anywhere (ADR 0007 hard boundary), and nothing is sent to any server before the maker chose "Add my computer", "Join my devices" or a sync switch (R-SYS-007).

### Choosing and syncing workspaces

8. Given A and B just paired, when A confirms, then A shows its open and recent workspaces as a list with a check box each, all checked (Proposal), and "Sync these" / "Not now". "Not now" syncs nothing and can be done later from the Workspace block of the Share tab.
9. Given a workspace W with the switch "Sync to my devices" off, when the maker turns it on in the Workspace block of the Share tab on A, then, after one confirm that says "names, folders and files leave this computer encrypted, to your own computers only", A (a) makes W a shared workspace as in `0095` criterion 25 if it is not one (genesis keyring entry, workspace room, one sealed snapshot), (b) admits every paired device of the maker as a member without any link or prompt, (c) shows each paired device with its sync state in the Share tab.
10. Given B is online when W is turned on, then W appears on B as a folder `<Synced workspaces folder>/<W's name>` with the same tree of documents within 30 s for W2 (20 documents of 1,000 objects) on a LAN server (Proposal, as `0095` criterion 28), a notice on B says "Workspace W arrived", and the tree shows documents as they arrive with a count. No input on B is needed. A name that exists in B's folder gets " 2".
11. Given B is closed when W is turned on, when B is started later and connects, then W appears as in criterion 10 without any further action on A. A needs not be online (A already holds B's public keys from pairing and has written B's wrap). [ADR]
12. Given a workspace W3 whose switch is off, then nothing about W3 reaches the server: no room, no record (checked in the server's data directory), and W3 does not appear on B. A workspace created, opened or copied afterwards is off until the maker turns it on (SYNC-SCOPE B).
13. Given W is synced, when the maker turns the switch off on A ("Stop syncing W"), then A removes every paired device from W as in criteria 21 and 22, W stays on A as a workspace shared with whoever else is in it (nobody, if only own devices), and on B the folder and its documents stay as local editable files with a notice. Turning it on again later admits B again; B is asked whether to connect its old copy (criterion 14).
14. Given B already holds a folder that is a copy of W (same workspace id, `0095` criterion 2) when W is turned on, then B offers "Connect this folder" or "Create a second copy", and "Connect" merges B's files with the room as an offline edit would (`0092` criteria 17 to 19); nothing on either side is lost. (Proposal)
15. Given a document name the target OS refuses (for example `a:b.curvyo` on Windows), then B writes the file under a safe replacement name, tells the maker, and keeps the document id; renames on either side still reach the other (`0095` criterion 36). (Proposal)
16. Given W is shared with another maker as well (A is admin, a third maker M is a member), when W is turned on for my devices, then B is added with the role A has in W, M's participant list shows B within 2 s as "<name> (<computer name>)" and nothing else about it, and M does nothing. [ADR]
17. Given W is shared and A is only a member (not an admin) of W, then the row in the Share tab says "You are not an admin of W; ask an admin to add this computer", syncs nothing for W and raises no error (slice 2 delivers the admission).

### Staying in sync

18. Given W is synced and both A and B are online, when A edits a document, then B sees it with the guarantees of `0092` criterion 6 (300 ms p95) for the open document and `0095` criterion 35 (2 s) for the document list, and a document created, renamed, moved or deleted on one computer is created, renamed, moved or goes to the OS trash on the other, as `0095` criteria 35 to 40.
19. Given A and B edit the same document at the same time, or different documents, then both converge without loss; frame time caused by sync stays within `0092` criterion 13 (16 ms of thread CPU) for a workspace of 50 documents (`0095` criteria 31 and 32 hold; sync of my own devices adds no second connection per workspace).
20. Given B is offline for ten minutes (network off) and edits three documents while A edits two, when B is back online, then both converge within 30 s with no lost edit and no duplicated document (`0092` criteria 17 to 19, `0095` criterion 33). Closing the app and starting it later restores the connection of every synced workspace without any action.
21. Given the server is restarted while A and B are connected, then both reconnect within 30 s and lose no edit (`0092` criterion 20).
22. Given My devices, then each row shows name, fingerprint, "this computer", last seen (time of its last connection) and per workspace one of "In sync", "Syncing n of m documents", "Offline, n changes waiting"; the title bar and the File tab's header line show the same state of the open workspace (`0095` criterion 25e).

### Removing a computer, a lost computer, leaving

23. Given A (admin of every synced workspace) and B in two synced workspaces, when the maker chooses Remove on B's row in My devices (or "Remove a computer I no longer have"), then, after a confirm that says in one sentence that removal does not reach back to the copies on that computer, A writes for each synced workspace in which A is admin, in this order, a `RemoveMember` entry, a `NewEpoch` with a fresh key wrapped to every remaining participant and a fresh signed snapshot under the new epoch (as `0095` criterion 43), as one action with a progress count "2 of 2 workspaces". [ADR]
24. Given B was removed, then the server refuses B's connection to each of those rooms with the reason "removed"; no envelope written afterwards can be opened with B's keys; B (when online) shows "This computer was removed from your devices. Your copies stay here as local files." and keeps all documents as editable local files, no longer a synced workspace. A third computer C that was online continues editing across the rotation without action and loses no edit; C offline during the rotation seals its offline edits under the new epoch when it reconnects (`0095` criterion 44).
25. Given B is offline during the removal, when B reconnects, then B learns of the removal from the keyring, is disconnected, and its offline edits are not merged (`0095` criterion 45).
26. Given a synced workspace in which the removing device is not an admin, then B stays a member of it, the removal dialog names the workspace and why ("Ask an admin of W to remove this computer"), and the other workspaces are still done.
27. Given B chooses "Leave my devices" on itself, then B writes its own `RemoveMember` for each synced workspace, forgets the epoch keys of those workspaces and its pairing, and keeps the files as local; the next time A connects as admin it rotates the keys of those workspaces without a prompt, because B still holds the current key until then (ADR 0010 decision 8); A's My devices says "Rotation pending" until done. [ADR]
28. Given the maker turns the first sync switch on while only one computer is paired, then one sentence says "If you lose this computer, the copy on the server cannot be opened by anyone. Pair a second computer to have a backup" (a notice, no block). (Proposal)
29. Given DEV-LOST (default A, customer did not answer): slice 1 has no recovery code, and the only "Restore" action is a browser device's "Restore with passkey" (Frage 145 A, criterion 36). A maker who lost every computer and has no passkey backup still has the plaintext files on any disk that holds them; the server copy cannot be opened (ADR 0010 decision 13). The recovery code of ADR 0008 decision 9 is a slice 2 story. (Proposal, amended 2026-11-04 to the built behaviour.)

### What the server sees, keys, platforms

30. Given the server's stored data after criteria 1 to 22 (database files, logs, captured request bodies), then none of these is found: a workspace name `ZQX-WS-7731`, a document name `ZQX-DOC-7731`, a computer name `ZQX-PC-7731`, the pairing code, a text marker inside a document, the number of documents or folders. The server sees, as in ADR 0037 decision 11 and ADR 0010: one room per synced workspace, record sizes and times, the keyring (public keys and fingerprints of my devices) and which of those identities connect when. It can therefore see that two identities always appear in the same rooms. This exposure is stated in the Share tab's confirm of criterion 9 and in the privacy statement. Sealing cannot be disabled (ADR 0008).
31. Given each computer, then its device keys and epoch keys are kept in the OS credential store or the encrypted key file (ADR 0007 decision 1, ADR 0031 decision 12), never in `.curvyo-workspace/` or any `.curvyo` file.
32. Given three computers on Linux in the test, then the behaviour above is verified on Linux; Windows and macOS run the same code and are verified in the nightly (`CLAUDE.md` §8), not by hand in this slice. Cross-OS differences in file names are criterion 15.

### The browser as a paired device (slice 1b; customer decision DEV-BROWSER B)

Setup: as above, plus one browser instance R (the app in a browser tab, own browser profile) and the same server. All criteria of pairing, sync, offline, removal and server visibility (1 to 28, 30) hold for R as for a desktop, with the differences below. The mechanism and where the keys live are decided in ADR 0040; the customer's remark that browser key custody may force accounts is an open dependency (below), not decided here.

33. Given R and a paired desktop A, when R enters a code shown by A ("Join my devices" in R's Share tab), or A enters a code shown by R ("Add my computer" in R), then pairing completes as in criteria 1 to 3 with the same check phrase, and the browser's name defaults to the browser and OS name. No Synced workspaces folder is asked in the browser. [ADR]
34. Given R is paired, then R's device keys and the epoch keys of every synced workspace are generated and kept in the browser's site storage for that origin, never exported in plaintext and never sent to the server or to A; a reload or restarting the browser keeps R paired and connected without a new pairing. The Share tab says, at pairing, in one sentence that clearing this site's data removes this device; once R holds a passkey backup (criterion 36), the own row says instead that clearing site data does not lose this device. The only copy of R's identity that leaves the browser is the passkey backup, sealed with a key from the passkey and unreadable to the server (ADR 0040 decision 13). A stored key record that does not open is refused and never replaced by a new identity. [ADR] (Proposal, amended 2026-11-04 to the built behaviour.)
35. Given W is synced and R is paired, then W appears in R as a workspace held in browser storage with its document list, and its documents open and edit in R with the guarantees of criteria 18 to 21 (300 ms p95 for the open document, 2 s for the list, offline merge, reconnect). Whether R holds every document of W or loads them on demand is not decided here. Download of a document or the workspace as `.curvyo` files is not part of this slice. [ADR]
36. Given R's site data is cleared (or the browser profile is deleted) and R has no passkey backup, then R is a lost device: it loses keys, pairing and local copies; the rest of my devices do not notice by themselves. The maker removes R's old row from A with Remove (criteria 23 and 24), and pairs the browser again as a new device, which appears under a new fingerprint. Given R made a passkey backup ("Back up with passkey", Frage 145 A), then after clearing, "Restore with passkey" with the same passkey and server brings R back as the same device (same fingerprint, back in the group without pairing, its synced workspaces arrive again); a browser that is in a group refuses a restore. A removed browser device shows the notice of criterion 24 and no longer holds the keys of the new epoch; its backup then restores nothing usable. (Proposal, amended 2026-11-04 to the built behaviour.)
37. Given the browser can store only a limited amount, when the synced workspaces of R exceed what the browser allows, then R says so with the numbers and which workspaces it holds, and does not lose or truncate any document silently (Proposal; the storage quota is the browser's, not ours).
38. Given R is paired, when the maker switches a workspace on in R, then W is made a synced workspace as in criterion 9 and appears on every paired desktop as in criterion 10. A browser only switches workspaces it already holds (the synced workspaces in its browser storage); it has no local workspace of its own to start one from, so a workspace starts on a desktop. [ADR] (Proposal, amended 2026-11-04 to the built behaviour.)
39. Given two browsers on different origins or profiles, then each is its own device; two tabs of one browser profile are one device (one set of keys, one connection per workspace, no double sync). [ADR]
40. Given the server's stored data after R's pairing and sync, then criterion 30 holds unchanged for R, and no account, email or password was asked in the browser either (R-SYS-007, ADR 0007) unless the customer decides otherwise on the open dependency below.

## Out of scope

- Workspaces in which the maker is only a member of someone else's shared workspace (slice 2); admission there needs an admin of that workspace.
- Documents opened as single files, shared or not (slice 2). A plain `.curvyo` file outside a workspace never syncs.
- The mobile companion (slice 4). Download of workspace files from the browser, a browser workspace that is not a synced workspace, and several browser profiles sharing keys.
- The material library and shared assets (slice 3, R-MAT-005, after `0028`).
- Pairing without a server (LAN-direct, Bluetooth, QR by camera). A QR code is a presentation of the same code and belongs with the mobile companion.
- Folder sync with Nextcloud or similar (R-SYS-004 names it as an additional option; it needs no code from us, and nothing in this spec stops it).
- Settings sync (shortcuts, recent files, per-user font library), the machine profiles of `0026`, and the credentials of a machine connector (`0064`); each would be its own story.
- Per-device or per-document filters, read-only devices (cannot be enforced, ADR 0010 decision 12), a "this computer sees only W1" mode other than not switching W2 on.
- Selective sync of single documents, history choice for a new device (every device receives the current state; `0092` slice 2 owns the history choice), conflict handling for a folder synced by another tool.
- A hosted instance of the server, TLS in the server, quotas (debts of `0092`).
- A maker account, an email, a password, server-side recovery (ADR 0007; out unless the customer decides otherwise on the open dependency below).

## UX notes

(to be filled in by ux-engineer before Ready)

Hints from the product owner, not decisions: the Share tab gets a "My devices" block above the Workspace block with the list of criterion 22, "Add my computer" and "Join my devices" buttons, and in the Workspace block one switch "Sync to my devices"; pairing is a short two-screen flow with the code large and the four check words large on both sides; Remove sits on the device row with the one-sentence truth of criterion 23.

## Customer decisions

- 2026-11-04: sync across the maker's own computers is next (NEXT A), slice 2 of `0095`; pairing once, then automatic through the server, no invite link, end-to-end encrypted, the server sees nothing but ciphertext.
- Carried over: WS-META (`0095`), COOP-JOIN A (`0092`), a participant is a device (ADR 0010 answer 2, 2026-10-02), no accounts (ADR 0007), no read-only role (ADR 0010 decision 12).

### Customer decisions (2026-11-04)

- DEV-IDENTITY: A. Each computer keeps its own keys; pairing links them (criteria 1 to 6). No maker key, no server account.
- PAIR-METHOD: A. One-time code shown on the old computer, typed on the new one, four check words on both screens (criteria 1 and 2).
- SYNC-SCOPE: B. "Sync to my devices" per workspace, all existing ones offered and checked at pairing, new ones start off (criteria 8, 9, 12).
- DEV-ROLE: A. A paired computer gets the role of the computer that admits it (criterion 16); a workspace where the maker is only a member does not sync in slice 1 (criterion 17).
- DEV-LOST: not answered, default A. No recovery code in slice 1 (criteria 28, 29).
- DEV-UI: A. Share tab: "My devices" block and the switch in the Workspace block.
- DEV-BROWSER: B. The browser pairs as a device and keeps workspaces in browser storage (criteria 33 to 40, slice 1b). Customer's remark: "Browser könnte ein ausschlaggebender Punkt sein, um doch Accounts machen zu müssen." Recorded as the open dependency in "Open questions"; ADR 0040 evaluates it and the architect raises the question.

## Open questions

No customer question is open. One dependency is open and is not decided here:

**Open dependency: browser key custody may force accounts (ADR 0040).** The customer remarked (2026-11-04): "Browser könnte ein ausschlaggebender Punkt sein, um doch Accounts machen zu müssen." A browser device keeps its keys in site storage that the maker or the browser can clear (criteria 34, 36), and it has no OS keychain or recovery file. The architect evaluates in ADR 0040 whether browser key custody without an account is enough (non-extractable WebCrypto keys in IndexedDB, a passphrase-wrapped file, re-pairing after loss) or whether an account is needed, and raises it as a customer question. Until the customer decides: no account in this spec (criteria 7 and 40), the browser is a device with its own keys, and losing site data is a lost device (criterion 36). If an account is chosen, DEV-IDENTITY A, criteria 1 to 7 and ADR 0007 are revisited and this spec is amended before slice 1b is built; slice 1 (desktop to desktop) does not depend on the answer.

Not asked, because a safe Proposal is marked: pairing code format and its 5 minutes (1), the check phrase of four words (2), the default Synced workspaces folder (2), all boxes checked at pairing (8), connect an existing copy (14), safe replacement of illegal file names (15), automatic rotation after "Leave my devices" (27), the one-computer notice (28).

## Links

Requirements: R-SYS-004, R-COLLAB-003, R-COLLAB-006, R-SYS-002, R-SYS-007, R-MAT-005 (slice 3), R-SYS-012 (pairing and my devices) in `docs/requirements.md`; ADR 0004 §5, 0007, 0008 decision 9, 0009, 0010 decisions 1, 8, 12 and 13, 0031, 0037, 0040 (accepted 2026-11-04) in `docs/adr/`; `specs/0092-cooperative-editing/`, `specs/0095-workspace/`; `docs/technical-debt.md` ("Joining needs a member online", "Browser key custody is undecided").
PR: none yet.
