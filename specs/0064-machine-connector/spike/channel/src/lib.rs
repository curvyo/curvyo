//! Spike 0064: Noise XX (snow) plus SPAKE2 pairing bound to the handshake hash, with all
//! randomness supplied by the caller as seed bytes. Throwaway code, never merged.

use rand_chacha::ChaCha20Rng;
use rand_core::{RngCore, SeedableRng};
use snow::params::NoiseParams;
use snow::resolvers::{CryptoResolver, DefaultResolver, FallbackResolver};
use snow::types::{Cipher, Dh, Hash, Random};
use spake2::{Ed25519Group, Identity, Password, Spake2};

/// Noise pattern from ADR 0032 Decision 2.
pub const PATTERN: &str = "Noise_XX_25519_ChaChaPoly_BLAKE2s";

/// snow `Random` backed by a ChaCha20 stream seeded by the caller.
struct SeededRandom(ChaCha20Rng);

impl Random for SeededRandom {
    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), snow::Error> {
        self.0.fill_bytes(dest);
        Ok(())
    }
}

/// Resolver that only provides the RNG; everything else falls back to snow's default resolver.
struct SeedResolver([u8; 32]);

impl CryptoResolver for SeedResolver {
    fn resolve_rng(&self) -> Option<Box<dyn Random>> {
        Some(Box::new(SeededRandom(ChaCha20Rng::from_seed(self.0))))
    }
    fn resolve_dh(&self, _: &snow::params::DHChoice) -> Option<Box<dyn Dh>> {
        None
    }
    fn resolve_hash(&self, _: &snow::params::HashChoice) -> Option<Box<dyn Hash>> {
        None
    }
    fn resolve_cipher(&self, _: &snow::params::CipherChoice) -> Option<Box<dyn Cipher>> {
        None
    }
}

fn builder(seed: [u8; 32]) -> snow::Builder<'static> {
    let params: NoiseParams = PATTERN.parse().expect("spike: pattern parses");
    snow::Builder::with_resolver(
        params,
        Box::new(FallbackResolver::new(Box::new(SeedResolver(seed)), Box::new(DefaultResolver))),
    )
}

/// Generates a static X25519 keypair from caller seed bytes: (private, public).
pub fn keypair(seed: [u8; 32]) -> (Vec<u8>, Vec<u8>) {
    let kp = builder(seed).generate_keypair().expect("spike: keypair");
    (kp.private, kp.public)
}

/// Outcome of one full in-memory run: both transport states work, PAKE keys match.
pub struct RunResult {
    pub handshake_hash_equal: bool,
    pub remote_static_seen_by_initiator: Vec<u8>,
    pub transport_roundtrip_ok: bool,
    pub pake_keys_equal: bool,
    pub tamper_rejected: bool,
}

