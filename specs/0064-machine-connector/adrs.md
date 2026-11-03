# ADRs for the machine connector

Architect, 2026-11-02. Customer decisions: CONN-FIRST A (GRBL over serial, 2026-10-17); CONN-SEC B, TLS with key
pinning or an EST certificate, and CONN-BROWSER A and B (2026-11-02); CONN-PATH
default A (direct on the LAN first); CONN-SAFETY default A (local start as an option, off). Customer
input of 2026-11-02: at least the protocol between connector and workbench is next.

- [ADR 0035](../../docs/adr/0035-machine-connector-protocol.md) (new, Accepted by the customer
  2026-11-02): crates `curvyo-connector-core` (messages, version check, job state machine,
  chunk assembly; wasm32), `curvyo-connector-io` and `curvyo-connector-server`; WebSocket over TLS 1.3
  (`rustls` with `ring`); connector key pinned at pairing (SPAKE2 over the code, SPKI bound) or a
  CA certificate via EST (config option); paired clients hold a token; JSON messages, protocol 0;
  job = GRBL G-code text; browser through the web app the connector serves, and the relay later. Covers criteria 2, 3, 6, 9 (protocol part), 10, 11.
- [ADR 0004](../../docs/adr/0004-persistence-and-cross-machine-sync.md) §4, §5: the relay route
  and machine-resource leases, later.
- [ADR 0007](../../docs/adr/0007-asset-library-connector-and-credential-storage.md): the OS
  credential store holds the app's pairing token; the "asset connector" is a different thing.
- [ADR 0008](../../docs/adr/0008-end-to-end-encryption-of-sync-and-collaboration.md): the relay
  sees ciphertext only; how the TLS channel is carried end to end through it is the relay story's decision.
- [ADR 0010](../../docs/adr/0010-document-keyring-admins-and-revocation.md): not reused (ADR 0035
  option D).

## Feature-local decisions

- 2026-11-02: The first story builds only `curvyo-connector-core` with a fake controller test
  double; `-io` (WebSocket, TLS, pinning, pairing) and `-server` follow as one story; serial and GRBL
  streaming as the next.
- 2026-11-02: Criterion 4 (connector named in the machine profile, connection test) waits for
  `0026`; until then the connector address, its pinned key hash and the pairing token (in the OS credential store) are a per-user app setting beside
  `extensions.toml`, never in the project file (a shared project must not carry a workshop's
  address or keys).
