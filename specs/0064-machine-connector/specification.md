# Machine connector: a small program next to the machine that the app talks to, on the local network or through the sync service

Status: Draft (a sketch for the customer; not Ready: protocol, security and the first target need the architect's ADR and the customer)
Priority: Should
Origin: Customer (request of 2026-10-17, translated): "Machines are attached by a connector, which can be a very simple Rust service that compiles and runs from itself. It can then talk to a local instance in the network or via the sync service."

## User value

As a maker whose laser stands in a workshop and whose computer, or browser, is somewhere else, I want to run a small program on the machine that is plugged into the laser, and send a job to it from Curvyo and watch it run, so that I need neither a cable to my laptop nor a desktop-only app (`R-MFG-LASER-002`, `R-MFG-LASER-003`).

**What other tools do.** LightBurn: talks to the controller directly over USB serial or a network address from the same PC; "LightBurn Bridge" is a separate small program for a machine PC. OctoPrint (3D printers): a service next to the printer with a web interface and API; the slicer uploads to it over the network. Mainsail and FluidNC: the controller has its own web server. Carbide Motion and Easel: a local service in the browser's reach. Pattern: a small service near the machine, spoken to over HTTP or WebSocket; that is what the customer describes.

**Word clash.** ADR 0007 already has an "asset connector" (a client of a third-party asset library). This is the **machine connector**; the two are different things and use different words in code and UI.

## What it is (Proposal)

A single self-contained binary, written in Rust, that the maker downloads or builds (`cargo install`), starts on a computer next to the machine and leaves running. It holds the link to the controller, accepts jobs, runs them, reports status. The Curvyo app is a client of it. There are two ways to reach it:

- **Direct (local network):** the app connects to the connector's address on the LAN.
- **Through the sync service** (`curvyo-sync-server`, ADR 0004): the connector keeps an outbound connection to the sync service and the app reaches it from anywhere through the relay, end-to-end encrypted so the relay cannot read a job (ADR 0008).

## Acceptance criteria (as far as clear)

1. Given the connector program, then it runs without the desktop app, without a display and without an account, on Linux first (a Raspberry Pi or a workshop PC), with a command line and a configuration file; it needs no installer.
2. Given its first start, then it prints a **pairing code** and its address. The app pairs once by typing the code; afterwards both sides hold a shared key and the code is spent. Nothing is sent to a connector that is not paired.
3. Given pairing, then the connection is encrypted on the LAN (no plain HTTP with a token) and the app remembers the connector by its key, so a changed key is shown as a changed key, not silently accepted.
4. Given the app, then the machine profile (`0026`) names the connector (direct address or relayed id) and the controller port; a connection test is a symbol button in the profile with a visible result (connected, controller answered, version), no popup.
5. Given the first connector (decided: GRBL over serial), then it speaks **GRBL over serial** to one controller: it reports status and position, accepts a G-code job, streams it with the controller's flow control, pauses, resumes and stops it, and can run the **frame** (outline at low power, `R-MFG-LASER-003`).
6. Given a job in progress, then the app shows progress and status from the connector, and a lost connection never leaves the job in an unknown state: the connector keeps running the job and the app reconnects and reads the state; stop works even from a fresh connection.
7. Given the browser build (`R-SYS-005`), then it reaches a connector the same way (network), because it has no serial access of its own; export of the G-code file stays always available.
8. Given the sync service as the path, then the connector's outbound link is the only open connection; no inbound port needs to be opened on the workshop network.
9. Given safety, then a machine can be set to **require a local start**: the job is accepted and shown at the connector (a button or a key on the connector's host), and runs only after that (Question 3). The default is off; it is the maker's setting (permit over refuse).
10. Given logs, then the connector keeps a local log of jobs (time, name, result) and sends nothing else anywhere. No telemetry (`R-SYS-007`).
11. Given the code, then the message types (job, status, commands) are plain data in a `*-core` crate that builds for wasm32 (so the browser app and the connector share them); the connection code (serial, sockets, TLS) is in an `-io` crate; the connector binary is thin.

## Out of scope

- Other controllers (Ruida, Trocen, Marlin, Smoothieware, plotters, embroidery machines): each becomes its own entry, one family at a time (`CLAUDE.md` §6). A connector for them may be an extension (`0065`).
- USB without serial emulation, camera feeds, firmware update, machine-side jog panel beyond frame, job queues for many users.
- An accounts system or a hosted connector service; the sync service already has the accounts it has.
- Discovery by multicast (Proposal for later: the pairing address is typed or scanned).

## Architect notes (2026-10-17)

- New crates need an ADR (`CLAUDE.md` §5): a protocol core crate, an `-io` crate for GRBL/serial and the transport, and a `-server`-style binary. Crate naming needs the exception of `CLAUDE.md` §8 extended (the connector is a binary like `curvyo-sync-server`). **`needs-customer`** (data leaves the machine over the relay; security model).
- Decide: HTTP/WebSocket or a framed binary protocol (WebSocket is reachable from the browser; recommended), how a job travels (G-code text as data, so the connector needs no CAM code), and how the relay reuses the document keyring of ADR 0010 for pairing.
- The connector holds no document and no user data beyond the job in flight.

## Decided (customer, 2026-10-17)

- **CONN-FIRST: A.** The first connector talks GRBL over serial (USB), reached directly on the LAN first (the CONN-PATH default).

## Open questions (customer; each has a default)

Frage 1 (CONN-PATH): Which route comes first, direct on the LAN or through the sync service?
- A (default, recommended): direct on the LAN first; it needs no server and is testable at once. The relay route follows when the sync service exists.
- B: relay first, because the browser and remote use are the main reason.
- Default if unanswered: A.

Frage 2 (CONN-SAFETY): Is a local start confirmation (criterion 9) built in the first version?
- A (default): yes, as an option, off by default.
- B: always required for a remote start.
- C: not built yet.
- Default if unanswered: A.

## Links

Requirements: R-MFG-LASER-002, R-MFG-LASER-003 (existing); R-MFG-011 (new)
Depends on: `specs/0026` machine-profile (reserved), `specs/0029` laser job output (reserved, the G-code), ADR 0004 (sync service), ADR 0008 (end-to-end encryption), ADR 0010 (keyring)
Related: `specs/0063-manufacturing-workbenches/`, `specs/0065-extensions-sdk/`, ADR 0007 (the other "connector")
ADRs: a connector ADR (protocol, pairing, crates), `needs-customer`
PR: -
