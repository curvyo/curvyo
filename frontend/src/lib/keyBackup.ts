// The browser's passkey key backup (`specs/0096-own-devices-sync` M9, ADR 0040 decision 13,
// Frage 145 A): a passkey with the WebAuthn PRF extension gives 32 secret bytes; Rust derives the
// record's wrap key, id and write key from them, seals and signs; this file asks the passkey and
// carries one exchange's frames over its own socket. The PRF output never leaves this tab.

import { type WasmKeyBackup, key_backup_prf_input } from "@/wasm-bindings/curvyo_editor_wasm.js";

/** What asking the passkey gave: its PRF output and credential id, or why not. */
export type PasskeyAnswer =
  | { kind: "prf"; prf: Uint8Array; credential: Uint8Array }
  | { kind: "no-prf" }
  | { kind: "cancelled" };

/** How one exchange with the server ended (the JSON of `WasmKeyBackup.take_outcome`). */
export type BackupOutcome =
  | { kind: "read"; identity: string; room: string; server: string; label: string }
  | { kind: "done" }
  | { kind: string; text: string };

/** How long one exchange may take before it counts as failed (ms). */
const EXCHANGE_MS = 15_000;

function random(length: number): Uint8Array {
  const bytes = new Uint8Array(length);
  crypto.getRandomValues(bytes);
  return bytes;
}

/** Whether this page can ask for a passkey at all (a secure context with WebAuthn). */
export function passkeysAvailable(): boolean {
  return typeof window !== "undefined" && window.isSecureContext && "PublicKeyCredential" in window;
}

type PrfResults = { enabled?: boolean; results?: { first?: BufferSource } };

function prfOf(credential: Credential | null): PrfResults | undefined {
  const extensions = (credential as PublicKeyCredential | null)?.getClientExtensionResults() as
    | { prf?: PrfResults }
    | undefined;
  return extensions?.prf;
}

function bytesOf(source: BufferSource): Uint8Array {
  return source instanceof ArrayBuffer ? new Uint8Array(source) : new Uint8Array(source.buffer, source.byteOffset, source.byteLength);
}

function prfExtension() {
  return { prf: { eval: { first: key_backup_prf_input() as BufferSource } } } as AuthenticationExtensionsClientInputs;
}

/** Asks an existing passkey of this site (any, or `credential`) for its PRF output. */
export async function askPasskey(credential?: Uint8Array): Promise<PasskeyAnswer> {
  let got: Credential | null;
  try {
    got = await navigator.credentials.get({
      publicKey: {
        challenge: random(32) as BufferSource,
        userVerification: "required",
        allowCredentials: credential ? [{ type: "public-key", id: credential as BufferSource }] : [],
        extensions: prfExtension(),
      },
    });
  } catch {
    return { kind: "cancelled" };
  }
  const first = prfOf(got)?.results?.first;
  if (!got || !first) {
    return { kind: "no-prf" };
  }
  return { kind: "prf", prf: bytesOf(first), credential: new Uint8Array((got as PublicKeyCredential).rawId) };
}

/** Makes a new discoverable passkey for this device's backup and asks it for its PRF output;
 * a passkey without PRF is reported (and the browser told to forget it where it can). */
export async function makePasskey(label: string): Promise<PasskeyAnswer> {
  let made: Credential | null;
  try {
    made = await navigator.credentials.create({
      publicKey: {
        rp: { name: "Curvyo" },
        user: { id: random(16) as BufferSource, name: `Curvyo keys: ${label}`, displayName: label },
        challenge: random(32) as BufferSource,
        pubKeyCredParams: [
          { type: "public-key", alg: -8 },
          { type: "public-key", alg: -7 },
          { type: "public-key", alg: -257 },
        ],
        authenticatorSelection: { residentKey: "required", userVerification: "required" },
        extensions: prfExtension(),
      },
    });
  } catch {
    return { kind: "cancelled" };
  }
  if (!made) {
    return { kind: "cancelled" };
  }
  const credential = new Uint8Array((made as PublicKeyCredential).rawId);
  const prf = prfOf(made);
  const first = prf?.results?.first;
  if (first) {
    return { kind: "prf", prf: bytesOf(first), credential };
  }
  if (prf?.enabled) {
    // Some authenticators evaluate the PRF on use only; one that still gives none is forgotten.
    const asked = await askPasskey(credential);
    if (asked.kind === "no-prf") {
      forgetPasskey(credential);
    }
    return asked;
  }
  forgetPasskey(credential);
  return { kind: "no-prf" };
}

/** Tells the passkey provider a credential is of no use (where the browser offers it). */
function forgetPasskey(credential: Uint8Array) {
  const signal = (
    PublicKeyCredential as unknown as {
      signalUnknownCredential?: (options: { rpId: string; credentialId: string }) => Promise<void>;
    }
  ).signalUnknownCredential;
  if (!signal) {
    return;
  }
  const id = btoa(String.fromCharCode(...credential)).replaceAll("+", "-").replaceAll("/", "_").replace(/=+$/, "");
  void signal({ rpId: location.hostname, credentialId: id }).catch(() => undefined);
}

/** Runs one exchange with the relay at `address` on its own socket; frees `backup`. */
export function runExchange(address: string, backup: WasmKeyBackup): Promise<BackupOutcome> {
  return new Promise((resolve) => {
    let socket: WebSocket | null = null;
    let ended = false;
    const end = () => {
      if (ended) {
        return;
      }
      ended = true;
      window.clearTimeout(timer);
      backup.closed();
      const outcome = JSON.parse(backup.take_outcome()) as BackupOutcome;
      backup.free();
      socket?.close();
      resolve(outcome);
    };
    const timer = window.setTimeout(end, EXCHANGE_MS);
    try {
      socket = new WebSocket(address);
    } catch {
      end();
      return;
    }
    socket.binaryType = "arraybuffer";
    socket.onopen = () => socket?.send(backup.first_frame() as BufferSource);
    socket.onmessage = (event) => {
      if (!(event.data instanceof ArrayBuffer)) {
        return;
      }
      const answer = backup.receive(new Uint8Array(event.data));
      if (answer) {
        socket?.send(answer as BufferSource);
      }
      if (backup.finished()) {
        end();
      }
    };
    socket.onclose = end;
    socket.onerror = end;
  });
}

/** The key store record that says this device is backed up (not a secret, not in the backup). */
export const BACKUP_RECORD = "backup";

/** What the `backup` record holds. */
export interface BackupMark {
  server: string;
  /** The passkey's credential id, so Remove asks that passkey. */
  credential: number[];
}

/** 24 fresh bytes for sealing a record. */
export function sealNonce(): Uint8Array {
  return random(24);
}
