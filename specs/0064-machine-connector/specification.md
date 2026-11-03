# Machine connector: a small program next to the machine that the app talks to, on the local network first, through the sync service later

Status: Draft (Ready-candidate: becomes Ready when the architect's ADR and the UX notes exist and the customer has seen the open questions below; every question has a default)
Priority: Should
Origin: Customer (request of 2026-10-17, translated): "Machines are attached by a connector, which can be a very simple Rust service that compiles and runs from itself. It can then talk to a local instance in the network or via the sync service." Start of work: customer, 2026-11-02. Slice contents below are Proposal unless marked Decided.

## User value

As a maker whose laser stands in a workshop and whose computer, or browser, is somewhere else, I want to run a small program on the PC or Raspberry Pi that is plugged into the laser, pair it once from Curvyo on the same network, and from the app see the machine, move it, frame a job, send the job and watch it run, so that I need neither a cable to my laptop nor a desktop-only app (`R-MFG-LASER-002`, `R-MFG-LASER-003`).

**What other tools do.** LightBurn: talks to the controller over USB serial or a network address from the same PC; "LightBurn Bridge" is a separate small program for a machine PC; no pairing, no encryption, anyone on the network who knows the address can send. OctoPrint: a service next to the printer with a web interface and an API key; plain HTTP by default. Mainsail and FluidNC: the controller has its own web server, no authentication by default. Carbide Motion and Easel: a local service in the browser's reach. Pattern: a small service near the machine, spoken to over HTTP or WebSocket.

**What we do better.** (1) Nothing on the network can read a status or send a byte to the machine before a one-time pairing; the key is remembered and a changed key is shown. (2) Stop is reachable on every screen and works from a fresh connection, in the middle of a job, whatever the buffer holds. (3) A GRBL simulator ships with the code, so the whole path is testable and demonstrable with no laser at all. (4) The same path serves the desktop app and the browser, because neither opens a serial port.

**Word clash.** ADR 0007 has an "asset connector" (a client of a third-party asset library). This is the **machine connector**; code and UI use different words for the two.

## Where the G-code comes from today

Nothing in the repository produces G-code yet: there is no machine profile (`0026`), no manufacturing roles (`0027`), no laser job output (`0029`), and no workbench shell (`0063`, stage 1). The connector carries G-code as opaque text, so it needs no CAM code. For **slice 1** the app therefore needs a source of G-code that exists today; this is **Question 3 (CONN-GCODE)**. The default is a G-code file the maker picks (from LightBurn, Inkscape's J Tech plugin or any other tool). Producing G-code from the drawing is the next slice and belongs to `0029`.

Slice 1 also needs a **minimal machine record** to satisfy `R-SYS-008` (no job without a machine profile). It is a stand-in that `0026` replaces; it holds only: name, work area width and height in mm, the paired connector, the frame power. It lives in the app's local settings. No migration is promised when `0026` arrives (Proposal).

## Slices (Proposal; one PR each, each tried end to end)

| Slice | Delivers | Needs |
|---|---|---|
| **1 (this spec)** | Connector binary, protocol, pairing, GRBL over serial, GRBL simulator, status, home, jog, frame, send a G-code file, pause, resume, stop, progress, reconnect; direct on the LAN; desktop app and browser | answers to Questions 3 to 5, the connector ADR |
| 2 | G-code from the drawing, with the preview and the gate of `0029` | `0026` to `0029`, workbench shell |
| 3 | Relay path through `curvyo-sync-server`, end-to-end encrypted (`R-SYS-007`) | the sync service, ADR 0008 and 0010 |
| 4 | Further controllers, one family at a time (`0065`) | the second-family ADR |

## Meaning of the operations (one line each, for the customer to veto)

- **Pair:** the app and the connector prove to each other that the same person typed the same one-time code, and from then on trust each other's key; nothing else can talk to the machine.
- **Home:** the machine drives to its limit switches to learn where it is (GRBL `$H`); nothing else moves until it has.
- **Jog:** move the head by a chosen distance in X or Y at a chosen speed, only while no job runs.
- **Frame:** the head traces the rectangle around the job without cutting, with the laser off or at the low power set in the machine record, so the maker sees where the job will land.
- **Pause / Resume:** feed hold; the laser goes off and the head stops; resume continues exactly where it stopped.
- **Stop:** halt, reset the controller, laser off, job ended and marked "stopped"; the job cannot be resumed.

## Acceptance criteria

### Connector program

1. Given the connector binary, when it is started on Linux (x86_64 and aarch64, so a Raspberry Pi 4) with no display, no desktop app and no account, then it runs from the command line with a configuration file (serial device, baud rate defaulting to 115200, listen address and port) and needs no installer. Verified by starting it in a container without a display.
2. Given the flag `--simulate`, when the connector starts, then it uses the built-in GRBL simulator instead of a serial port and behaves as a connected, unhomed machine, so the whole feature can be tried and tested without hardware (Proposal).
3. Given the connector keeps a local job log, when a job ends (done, stopped, failed), then one line with time, job name, line count and result is appended to a local file; the connector sends no telemetry and contacts no other host (`R-SYS-007`). Verified by running it with network access limited to the test client and reading the log.

### Pairing and security on the LAN

4. Given a connector with no paired device, when it starts, then it prints its address and a **pairing code** of at least 8 digits, valid for 5 minutes and for one successful pairing; given a connector with at least one paired device, then it prints no code unless started with `pair` (or the command `curvyo-connector pair` is run), so a running workshop machine does not offer a code to the network.
5. Given an unpaired client, when it connects, then the only thing the connector answers is the pairing handshake; no status, position, version or error text reaches it, and no command is executed. Verified by a client that skips pairing and tries every message type.
6. Given the pairing code, when a client enters it wrong 5 times or after 5 minutes, then the code is spent and the connector prints a new one only on a new `pair` command. The code is not sent over the network in a form from which it could be recovered offline by a passive listener (the architect picks the method, for example a password-authenticated key exchange; a test records a handshake and shows the code is not recoverable from it by a dictionary run of all 10^8 codes in the test).
7. Given a successful pairing, then both sides store the other's public key, the code is spent, and the connector lists paired devices by name and key fingerprint (`curvyo-connector devices`); `curvyo-connector unpair <name>` removes one and that device is refused from then on, including connections already open.
8. Given a paired connection, then everything on it (commands, G-code, status) is encrypted and authenticated; a test captures the bytes of a job send and finds no part of the G-code in them, and a test that flips one byte in a message sees the message refused and the connection closed. A replayed message is refused.
9. Given the app has paired a connector, when the connector later presents a different key under the same address, then the app shows "the connector's key changed" with both fingerprints and does not connect until the maker re-pairs; it never accepts a changed key silently.
10. Given the secret files of the connector (its key, the paired keys), then they are readable only by the user that runs it (mode 0600) and are never written to the job log.

### Machine state, home, jog

11. Given a paired connection to a connected controller, then the app shows connection (connector reachable, controller answered, GRBL version), machine state (Idle, Run, Hold, Jog, Alarm, Home, Door, Check as GRBL reports them), and the work position in mm, and the position updates within 1 second of a change (simulator: 4 status polls per second, observed at the app).
12. Given a machine in Alarm state, then the app shows the alarm code and its plain-language text, offers Home and Unlock as separate explicit symbol buttons, and sends no job and no jog until the state is Idle.
13. Given the Home button, when pressed on an idle machine, then the connector sends `$H`, the app shows state Home and then Idle, and position zero at the machine's origin.
14. Given a jog control with step sizes 0.1, 1, 10 and 100 mm and a speed set in the machine record, when a jog is pressed and the machine is idle, then exactly one `$J=` command with that distance and speed is sent; when a job is running or paused, then jog is refused with a visible reason; a jog that would leave the work area of the machine record is refused with a visible reason before anything is sent.
15. Given a jog in motion, when the maker presses stop or releases a held jog key, then the jog-cancel realtime byte (0x85) is sent and the head stops within the controller's deceleration, not at the end of the distance.

### Frame

16. Given a G-code file loaded in the app, when the maker presses Frame, then the app computes the axis-aligned bounding box of the file's motion (G0, G1, G2, G3 including arcs, absolute and relative mode, millimetres or inches) with an error of at most 0.01 mm, sends a frame move as a separate small job along that box, and the head traces it once with the laser at the frame power from the machine record. The default frame power is 0 (laser off), for machines with a pointer; the maker may set 1 to 100 percent (Question 5 may change the default).
17. Given a frame in progress, when the maker presses Stop, then it ends at once and the laser is off; given a frame, then it never runs while a job runs, and the machine returns to the start point of the box when done.

### Send, run, pause, stop

18. Given a loaded G-code file and a connected, idle, homed machine, then the **send panel** states, before the Send button is enabled, the machine record name, the file name, line count, bounding box and work area, and the material (shown as "none selected" until the material library exists, `R-SYS-008`).
19. Given no machine record is selected, or the machine record has no paired connector, then Send is disabled with the reason shown in text, and the connector also refuses a job that does not name a machine record, so a second client cannot bypass the gate (`R-SYS-008`).
20. Given the file's bounding box lies outside the work area of the machine record, then Send is disabled with the overshoot in mm per side shown; Frame still works.
21. Given an accepted job, then the connector streams it with the controller's flow control: it never has more than 127 bytes of unacknowledged G-code in the controller's receive buffer (character-counting streaming) and sends the next line when `ok` frees room. Verified on the simulator, which reports an error and fails the test if a 128th byte arrives unacknowledged.
22. Given a running job, then the app shows the state, the line count sent and acknowledged out of the total, elapsed time, and the position, updating at least once per second. Progress is by acknowledged lines (it is not a time estimate; the estimate comes with `0029`).
23. Given a running job, when Pause is pressed, then the feed-hold byte (`!`) goes out as a realtime byte at once, the state shows Hold, the laser is off; when Resume is pressed, then `~` is sent and the job continues with the next line, with no line lost or repeated (simulator: the sequence of executed lines equals the file).
24. Given a running or paused job or a frame or a jog, when Stop is pressed, then the connector sends the feed hold, waits for the hold to complete or for 2 s, then the soft reset (0x18), discards every unsent line, marks the job "stopped", and reports Idle or Alarm as the controller does; after the reset, no further job line reaches the controller. Verified on the simulator with the buffer full: the reset byte is written before any line that was queued behind it, and no line follows it.
25. Given the Stop button in the app, then it is visible and enabled on every screen of the app where a machine is connected, whatever the dialog or panel state, reachable by one tap or click and by one keyboard shortcut (the key is chosen in the review of `R-SYS-009`), never behind a confirmation. A failed stop (connector unreachable) is shown as a failure in red, never silently.
26. Given a stop request from any paired client, even one that connected a moment ago and sent nothing else, then the connector executes it; a stop never waits for a lock held by a job, a send, or another client.
27. Given an `error:N` reply to a line, then the connector halts the job (feed hold, reset), reports the line number, the line text and the GRBL error text, and the job is marked "failed"; given an `ALARM:N` the same, and the app shows the alarm.
28. Given one controller, then only one job runs at a time; a second Send from another client is refused with "machine busy" and the running job is unaffected.

### Local start confirmation

29. Given the machine option **require local start** (off by default, Question 2), when it is on and a job is sent, then the connector accepts it, reports the state "waiting for start at the machine", and does not move the machine until someone confirms at the connector (a key press in its terminal, or `curvyo-connector start`, on the host). The job times out after 10 minutes unconfirmed and is discarded, logged as "not started". Verified on the simulator: no motion is observed before the confirmation. Stop, Frame and Jog are not affected by this option's wait.

### Link loss and restart

30. Given a running job, when the app's connection drops (network lost, laptop closed, browser tab closed), then the connector keeps running the job (setting `on_link_lost`: `continue`, the default, or `hold`, Question 4), the job log has the result, and when the app reconnects within the same session it reads the running state, line count and position and shows them; stop works from the new connection at once.
31. Given the setting `on_link_lost = hold`, when the last client drops, then the connector sends a feed hold and shows "held, link lost"; the job resumes only when a client reconnects and presses Resume.
32. Given the controller is unplugged or the serial port fails during a job, then the connector marks the job "failed: controller lost", does not retry the job, shows the state "controller lost" to every client, and reconnects to the port when it reappears; the job is never resumed by itself, and the app tells the maker the machine position is unknown and to home again.
33. Given the connector process is restarted (or the host loses power), then no job resumes by itself; on start the machine is treated as unhomed in the app, and the log shows the last job as "interrupted".

### Shared code and tests

34. Given the code, then the message types (pairing, commands, state, job, errors), the G-code reader (bounding box, line count), the GRBL line parser and the streaming state machine are plain data and pure functions in a `*-core` crate that builds for `wasm32-unknown-unknown`, with no serial, socket, clock or thread; the serial and network code is in an `-io` crate; the connector binary is thin (`CLAUDE.md` §5 and §6). The crate split and the names are the architect's ADR.
35. Given the GRBL simulator, then it is pure (virtual time advanced by explicit ticks, no clock), speaks the same byte stream as the serial port, and implements: `?` status with the state and positions, `$H`, `$J=`, `$X`, `$$` (including work area travel), line replies `ok` and `error:N`, `ALARM:N`, the realtime bytes `!`, `~`, 0x18, 0x85, a 127-byte receive buffer, a planner that executes lines over simulated time, and fault injection (error at line N, alarm at line N, unplug, slow acknowledgements). Every criterion above that names the simulator runs in CI without hardware.
36. Given golden files in `tests/fixtures/`, then they hold the recorded GRBL reply lines for each state and error code and a G-code sample with every supported motion command, so the parser is tested against real controller text (`CLAUDE.md` §5).
37. Given a real GRBL 1.1 controller (customer hardware, Question 5), then the demo check is: pair, home, jog 10 mm, frame, send a 20-line file with the laser disabled in the file, pause, resume, stop. The result is reported in the PR; CI does not need the device.

### Browser and desktop

38. Given the desktop app and the browser build (`R-SYS-005`), then both use the same network client to the connector; neither opens a serial port. Export of the G-code file stays available whatever the connector's state. Which page origins a browser can reach is Question 6 (CONN-BROWSER).

## Out of scope

- The relay path through the sync service (slice 3); no inbound port and relay encryption are specified there, not here.
- Producing G-code from the drawing, a job preview and time estimate (`0029`), roles, materials, nesting.
- Other controllers (Ruida, Trocen, Marlin, Smoothieware, plotters, embroidery): one family at a time (`CLAUDE.md` §6); extensions (`0065`).
- USB without serial emulation, camera feeds, firmware update, a machine-side jog panel, job queues for many users, several machines on one connector.
- Discovery by multicast or mDNS (the address is typed or copied; Proposal for later).
- Resuming a job after a connector restart or a power cut; overrides (feed/power sliders) while running; work-coordinate offsets (G54 to G59) management.
- Accounts, a hosted connector service, an installer, a system service file (a documented example unit is fine, not shipped as a feature).
- Reading or editing GRBL's `$` settings beyond the work-area travel read.

## UX notes

(filled in by ux-engineer before Ready). Surfaces the notes must cover: the machine record form with the connector pairing steps and the changed-key warning; the machine panel (state, position, home, jog, frame) reachable in the desktop app and the browser; the send panel (criterion 18); progress; the always-visible Stop (criterion 25) and its place when the workbench shell exists; the connector's terminal output (pairing code, local start prompt). No popups for results; symbol buttons with tooltips and accessible names (`docs/design-system.md`).

## Architect notes (2026-10-17, extended 2026-11-02 as Proposal)

- New crates need an ADR (`CLAUDE.md` §5): a protocol/GRBL core crate (with the simulator), an `-io` crate for serial and the transport, and a thin binary. The `CLAUDE.md` §8 crate-suffix list needs the connector binary added, like `curvyo-sync-server`. **`needs-customer`** for the security model (pairing, encryption) and because the relay later sends data out of the machine.
- Decide in the ADR: WebSocket with JSON or a binary frame; how encryption works so that a **browser** can use it: a browser page served over https cannot open plain `ws://` to a LAN address (mixed content) and cannot pin a self-signed certificate, so an application-level secure channel keyed from the pairing (for example a Noise-style handshake) over `ws://` is the likely answer, and an https-hosted app then reaches the connector only through the relay (Question 6). Also: how pairing reuses the device keypairs of ADR 0010 (ed25519 identity, X25519 agreement) so the relay slice needs no second identity system; how the stop path bypasses the job queue (a separate high-priority channel into the serial writer); where the G-code reader lives so the app and the connector share it.
- The connector holds no document and no user data beyond the job in flight and its job log.

## Decided (customer)

- **CONN-FIRST: A (2026-10-17).** The first connector talks GRBL over serial (USB), reached directly on the LAN first (the CONN-PATH default).

## Open questions (customer; each has a default)

Frage 1 (CONN-PATH): Which route comes first, direct on the LAN or through the sync service?
- A (default, recommended): direct on the LAN first; no server needed, testable at once. The relay follows when the sync service exists (slice 3).
- B: relay first, because the browser and remote use are the main reason.
- Default if unanswered: A. (Your request of 2026-11-02, "same LAN", is read as A.)

Frage 2 (CONN-SAFETY): Is a local start confirmation (criterion 29) built in the first version?
- A (default): yes, as an option, off by default.
- B: always required for a remote start.
- C: not built yet.
- Default if unanswered: A.

Frage 3 (CONN-GCODE): Where does the G-code of slice 1 come from, since nothing in Curvyo produces it yet?
- A (default, recommended): a G-code file the maker picks; Curvyo frames, sends and controls it. Smallest slice, tests the connector fully; "draw to laser" follows in slice 2 with `0029`.
- B: a minimal "cut all visible outlines" export in slice 1: one speed, one power, one pass, entered in the send panel. It is a thin first piece of `0029` and makes slice 1 end-to-end from the drawing, but it adds a G-code writer, unit and flip handling, and decisions `0027` and `0029` should make (roles, order, kerf), so it risks being redone.
- C: wait for `0026` to `0029` and the workbench shell, then build the connector.
- Default if unanswered: A.

Frage 4 (CONN-LOSS): When the app's connection drops during a job, what does the machine do?
- A (default): keep running; the job is the maker's, the log and reconnect show the truth (criterion 30).
- B: feed hold with laser off until someone reconnects (criterion 31). Safer for an unattended laser, but a Wi-Fi blip scorches the material at the hold point on some lasers.
- C: stop the job.
- Default if unanswered: A, with `on_link_lost = hold` available as a setting.

Frage 5 (CONN-HW): Which controller and machine do you test with first?
- A (default): any GRBL 1.1 controller over USB serial at 115200 baud (a diode laser on a Creality, Ortur, Sculpfun or a CNC 3018 type board); only GRBL 1.1 behaviour is specified. Please name your machine and firmware (GRBL 1.1, grblHAL, FluidNC in serial mode) so criterion 37 is run on it.
- B: a grblHAL or FluidNC machine; their extra messages must be added to the parser and fixtures.
- Default if unanswered: A, CI on the simulator only, hardware check at the demo.

Frage 6 (CONN-BROWSER): Which browsers must reach the connector in slice 1?
- A (default, recommended): the desktop app and a browser build served over plain `http` or from `localhost` (self-hosted or developer use); an app served over `https` (a hosted Curvyo) reaches machines only through the relay (slice 3), because browsers block `ws://` from an https page and reject unpinned self-signed certificates.
- B: the hosted https app must work in slice 1; the connector must then get a browser-trusted `wss` certificate (a hostname and a certificate the maker obtains, for example via a dynamic DNS name and Let's Encrypt), which is not "start and pair once".
- Default if unanswered: A.

Frage 7 (CONN-UI): What does the connector itself show?
- A (default): headless; terminal output and a log; commands `pair`, `devices`, `unpair`, `start` (local start), config file. Fits a Raspberry Pi over SSH.
- B: A plus a small status and approve page on `localhost` of the host.
- C: a tray icon app for desktops.
- Default if unanswered: A.

## Links

Requirements: R-MFG-LASER-002, R-MFG-LASER-003, R-SYS-005, R-SYS-007, R-SYS-008 (existing); R-MFG-011 (existing, Draft sketch). Proposed for `docs/requirements.md` (not edited in this staging run): R-MFG-011 text extended to "pairing, encryption, stop reachable, simulator"; new R-MFG-012 "A machine job is refused without a selected machine profile, also by the connector" (Proposal, Must, flows from R-SYS-008).
Depends on: `specs/0026` machine profile (reserved; slice 1 uses a stand-in), `specs/0029` laser job output (reserved, the G-code source for slice 2), ADR 0004 (sync service), ADR 0008 (end-to-end encryption), ADR 0010 (keyring, device keys), ADR 0011 (crate layout)
Related: `specs/0063-manufacturing-workbenches/`, `specs/0065-extensions-sdk/`, ADR 0007 (the other "connector")
ADRs: a connector ADR (protocol, pairing, encryption, crates), `needs-customer`; to be written by the architect
PR: -

## Customer decisions (2026-11-03)

- **CONN-GCODE: C.** Slice 1 waits for the manufacturing workbench (`0063`), which produces the G-code. This story is parked until then; the workbench and the connector side are being built by the Phai session, this spec, ADR 0032 (draft), the UX notes and the spike report are handed over as input.
- **CONN-LOSS: A, sharpened.** The connector receives the complete G-code before the run and can drive the job to the end on its own. Over the link (LAN or sync) only state and signals travel. The connector decides when a requested pause can take effect (at a safe point, feed hold); **stop goes through at once** (soft reset), accepting that steps may be lost.
- **CONN-BROWSER: B.** The hosted https app must reach connectors in slice 1.
- **CONN-UI:** inside Curvyo, no separate connector UI; probably in the Properties panel next to the machine (place to be decided with the workbench).
- **CONN-SEC:** to be aligned with the Phai session. Customer's direction: both self-signed https with public key pinning plus a password, and real certificates via EST (RFC 7030). ADR 0032 must be revised accordingly (it proposed Noise XX over `ws://` without TLS).
- **CONN-BIN: A.** Linux x86_64 and aarch64 suffice for testing.
- **STOP-KEY: C.** Stop is the button only; no keyboard shortcut.
- **JOG-RELEASE: A.** A short click jogs one step; press and hold jogs while held and stops on release.
- **CONN-PATH, CONN-SAFETY:** defaults (A, A) unless the Phai session decides otherwise with the customer.
