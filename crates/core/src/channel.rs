//! Hybrid post-quantum encrypted deal messages: X25519 + ML-KEM-768 (FIPS 203).
//! The AEAD key = HKDF-SHA256(ss_mlkem || ss_x25519, transcript). The message stays
//! confidential as long as EITHER primitive holds. ChaCha20-Poly1305 AEAD.
//!
//! Note: the libp2p transport (Noise XX, X25519) is NOT post-quantum yet. This
//! application-layer channel protects deal details end to end on top of it.

use crate::{derive, CoreError, Identity};
use chacha20poly1305::{
    aead::{Aead, KeyInit, Payload as AeadPayload},
    ChaCha20Poly1305, Nonce,
};
use ml_kem::{
    kem::{Decapsulate, Encapsulate},
    EncodedSizeUser, KemCore, MlKem768,
};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use x25519_dalek::{PublicKey, StaticSecret};

type Ek = <MlKem768 as KemCore>::EncapsulationKey;
type Dk = <MlKem768 as KemCore>::DecapsulationKey;

/// Public receiving keys a peer shares (e.g. inside a DealAccept or out of band).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KemPublic {
    pub x25519: String,
    pub mlkem768: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sealed {
    pub eph_x25519: String,
    pub kem_ct: String,
    pub nonce: String,
    pub ct: String,
}

fn kem_keys(master: &[u8; 32]) -> (StaticSecret, Dk, Ek) {
    let xs = StaticSecret::from(derive(master, "x25519"));
    let (dk, ek) = MlKem768::generate_deterministic(
        &derive(master, "ml-kem-768-d").into(),
        &derive(master, "ml-kem-768-z").into(),
    );
    (xs, dk, ek)
}

fn aead_key(ss_kem: &[u8], ss_x: &[u8], eph: &[u8], ct: &[u8], rx: &[u8]) -> ChaCha20Poly1305 {
    let mut ikm = ss_kem.to_vec();
    ikm.extend_from_slice(ss_x);
    let mut info = b"lastchange-hybrid-kem-v1".to_vec();
    info.extend_from_slice(eph);
    info.extend_from_slice(ct);
    info.extend_from_slice(rx);
    let mut k = [0u8; 32];
    hkdf::Hkdf::<Sha256>::new(None, &ikm)
        .expand(&info, &mut k)
        .expect("hkdf");
    ChaCha20Poly1305::new(&k.into())
}

impl Identity {
    pub fn kem_public(&self) -> KemPublic {
        let (xs, _, ek) = kem_keys(&self.master_bytes());
        KemPublic {
            x25519: hex::encode(PublicKey::from(&xs).as_bytes()),
            mlkem768: hex::encode(ek.as_bytes().as_slice()),
        }
    }

    /// Decrypt a message sealed to this identity. `aad` must match.
    pub fn open(&self, s: &Sealed, aad: &[u8]) -> Result<Vec<u8>, CoreError> {
        let bad = |m: &str| CoreError::Crypto(m.into());
        let (xs, dk, _) = kem_keys(&self.master_bytes());
        let eph: [u8; 32] = hex::decode(&s.eph_x25519)
            .ok()
            .and_then(|v| v.try_into().ok())
            .ok_or_else(|| bad("eph"))?;
        let ctb = hex::decode(&s.kem_ct).map_err(|_| bad("ct"))?;
        let ct =
            ml_kem::Ciphertext::<MlKem768>::try_from(ctb.as_slice()).map_err(|_| bad("ct len"))?;
        let ss_kem = dk.decapsulate(&ct).map_err(|_| bad("decaps"))?;
        let ss_x = xs.diffie_hellman(&PublicKey::from(eph));
        let rx = PublicKey::from(&xs);
        let cipher = aead_key(
            ss_kem.as_slice(),
            ss_x.as_bytes(),
            &eph,
            &ctb,
            rx.as_bytes(),
        );
        let nonce = hex::decode(&s.nonce).map_err(|_| bad("nonce"))?;
        if nonce.len() != 12 {
            return Err(bad("nonce len"));
        }
        cipher
            .decrypt(
                Nonce::from_slice(&nonce),
                AeadPayload {
                    msg: &hex::decode(&s.ct).map_err(|_| bad("hex"))?,
                    aad,
                },
            )
            .map_err(|_| bad("decrypt failed"))
    }
}

/// Encrypt `pt` to a recipient's hybrid public keys.
pub fn seal_to(to: &KemPublic, pt: &[u8], aad: &[u8]) -> Result<Sealed, CoreError> {
    let bad = |m: &str| CoreError::Crypto(m.into());
    let rx: [u8; 32] = hex::decode(&to.x25519)
        .ok()
        .and_then(|v| v.try_into().ok())
        .ok_or_else(|| bad("x25519 key"))?;
    let ekb = hex::decode(&to.mlkem768).map_err(|_| bad("ek hex"))?;
    let ek = Ek::from_bytes(
        &ml_kem::Encoded::<Ek>::try_from(ekb.as_slice()).map_err(|_| bad("ek len"))?,
    );
    let mut rng = rand::rngs::OsRng;
    let (ct, ss_kem) = ek.encapsulate(&mut rng).map_err(|_| bad("encaps"))?;
    let eph = StaticSecret::random_from_rng(rng);
    let eph_pub = PublicKey::from(&eph);
    let ss_x = eph.diffie_hellman(&PublicKey::from(rx));
    let cipher = aead_key(
        ss_kem.as_slice(),
        ss_x.as_bytes(),
        eph_pub.as_bytes(),
        ct.as_slice(),
        &rx,
    );
    let mut nonce = [0u8; 12];
    rand::RngCore::fill_bytes(&mut rng, &mut nonce);
    let c = cipher
        .encrypt(Nonce::from_slice(&nonce), AeadPayload { msg: pt, aad })
        .map_err(|_| bad("encrypt"))?;
    Ok(Sealed {
        eph_x25519: hex::encode(eph_pub.as_bytes()),
        kem_ct: hex::encode(ct.as_slice()),
        nonce: hex::encode(nonce),
        ct: hex::encode(c),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roundtrip_and_tamper() {
        let bob = Identity::generate();
        let s = seal_to(&bob.kem_public(), b"meet at the station, 5pm", b"deal-123").unwrap();
        assert_eq!(
            bob.open(&s, b"deal-123").unwrap(),
            b"meet at the station, 5pm"
        );
        assert!(bob.open(&s, b"deal-999").is_err(), "aad mismatch");
        assert!(
            Identity::generate().open(&s, b"deal-123").is_err(),
            "wrong recipient"
        );
        let mut t = s.clone();
        let mut c = hex::decode(&t.ct).unwrap();
        c[0] ^= 1;
        t.ct = hex::encode(c);
        assert!(bob.open(&t, b"deal-123").is_err(), "ciphertext tamper");
        let mut t = s.clone();
        let mut c = hex::decode(&t.kem_ct).unwrap();
        c[5] ^= 1;
        t.kem_ct = hex::encode(c);
        assert!(bob.open(&t, b"deal-123").is_err(), "kem ct tamper");
        assert_eq!(hex::decode(&s.kem_ct).unwrap().len(), 1088);
    }
}
