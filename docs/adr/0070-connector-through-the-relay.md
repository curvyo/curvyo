# ADR 0070: Reaching a machine connector through the relay

**Status:** Accepted (customer, 2026-11-17: Frage 218 CONN-RELAY A, Frage 219 CONN-SHARE B,
Frage 220 CONN-LAN A with the addition "a stand-alone connector would then also include a relay
server", Frage 221 CONN-REMOTE-START C; see "Customer answers" at the end). Proposed 2026-11-17
(architect draft). `needs-customer` (answered): persistence/sync and security model (a second
kind of room on the relay), data leaving the machine (jobs cross the internet, sealed), a
reversal of the customer's TLS preference of Frage 120 for the relay path, and sharing (kept to
the workspace, ADR 0044). The answers changed decisions 1, 2, 7, 8, 9 and the Consequences
before acceptance; decision 2a is new. Asked by the customer's feedback on `specs/0064-machine-connector` (2026-11-17,
translated): "Of course one can connect to the connector directly, but it must also work through
a relay server so one can use one's connector from anywhere; this would be like 'add device' for
the instance. Full cloud must be possible. For that the connector has to register with the relay
and be securely shared there." Extends ADR 0035 §8 (browser anywhere, left open there), amends
ADR 0004 §4 (who holds the machine lease). Choices marked (Proposal) are the architect's.

## Context

ADR 0035 built the LAN path: TLS 1.3, the connector's key pinned at pairing (SPAKE2 over a code),
a per-profile bearer token, `--revoke` on the connector's host. It works when the app and the
connector see each other. It does not give: reach from outside the workshop without opening a
port (spec `0064` criterion 8), a browser path without the self-signed warning (Frage 121 A),
sharing a machine with another maker, or revocation from the app. The customer wants the
connector to be enrolled "like a device" and shared through the relay.

What already exists and must not be duplicated: device identities (ed25519 + X25519, ADR 0010
§1), rooms with a signed keyring, roles admin/member, removal with key rotation and the relay
closing a removed member's connection (ADR 0031 6a), pairing over the relay with CPace, check
words and forward-only slots (ADR 0040 decisions 3, 4), the device group and its `reconcile`
(ADR 0040 decision 5), the relay that forwards bytes it cannot read (`PairMsg`, `Awareness`). The
relay stays blind: nothing but routing state and ciphertext (WS-META, ADR 0008 §1).

Fixed points: the connector holds no document key (ADR 0035 option D); one job at a time and the
job keeps running when a client goes away (ADR 0035 §7); `curvyo-connector-core` is the shared
protocol of app, browser and connector; no accounts (ADR 0004 §10, ADR 0007); no telemetry
(R-SYS-007); self-hosting is a supported configuration (ADR 0004 §13).

## Options

