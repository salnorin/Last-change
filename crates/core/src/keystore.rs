//! Encrypted keystore: the 32-byte master seed encrypted with XChaCha20-Poly1305
//! under an Argon2id key derived from the user's passphrase. No profile data.

use crate::{CoreError, Identity};
use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    XChaCha20Poly1305, XNonce,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keystore {
    pub version: u32,
    pub kdf: String,
    pub m_cost_kib: u32,
    pub t_cost: u32,
    pub p_cost: u32,
    pub salt: String,
    pub nonce: String,
    pub ciphertext: String,
    /// Public, for display only.
    pub pseudonym: String,
}

fn kdf(pass: &str, salt: &[u8], m: u32, t: u32, p: u32) -> Result<[u8; 32], CoreError> {
    let mut k = [0u8; 32];
    Argon2::new(
        Algorithm::Argon2id,
        Version::V0x13,
        Params::new(m, t, p, Some(32)).map_err(|e| CoreError::Crypto(e.to_string()))?,
    )
    .hash_password_into(pass.as_bytes(), salt, &mut k)
    .map_err(|e| CoreError::Crypto(e.to_string()))?;
    Ok(k)
}

impl Keystore {
    /// Default parameters: Argon2id, 64 MiB, 3 passes, 1 lane.
    pub fn encrypt(id: &Identity, pass: &str) -> Result<Self, CoreError> {
        Self::encrypt_with(id, pass, 64 * 1024, 3, 1)
    }

    pub fn encrypt_with(
        id: &Identity,
        pass: &str,
        m: u32,
        t: u32,
        p: u32,
    ) -> Result<Self, CoreError> {
        if pass.len() < 8 {
            return Err(CoreError::Crypto(
                "passphrase must be at least 8 characters".into(),
            ));
        }
        let mut salt = [0u8; 16];
        let mut nonce = [0u8; 24];
        rand::RngCore::fill_bytes(&mut rand::rngs::OsRng, &mut salt);
        rand::RngCore::fill_bytes(&mut rand::rngs::OsRng, &mut nonce);
        let key = zeroize::Zeroizing::new(kdf(pass, &salt, m, t, p)?);
        let ct = XChaCha20Poly1305::new(&(*key).into())
            .encrypt(XNonce::from_slice(&nonce), id.master_bytes().as_slice())
            .map_err(|_| CoreError::Crypto("encrypt".into()))?;
        Ok(Self {
            version: 1,
            kdf: "argon2id".into(),
            m_cost_kib: m,
            t_cost: t,
            p_cost: p,
            salt: hex::encode(salt),
            nonce: hex::encode(nonce),
            ciphertext: hex::encode(ct),
            pseudonym: id.pseudonym(),
        })
    }

    pub fn decrypt(&self, pass: &str) -> Result<Identity, CoreError> {
        let bad = || CoreError::Crypto("wrong passphrase or corrupted keystore".into());
        let salt = hex::decode(&self.salt).map_err(|_| bad())?;
        let nonce = hex::decode(&self.nonce).map_err(|_| bad())?;
        if nonce.len() != 24 {
            return Err(bad());
        }
        let key =
            zeroize::Zeroizing::new(kdf(pass, &salt, self.m_cost_kib, self.t_cost, self.p_cost)?);
        let pt = zeroize::Zeroizing::new(
            XChaCha20Poly1305::new(&(*key).into())
                .decrypt(
                    XNonce::from_slice(&nonce),
                    hex::decode(&self.ciphertext).map_err(|_| bad())?.as_slice(),
                )
                .map_err(|_| bad())?,
        );
        let mut m = zeroize::Zeroizing::new([0u8; 32]);
        if pt.len() != 32 {
            return Err(bad());
        }
        m.copy_from_slice(&pt);
        Ok(Identity::from_master(&m))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn encrypt_decrypt() {
        let id = Identity::generate();
        let ks = Keystore::encrypt_with(&id, "correct horse", 1024, 1, 1).unwrap();
        assert_eq!(
            ks.decrypt("correct horse").unwrap().pseudonym(),
            id.pseudonym()
        );
        assert!(ks.decrypt("wrong pass!").is_err());
        assert!(Keystore::encrypt_with(&id, "short", 1024, 1, 1).is_err());
        let json = serde_json::to_string(&ks).unwrap();
        assert!(!json.contains(&hex::encode(id.master_bytes())));
    }
}
