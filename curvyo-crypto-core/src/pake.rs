//! `CPace` on ristretto255 with SHA-512 (draft-irtf-cfrg-cpace-21, §7 and §8.3), the
//! password-authenticated key exchange of device pairing (ADR 0040 decision 4).
//!
//! Hand-written on `curve25519-dalek`'s ristretto255 and `sha2`'s SHA-512; checked against the
//! draft's test vectors (appendix A.1 to A.3 and B.3). Scalars come from passed-in entropy
//! (ADR 0031 decision 13).

use curve25519_dalek::ristretto::{CompressedRistretto, RistrettoPoint};
use curve25519_dalek::scalar::Scalar;
use curve25519_dalek::traits::IsIdentity;
use sha2::{Digest, Sha512};
use zeroize::{Zeroize, Zeroizing};

use crate::CryptoError;

/// The domain-separation identifier of the group (§8.3).
const DSI: &[u8] = b"CPaceRistretto255";
/// The domain-separation identifier of the ISK hash (§7.2: `G.DSI || b"_ISK"`).
const DSI_ISK: &[u8] = b"CPaceRistretto255_ISK";
/// SHA-512's input block size `H.s_in_bytes`.
const HASH_BLOCK: usize = 128;

/// Appends `data` with its LEB128 length in front (`prepend_len`, appendix A.1.1).
fn prepend_len(out: &mut Vec<u8>, data: &[u8]) {
    let mut length = data.len();
    loop {
        // The mask keeps the value below 128, so the conversion never falls back.
        let low = u8::try_from(length & 0x7f).unwrap_or_default();
        length >>= 7;
        if length == 0 {
            out.push(low);
            break;
        }
        out.push(low | 0x80);
    }
    out.extend_from_slice(data);
}

/// The length-value concatenation `lv_cat` (appendix A.1.3).
pub(crate) fn lv_cat(parts: &[&[u8]]) -> Vec<u8> {
    let mut out = Vec::new();
    for part in parts {
        prepend_len(&mut out, part);
    }
    out
}

/// The generator string of §8.1 (appendix A.2): the zero padding fills SHA-512's first block.
fn generator_string(prs: &[u8], ci: &[u8], sid: &[u8]) -> Vec<u8> {
    let mut dsi_len = Vec::new();
    prepend_len(&mut dsi_len, DSI);
    let mut prs_len = Vec::new();
    prepend_len(&mut prs_len, prs);
    let zpad = HASH_BLOCK.saturating_sub(1 + prs_len.len() + dsi_len.len());
    lv_cat(&[DSI, prs, &vec![0; zpad], ci, sid])
}

/// `calculate_generator` of §8.3: SHA-512 of the generator string, mapped by ristretto255's
/// element derivation (RFC 9496 §4.3.4).
fn calculate_generator(prs: &[u8], ci: &[u8], sid: &[u8]) -> RistrettoPoint {
    let hash: [u8; 64] = Sha512::digest(generator_string(prs, ci, sid)).into();
    RistrettoPoint::from_uniform_bytes(&hash)
}

/// The initiator-responder transcript `transcript_ir` (appendix A.3.4).
pub(crate) fn transcript_ir(ya: &[u8], ada: &[u8], yb: &[u8], adb: &[u8]) -> Vec<u8> {
    let mut out = lv_cat(&[ya, ada]);
    out.extend_from_slice(&lv_cat(&[yb, adb]));
    out
}

/// One party's secret scalar and public share of a `CPace` run. The scalar is wiped on drop.
pub(crate) struct CpaceParty {
    scalar: Scalar,
    share: [u8; 32],
}

impl Drop for CpaceParty {
    fn drop(&mut self) {
        self.scalar.zeroize();
    }
}

impl CpaceParty {
    /// Computes the generator from the password `prs`, channel identifier `ci` and session id
    /// `sid`, samples the scalar from 32 passed-in bytes as §8.3 recommends (bits above
    /// `group_size_bits` = 252 cleared) and computes the public share. Clearing the top four
    /// bits gives a value uniform on `[0, 2^252)`, which is the scalar group order up to a
    /// statistical distance of about `2^-126`, as the draft recommends.
    ///
    /// Refuses entropy that gives the zero scalar (`Malformed`).
    pub(crate) fn new(
        prs: &[u8],
        ci: &[u8],
        sid: &[u8],
        entropy: &[u8; 32],
    ) -> Result<Self, CryptoError> {
        let mut bytes = *entropy;
        bytes[31] &= 0x0f;
        let scalar = Scalar::from_bytes_mod_order(bytes);
        bytes.zeroize();
        if scalar == Scalar::ZERO {
            return Err(CryptoError::Malformed);
        }
        let share = (calculate_generator(prs, ci, sid) * scalar)
            .compress()
            .to_bytes();
        Ok(Self { scalar, share })
    }