**A. The connector is an enrolled member of its own machine room; an end-to-end channel through
the relay (recommended).**
- *Identity.* On first start the connector makes a device identity (the same `crypto-core`
  `Identity` as every device). Enrolment ("Add machine", beside "Add my computer") is the ADR 0040
  pairing over the relay: the connector opens a slot and prints the code and the four check
  words; the app types the code; CPace, confirmation and sealed hellos as ADR 0040 decision 4 with
  its own binding label `curvyo-enrol-machine-v1`, so a device pairing and a machine enrolment
  can never be confused. The app then writes `Genesis` of a new **machine room** (admins = the
  enrolling device) and `AddMember{connector, role: member}`, and a sealed **machine record**
  (name, the connector's identity, its LAN fingerprint if it also listens on the LAN). The code
  and the check words replace the SPAKE2 pairing, the identity key replaces the pinned SPKI, and
  keyring membership replaces the bearer token; on this path nothing else is needed.
- *Transport.* The connector keeps one outbound `wss://` connection to the relay, admitted to its
  machine room by `Hello`/`Proof` like any member (criterion 8: no inbound port). A client in the
  same room opens a **channel** to the identity the machine record names: new relay frames
  `ChannelOpen{target}`, `ChannelMsg{channel, bytes}`, `ChannelClose{channel, reason}` (relay
  protocol 4). The relay routes them between two admitted members of one room, like `PairMsg`:
  in memory, never stored, never logged.
- *End to end.* Inside the channel, a Noise `KK` handshake (`Noise_KK_25519_ChaChaPoly_SHA256`)
  on the two identities' X25519 keys, which both sides already know from the keyring; prologue
  `"curvyo-connector-channel-v1" ‖ machine room id ‖ relay URL`. Mutual authentication and forward
  secrecy per channel; the relay sees ciphertext. The transport messages carry the unchanged
  `curvyo-connector-core` JSON frames (`Hello`, `Welcome`, `SubmitJob`, `JobChunk`, `Status`,
  `Stop`, ...), each split into Noise messages of at most 64 KiB (Noise's limit) with a
  continuation bit (Proposal), so core's 1 MiB frame and 256 KiB chunks stay as they are. `Pair`
  and `Auth` are not sent on this path: the handshake is the authentication.
- *Authority.* The connector replays the machine room's keyring itself (`crypto-core`) and accepts
  a channel only from a current member; the relay's admission check is defence in depth (ADR 0010
  §10). The connector never writes a keyring entry.
- Cost: a new kind of room and three relay frames; a hand-written Noise `KK` on the primitives
  `crypto-core` already has (`curve25519-dalek`, `chacha20poly1305`, `sha2`, `hkdf`), tested
  against the published Noise vectors, like CPace (ADR 0040); the connector becomes the first
  native Rust client of the relay.

**B. The relay as a byte pipe carrying today's TLS end to end.** The connector registers an id
with the relay; the app runs the ADR 0035 TLS client over the pipe; pinning, SPAKE2 pairing and
the token are reused unchanged. The relay sees TLS records only. Two variants for the browser:
(B1) TLS inside wasm over a WebSocket (rustls with a wasm-capable crypto provider; `ring` on
`wasm32-unknown-unknown` is unverified, a spike would decide), or (B2) the relay as an SNI
passthrough on `<id>.relay.example` with a publicly trusted certificate on the connector (ACME
through the relay operator's DNS). Rejected: B1 adds a second TLS stack to the browser for one
feature; B2 needs wildcard DNS and a public CA for every self-hoster, and the operator who
controls DNS can issue a certificate for any connector (detectable only in CT logs), so it is not
end to end against the operator. In both, identity stays per-app-profile tokens: sharing needs a
code typed at each person's app while the connector prints it, revocation needs a shell on the
connector's host, nothing relates to workspace membership, and the relay must route an id it
cannot authenticate (anyone on the internet can knock and burn pairing attempts).

**C. The relay terminates TLS and re-originates it.** The relay holds the connector's trust and
the clients' tokens and forwards plaintext commands. Rejected: the relay reads every job and can
start every laser it serves; it contradicts ADR 0008 §1 and WS-META, and a breach of one instance
becomes physical risk in every workshop on it.

**D. The connector as an ordinary member of the device group.** Rejected: group members get the
devices list and the library stream (ADR 0062), and `reconcile` adds every group member to every
synced workspace. A connector on a Raspberry Pi would hold the keys to all the maker's work.
Least privilege puts it in a room of its own.

## Decision (option A, accepted 2026-11-17)

1. **Machine room.** An ADR 0031 room per connector, keyring per ADR 0010 unchanged (no new
   payload, roles admin and member, ADR 0010 §12). Admins: the owner's devices (the device group
   of the device that enrolled it); members: the connector and the persons it is shared with
   (decision 2a). One sealed stream, the machine record (`format_version = 1`, golden file:
   name, the connector's identity, the assigned workspace ids). The devices list (ADR 0040
   decision 1) gains `machines.<room id hex> = {label}` (readers ignore unknown fields, no version
   change), and `reconcile` treats each listed machine room like a switched-on workspace: every
   group device becomes admin there, a removed or leaving device is removed with `NewEpoch`. A
   non-group member (the connector, a shared person) is never removed by the device rule; the
   workspace rule of decision 2a removes shared persons.
2. **Who may command (Frage 221 C).** The connector tells two kinds of channel peer apart by the
   role its own keyring replay gives (no new field, no new payload): **admin = one of the
   owner's own devices**, **member = anyone else** (a shared person; the connector itself never
   opens a channel). The two are one because only the owner's `reconcile` writes `admin` in a
   machine room (decision 1) and the app offers no "make admin" there (Proposal; ADR 0041's
   last-admin rule and leave still apply). Every member may open a channel, read `Status`, send
   a job (`SubmitJob`), `Pause` and `Stop`. A **start** is `Start`, `Resume` and every later
   command that moves the head or fires the laser (`Frame`, and `0118`'s Home, jog and focus):
   - from an admin it acts as on the LAN: it runs at once, or waits for the local press only when
     the connector runs with `--require-local-start` (CONN-SAFETY A, Q17);
   - from a member it always goes to `AwaitingLocalStart` and runs only after the press at the
     connector, whatever the flag. `Resume` counts as a start because it restarts motion; a job
     paused and resumed by a member waits at its line (Proposal).

   `Stop` is never held. The rule lives in the connector: the `-io` session passes "local start
   required = flag or peer is not admin" with each start event to core's `Machine`, instead of
   one machine-wide option; the wire messages, `Status` and protocol 0 are unchanged. Clients of
   the LAN mode (ADR 0035 token, paired with a code read at the connector's own terminal) keep the
   flag-only rule: Frage 221 is about the relay path (Proposal). Admins add and remove members.
   Revocation: `RemoveMember` + `NewEpoch`; the relay closes that identity's connections (ADR
   0031 6a) and the connector closes its open channels on replaying the entry. A job already
   running keeps running; any member can stop it.

   **2a. Sharing through a workspace (Frage 219 B).** The workspace stays the only unit of sharing
   (ADR 0044). An admin assigns a machine to a workspace it is a member of: the machine record
   lists the workspace id, and the workspace's list stream (ADR 0037) carries
   `machines.<room id hex> = {label}` so its members find the room (readers ignore unknown
   fields; the exact shape and its format check are the sharing slice's, Proposal). `reconcile` on every
   admin device extends to machine rooms: each member of an assigned workspace that this device
   can read and that is not in the machine room gets `AddMember{role: member}` (never admin,
   whatever its role in the workspace); each non-group member that is in no assigned workspace
   any more (removed, left, or the workspace unassigned) is removed with `NewEpoch`. A workspace
   admin who is not the owner cannot change the machine room; the removal reaches it through the
   owner's devices. **Gap:** a removal from the workspace reaches the machine room when one of
   the owner's devices next runs `reconcile` (on start and on every keyring change of an assigned
   workspace); until then the removed person can still send, stop and request a start, and a
   start still needs the press at the machine. Recorded in `docs/technical-debt.md` with the
   freshness gap (Consequences) when the sharing slice is built.
3. **Lease.** The connector is the lock: one job at a time, `Busy` otherwise (ADR 0035 §7). The
   relay holds no lease, because it cannot read a command; this amends ADR 0004 §4 for machine
   resources.
4. **What the relay stores and sees.** Stored: the machine room's keyring (every member's public
   keys, the connector's among them) and the sealed machine record. In memory: which admitted
   connection is which identity, open channel numbers. Never stored or logged: channel bytes. Seen
   as metadata: that a room has one always-connected member and channel traffic (a machine), the
   connector's IP address (the workshop), when it is online (working hours), job sizes and timing
   (a burst of several MiB), which identities command it and when. The confirm at "Add machine"
   says so in one sentence (Proposal).
5. **Offline and reconnect.** An offline connector: `ChannelOpen` is refused `offline`; the app
   shows the machine as offline. No job queue on the relay (Proposal: a laser job is sent to a
   machine that is there). The connector reconnects with backoff 1 s to 60 s (Proposal). A job in
   flight survives every disconnect (connector, client or relay); a client that comes back opens a
   new channel, gets `Status` and can `Stop` (criterion 6). If the relay or the internet is down,
   remote stop is impossible: the LAN path and the machine's own emergency stop remain, which is
   one reason the LAN mode stays (decision 8).
6. **Browser reach.** The browser is a group device (ADR 0040), so it is admin of the machine room
   and opens the channel from its wasm session, exactly like the desktop. This decides Frage 121 A:
   no self-signed warning, no served web app needed for remote use. Frage 121 B (the web app
   served by the connector on the LAN) stays decided but loses its urgency.
7. **Accounts, cost, telemetry, self-hosting.** No account: identities are device keys, no email,
   no password (ADR 0004 §10, ADR 0007 kept). Cost on the hosted relay: one idle WebSocket per
   connector (pings), job bytes up to 64 MiB per job relayed live; quotas join the debt "Relay
   quotas". No telemetry; the connector still logs jobs only locally. Self-hosting: the same
   `curvyo-sync-server`, or the one built into the connector (decision 8b); the connector takes
   `--relay <url>`; a relay on the workshop LAN gives the same channel without internet.
8. **LAN (Frage 220 A, with the customer's addition "a stand-alone connector would then also
   include a relay server").**
   - (a) **The LAN mode of ADR 0035 stays as built; PR #375 needs no change.** Pinned TLS, code
     and token: the path for a connector that is not enrolled and the fallback when no relay is
     reachable.
   - (b) **Built-in relay (Proposal for the shape).** `curvyo-connector-server --relay <url>
     --host-relay` also runs the ADR 0031 relay in its own process: the library of
     `curvyo-sync-server` with its rules, store and limits unchanged, data in
     `<state dir>/relay/`, listening on the port of `<url>`. The connector then links to `<url>`
     exactly as to any relay; nothing in the enrolment, the channel or the app is special. A maker
     without a hosted or self-hosted service types that address as the app's server address (a
     `ws://host:port` LAN address, as for a self-hosted relay without a proxy, ADR 0031 decision
     11), and "Add my computer", workspaces, sharing and "Add machine" all work on the workshop
     network with the same flows and no internet: one enrolment model for the LAN and for remote
     use. The connector dials `<url>` itself, never `127.0.0.1`, because the server address text
     is bound into CPace (ADR 0040 decision 4) and into the channel prologue, so every party must
     use the same text.
   - (c) **One relay per maker (Proposal).** A machine room lives on the relay of the enrolling
     device's device group (the app's one server address); the machine is enrolled once and
     reached through that relay, whether it is hosted, self-hosted or built into the connector.
     Moving a machine to another relay is a new enrolment. A maker on a hosted relay whose
     internet is down uses (a). A direct channel to an enrolled connector on its LAN listener
     (the same Noise `KK`, no relay hop, pin taken from the machine record) would serve both paths
     with one enrolment at once; it is not planned until the customer asks for internet-free use
     with a hosted relay.
   - Rejected for (b): a one-room relay in `curvyo-connector-io` on `sync-core`'s sans-I/O relay
     rules (a second relay shell with its own store, and it holds no device group or workspace,
     so a second computer of the maker could not join without another relay); asking the maker to
     run `curvyo-sync-server` beside the connector (two programs, against the customer's words);
     serving the relay on the connector's TLS port 7447 (the relay connection lives in the
     frontend, ADR 0031 decision 2, and a webview WebSocket cannot pin a self-signed key).