/// Runs XX between app (initiator) and connector (responder), then SPAKE2 over `code_a`/`code_b`
/// with the Noise handshake hash as identity binding, then one transport message each way.
pub fn run(seeds: [[u8; 32]; 6], code_a: &str, code_b: &str) -> RunResult {
    let (app_priv, _app_pub) = keypair(seeds[0]);
    let (con_priv, con_pub) = keypair(seeds[1]);
    let mut ini = builder(seeds[2]).local_private_key(&app_priv).unwrap().build_initiator().unwrap();
    let mut res = builder(seeds[3]).local_private_key(&con_priv).unwrap().build_responder().unwrap();
    let mut buf = [0u8; 65535];
    let mut out = [0u8; 65535];
    // -> e
    let n = ini.write_message(&[], &mut buf).unwrap();
    res.read_message(&buf[..n], &mut out).unwrap();
    // <- e, ee, s, es
    let n = res.write_message(&[], &mut buf).unwrap();
    ini.read_message(&buf[..n], &mut out).unwrap();
    let remote = ini.get_remote_static().unwrap().to_vec();
    // -> s, se
    let n = ini.write_message(&[], &mut buf).unwrap();
    res.read_message(&buf[..n], &mut out).unwrap();
    let h_i = ini.get_handshake_hash().to_vec();
    let h_r = res.get_handshake_hash().to_vec();
    let mut ti = ini.into_transport_mode().unwrap();
    let mut tr = res.into_transport_mode().unwrap();

    // SPAKE2 over the code; the handshake hash is the shared identity, so a MITM (two hashes) fails.
    let (sa, ma) = Spake2::<Ed25519Group>::start_a_with_rng(
        &Password::new(code_a.as_bytes()),
        &Identity::new(&[b"app:" as &[u8], &h_i].concat()),
        &Identity::new(&[b"con:" as &[u8], &h_i].concat()),
        ChaCha20Rng::from_seed(seeds[4]),
    );
    let (sb, mb) = Spake2::<Ed25519Group>::start_b_with_rng(
        &Password::new(code_b.as_bytes()),
        &Identity::new(&[b"app:" as &[u8], &h_r].concat()),
        &Identity::new(&[b"con:" as &[u8], &h_r].concat()),
        ChaCha20Rng::from_seed(seeds[5]),
    );
    // PAKE messages travel inside Noise transport messages.
    let n = ti.write_message(&ma, &mut buf).unwrap();
    let k = tr.read_message(&buf[..n], &mut out).unwrap();
    let ka = sa.finish(&mb).unwrap();
    let kb = sb.finish(&out[..k]).unwrap();

    let n = tr.write_message(br#"{"t":"hello","protocol":[1,0]}"#, &mut buf).unwrap();
    let k = ti.read_message(&buf[..n], &mut out).unwrap();
    let roundtrip = &out[..k] == br#"{"t":"hello","protocol":[1,0]}"#;
    let n = ti.write_message(b"stop", &mut buf).unwrap();
    buf[n - 1] ^= 1;
    let tamper_rejected = tr.read_message(&buf[..n], &mut out).is_err();

    RunResult {
        handshake_hash_equal: h_i == h_r,
        remote_static_seen_by_initiator: if remote == con_pub { remote } else { Vec::new() },
        transport_roundtrip_ok: roundtrip,
        pake_keys_equal: ka == kb,
        tamper_rejected,
    }
}

/// wasm entry used by the size measurement and the node test: returns a bitmask of passed checks.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn spike_run(seed_bytes: &[u8], code_a: &str, code_b: &str) -> u32 {
    if cfg!(feature = "baseline") {
        return seed_bytes.len() as u32 + code_a.len() as u32 + code_b.len() as u32;
    }
    let mut seeds = [[0u8; 32]; 6];
    for (i, s) in seeds.iter_mut().enumerate() {
        s.copy_from_slice(&seed_bytes[i * 32..(i + 1) * 32]);
    }
    let r = run(seeds, code_a, code_b);
    u32::from(r.handshake_hash_equal)
        | u32::from(!r.remote_static_seen_by_initiator.is_empty()) << 1
        | u32::from(r.transport_roundtrip_ok) << 2
        | u32::from(r.pake_keys_equal) << 3
        | u32::from(r.tamper_rejected) << 4
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seeds() -> [[u8; 32]; 6] {
        core::array::from_fn(|i| [i as u8 + 1; 32])
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    fn handshake_and_pairing_with_the_right_code() {
        let r = run(seeds(), "12345678", "12345678");
        assert!(r.handshake_hash_equal);
        assert!(!r.remote_static_seen_by_initiator.is_empty());
        assert!(r.transport_roundtrip_ok);
        assert!(r.pake_keys_equal);
        assert!(r.tamper_rejected);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    fn wrong_code_gives_different_keys() {
        let r = run(seeds(), "12345678", "12345679");
        assert!(!r.pake_keys_equal);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    fn same_seeds_same_keys() {
        assert_eq!(keypair([7; 32]), keypair([7; 32]));
        assert_ne!(keypair([7; 32]).1, keypair([8; 32]).1);
    }
}