    /// The public share `Y` sent to the peer.
    pub(crate) fn share(&self) -> [u8; 32] {
        self.share
    }

    /// `scalar_mult_vfy` (§8.3): the shared point `K` from the peer's share. A share that does
    /// not decode, or a result that is the neutral element, aborts (`BadPublicKey`).
    fn shared_point(&self, peer_share: &[u8; 32]) -> Result<[u8; 32], CryptoError> {
        let point = CompressedRistretto(*peer_share)
            .decompress()
            .ok_or(CryptoError::BadPublicKey)?;
        let shared = point * self.scalar;
        if shared.is_identity() {
            return Err(CryptoError::BadPublicKey);
        }
        Ok(shared.compress().to_bytes())
    }

    /// The intermediate session key `ISK` (§7.2) for the given transcript; `K` is never
    /// exposed (§10.3). `K` and its length-value buffer are wiped; the internal state of
    /// SHA-512 (and of HKDF afterwards) is not.
    pub(crate) fn isk(
        &self,
        sid: &[u8],
        peer_share: &[u8; 32],
        transcript: &[u8],
    ) -> Result<[u8; 64], CryptoError> {
        let mut shared = self.shared_point(peer_share)?;
        let mut hash = Sha512::new();
        let prefix = Zeroizing::new(lv_cat(&[DSI_ISK, sid, &shared]));
        hash.update(prefix.as_slice());
        hash.update(transcript);
        shared.zeroize();
        Ok(hash.finalize().into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(text: &str) -> Vec<u8> {
        let text: String = text.split_whitespace().collect();
        (0..text.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
            .collect()
    }

    fn hex32(text: &str) -> [u8; 32] {
        hex(text).try_into().unwrap()
    }

    const PRS: &[u8] = b"Password";
    const CI: &str = "0b415f696e69746961746f720b425f726573706f6e646572";
    const SID: &str = "7e4b4791d6a8ef019b936c79fb7f2c57";
    const YA_SCALAR: &str = "da3d23700a9e5699258aef94dc060dfda5ebb61f02a5ea77fad53f4ff0976d08";
    const YB_SCALAR: &str = "d2316b454718c35362d83d69df6320f38578ed5984651435e2949762d900b80d";
    const YA: &str = "d6bac480f2c386c394efc7c47adb9925dcd2630b64f240c50f8d0eec482b9157";
    const YB: &str = "3ea7e0b19560d7c0b0f5734f63b955286dfa8232b5ebe63324e2d9e7433f7258";
    const K: &str = "80b69a8a76457ab6a4d7f887a4bf6b55a2f80ac19c333f917a05fc9887c8b40f";

    /// Appendix A.1.2 and A.1.4.
    #[test]
    fn prepend_len_and_lv_cat_match_the_draft() {
        let mut out = Vec::new();
        prepend_len(&mut out, b"");
        assert_eq!(out, hex("00"));
        assert_eq!(lv_cat(&[b"1234"]), hex("0431323334"));
        let range127: Vec<u8> = (0..127).collect();
        let mut expected = vec![0x7f];
        expected.extend_from_slice(&range127);
        assert_eq!(lv_cat(&[&range127]), expected);
        let range128: Vec<u8> = (0..128).collect();
        let mut expected = vec![0x80, 0x01];
        expected.extend_from_slice(&range128);
        assert_eq!(lv_cat(&[&range128]), expected);
        assert_eq!(
            lv_cat(&[b"1234", b"5", b"", b"678"]),
            hex("043132333401350003363738")
        );
    }

    /// Appendix A.3.5.
    #[test]
    fn transcript_ir_matches_the_draft() {
        assert_eq!(
            transcript_ir(b"123", b"PartyA", b"234", b"PartyB"),
            hex("03313233065061727479410332333406506172747942")
        );
        assert_eq!(
            transcript_ir(b"3456", b"PartyA", b"2345", b"PartyB"),
            hex("043334353606506172747941043233343506506172747942")
        );
    }

    /// Appendix B.3.1: generator string, its hash and the encoded generator.
    #[test]
    fn the_generator_matches_the_draft() {
        let string = generator_string(PRS, &hex(CI), &hex(SID));
        assert_eq!(string.len(), 170);
        assert_eq!(
            string,
            hex(concat!(
                "11435061636552697374726574746f3235350850617373776f726464",
                "00000000000000000000000000000000000000000000000000000000",
                "00000000000000000000000000000000000000000000000000000000",
                "00000000000000000000000000000000000000000000000000000000",
                "00000000000000000000000000000000180b415f696e69746961746f",
                "720b425f726573706f6e646572107e4b4791d6a8ef019b936c79fb7f",
                "2c57",
            ))
        );
        assert_eq!(
            Sha512::digest(&string).to_vec(),
            hex(concat!(
                "da6d3ddc8802fca9058755ffd3ebde08a9c2c74945901a258482a288",
                "b6663af06bf645c93cd1c51512307199c80e84908916d983b34af772",
                "05f90851a657ee27",
            ))
        );
        assert_eq!(
            calculate_generator(PRS, &hex(CI), &hex(SID))
                .compress()
                .to_bytes(),
            hex32("222b6b195fe84b1652badb6f6a3ae3d24341e7306967f0b8115b40d5698c7e56")
        );
    }

    /// Appendix B.3.2 to B.3.5: shares, the shared point and the initiator-responder ISK.
    #[test]
    fn shares_k_and_isk_match_the_draft() {
        let sid = hex(SID);
        let a = CpaceParty::new(PRS, &hex(CI), &sid, &hex32(YA_SCALAR)).unwrap();
        let b = CpaceParty::new(PRS, &hex(CI), &sid, &hex32(YB_SCALAR)).unwrap();
        assert_eq!(a.share(), hex32(YA));
        assert_eq!(b.share(), hex32(YB));
        assert_eq!(a.shared_point(&b.share()).unwrap(), hex32(K));
        assert_eq!(b.shared_point(&a.share()).unwrap(), hex32(K));
        let transcript = transcript_ir(&a.share(), b"ADa", &b.share(), b"ADb");
        assert_eq!(
            transcript,
            hex(concat!(
                "20d6bac480f2c386c394efc7c47adb9925dcd2630b64f240c50f8d0e",
                "ec482b915703414461203ea7e0b19560d7c0b0f5734f63b955286dfa",
                "8232b5ebe63324e2d9e7433f725803414462",
            ))
        );
        let isk = hex(concat!(
            "b69effbf61b51d56401c0f65601abe428de8206feaaf0e32198896dc",
            "ae7b35cd2b38950a39dfd5d4a79164614c2984f7daa460b588c1e80c",
            "3fa2068af7900447",
        ));
        assert_eq!(a.isk(&sid, &b.share(), &transcript).unwrap().to_vec(), isk);
        assert_eq!(b.isk(&sid, &a.share(), &transcript).unwrap().to_vec(), isk);
    }

    /// Appendix B.3.10 and B.3.11: a valid `scalar_mult_vfy`, and the two invalid shares that
    /// must abort.
    #[test]
    fn scalar_mult_vfy_matches_the_draft_and_refuses_invalid_shares() {
        let party = CpaceParty {
            scalar: Scalar::from_bytes_mod_order(hex32(
                "7cd0e075fa7955ba52c02759a6c90dbbfc10e6d40aea8d283e407d88cf538a05",
            )),
            share: [0; 32],
        };
        assert_eq!(
            party
                .shared_point(&hex32(
                    "2c3c6b8c4f3800e7aef6864025b4ed79bd599117e427c41bd47d93d654b4a51c"
                ))
                .unwrap(),
            hex32("7c13645fe790a468f62c39beb7388e541d8405d1ade69d1778c5fe3e7f6b600e")
        );
        assert_eq!(
            party.shared_point(&hex32(
                "2b3c6b8c4f3800e7aef6864025b4ed79bd599117e427c41bd47d93d654b4a51c"
            )),
            Err(CryptoError::BadPublicKey)
        );
        assert_eq!(party.shared_point(&[0; 32]), Err(CryptoError::BadPublicKey));
    }

    #[test]
    fn entropy_giving_the_zero_scalar_is_refused() {
        let mut entropy = [0; 32];
        entropy[31] = 0xf0;
        assert!(matches!(
            CpaceParty::new(PRS, b"", b"", &entropy),
            Err(CryptoError::Malformed)
        ));
    }

    #[test]
    fn a_different_password_gives_a_different_isk() {
        let sid = [9; 16];
        let a = CpaceParty::new(b"right", b"ci", &sid, &[1; 32]).unwrap();
        let b = CpaceParty::new(b"wrong", b"ci", &sid, &[2; 32]).unwrap();
        let transcript = transcript_ir(&b.share(), b"", &a.share(), b"");
        assert_ne!(
            a.isk(&sid, &b.share(), &transcript).unwrap(),
            b.isk(&sid, &a.share(), &transcript).unwrap()
        );
    }
}