9. **Crates (no new crate, Proposal).** `curvyo-connector-core`: the start events take "local
   start required" from the caller (decision 2), no wire change. `curvyo-crypto-core` +
   `channel` (Noise `KK`, vectors). `curvyo-sync-core` + channel frames and a pure
   `relay::Channels` (relay protocol 4; 1 to 3 still admitted; server before apps), enrolment
   messages under the new label. `curvyo-sync-server` + routing. `curvyo-connector-io` + the relay
   link (outbound wss, the room member session, channel mux), its `session` loop made to run over
   a channel as well as a socket; the connector binary gains edges to `crypto-core` and
   `sync-core` (both pure). `curvyo-connector-server` + `--relay`, `--enrol`, the identity and
   room files in its state directory, `--host-relay` (decision 8b) and an edge to the library of
   `curvyo-sync-server`. That library gains one blocking entry point for embedding (its own
   `tokio` runtime on one thread; local address; shutdown), so the connector binary declares no
   `tokio` itself; `tokio` and `axum` are already in `Cargo.lock` (no new lock entry; the binary
   grows by them, size measured in the story). `curvyo-sync-server` takes no edge back.
   `editor-wasm` + channel glue for desktop and browser (the
   relay connection is in the frontend, ADR 0031 decision 2). One new dependency question: the
   connector's root certificates for the relay's `wss://` (`rustls-native-certs` or
   `webpki-roots`; the latter's CDLA-Permissive-2.0 is not in `deny.toml`), decided in the story.

## Consequences

- One enrolment gives remote, browser and shared use; revocation is an app action, with
  rotation, not a shell command on the connector's host.
- The relay protocol grows a forward-only channel. The relay still never sees a job, but it does
  learn when and how much a workshop works; stated in the privacy text.
- A hand-written Noise `KK` is security-relevant code: Noise test vectors, the focused crypto
  review ADR 0008 asks for, and it reverses the customer's "no Noise" of Frage 120 for this path
  only (the browser objection to Noise does not apply, because wasm speaks it; Frage 218).
- ADR 0010 §10's freshness gap matters more here: a relay that withholds a `RemoveMember` from the
  connector keeps a removed member able to command until the connector sees it. Mitigation: the
  relay is the maker's choice, and an admin's app can check the connector's reported keyring head
  in `Welcome` (Proposal for the story).
- Remote start (Frage 221 C): the owner's devices start from anywhere; everyone else needs the
  press at the machine, so a shared machine never fires from outside the workshop on a stranger's
  command. "Own device" is the admin role, which ties the safety rule to `reconcile` and to the
  app never writing `admin` in a machine room for anyone but the owner's devices.
- Sharing (Frage 219 B) adds no second unit of sharing, but a removal from a workspace reaches the
  machine only when an owner's device runs `reconcile` (decision 2a); the start rule bounds what
  that gap allows.
- Built-in relay (decision 8b): one binary serves a workshop without any hosted service. Its
  outer connection is plain `ws://` on the workshop LAN, like every self-hosted relay without a
  proxy: jobs travel inside the Noise channel and records are sealed, so the LAN sees the
  metadata of decision 4, not the content. The browser build needs `wss://` and a proxy in front
  (ADR 0031). The connector binary carries `tokio` and `axum`.
- Note 2026-11-18: [ADR 0071](0071-connector-api-ctl-and-maintenance-mode.md) (Accepted 2026-11-18)
  amends what starts an enrolment (an owner's `curvyoctl enrol` instead of a code at first start),
  what "the press at the machine" is (an owner's confirmation, so presence at the laser is no
  longer implied), the LAN start rule and decision 9's flags, and inserts a slice "connector API,
  `curvyoctl` and maintenance mode" before the relay slice (relay becomes slice 6, sharing 7).
  The decision text above is unchanged.

## Concepts this touches

- ADR 0004 §4 (lease held by the connector, not the relay), §5 (relay: one more forward-only
  route), §13 (self-hosting unchanged).
- ADR 0008 §1, §10 (no plaintext; new metadata listed in decision 4).
- ADR 0010 (keyring reused unchanged in a new room kind; §10 freshness).
- ADR 0031 (relay protocol 4, three frames; admission unchanged; decision 11, the relay library
  embedded in the connector binary, decision 8b).
- ADR 0035 §3, §4 (pin, code, token stay LAN-only), §8 (browser anywhere decided here), option D
  (its objection does not apply to a machine room).
- ADR 0040 (pairing slots and CPace reused with a new label; devices list `machines`; `reconcile`
  extended; My devices lists machines; remove and leave reach machine rooms).
- ADR 0041 (roles, last-admin rule and leave apply to machine rooms).
- ADR 0044 (workspace as the only unit of sharing, kept by Frage 219 B); ADR 0037 (the
  workspace list stream carries the assigned machines, decision 2a).
- ADR 0038, ADR 0062, spec `0116` (the profile's `connection` names the machine room and relay,
  a profile record format bump, once `0116` is built; until then the link from a profile to its
  machine room is kept per computer beside the ADR 0035 credential record, Proposal; the room keys
  never travel in the library stream).
- ADR 0007 (credential store holds the machine room keys like other room keys; no accounts).
- Specs `0064` criteria 2, 3, 4, 7, 8, 9; `0095`, `0096`, `0097`, `0116`, `0118` (its Home, jog,
  frame and focus are starts under decision 2); `docs/technical-debt.md`
  ("Relay quotas", a new entry for the freshness gap).

## Customer answers (2026-11-17)

- Frage 218 CONN-RELAY: **A**, the connector as an enrolled member of its own machine room with an
  end-to-end channel through the relay.
- Frage 219 CONN-SHARE: **B**, every member of a workspace the machine is assigned to (decision
  2a).
- Frage 220 CONN-LAN: **A**, with the customer's addition (translated): "a stand-alone connector
  would then also include a relay server" (decision 8b; read by the architect as: the connector
  binary can host the full relay, so a maker without a hosted service gets the same enrolment and
  room model on the LAN).
- Frage 221 CONN-REMOTE-START: **C**, a local start is required for everyone but the maker's own
  devices (decision 2).

## Questions for the customer (as asked)

Frage 218 (CONN-RELAY): How does a connector become reachable through the relay?
- A: As an enrolled member of its own machine room, like adding a device: code and check words
  once, then an end-to-end channel through the relay (Noise, not TLS, on this path); the relay
  sees ciphertext and when and how much the machine works.
- B: The relay as a pipe for today's TLS connection; pairing per app with the code at the
  connector, revocation on the connector's host; the browser needs a second TLS stack or public
  certificates the relay operator controls.
- C: The relay decrypts and forwards. It can read every job and start every laser.
- Recommendation: A. Default if unanswered: A is not built; the LAN mode ships alone.

Frage 219 (CONN-SHARE): Who besides your own devices may use your machine?
- A: Persons you add to that machine one by one (each machine is its own thing you share).
- B: Every member of a workspace you attach the machine to; removing someone from the workspace
  also removes them from the machine (the workspace stays the only thing you share).
- C: Only your own devices for now; sharing later.
- Recommendation: B. Default if unanswered: C.

Frage 220 (CONN-LAN): What becomes of the direct LAN mode?
- A: It stays as it is built (PR #375); a later slice reaches an enrolled connector on the LAN
  with the same enrolment, no second pairing.
- B: It stays, with its own pairing, forever separate from the relay enrolment.
- C: It is removed; offline use means a relay on your own network.
- Recommendation: A. Default if unanswered: A.

Frage 221 (CONN-REMOTE-START): May a job be started from outside the workshop?
- A: Yes, as on the LAN; "require a local start" stays an option per machine, off by default.
- B: Through the relay, a local start at the machine is always required.
- C: Through the relay, a local start is required for everyone but your own devices.
- Recommendation: C. Default if unanswered: B.
