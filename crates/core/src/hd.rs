//! SLIP-10 hardened derivation for Ed25519 (BIP32/44 are secp256k1-only).
//! Not wired into the identity yet: switching would change every existing pseudonym,
//! so it needs a versioned migration. ML-DSA keys stay derived deterministically from
//! the (derived) seed via HKDF -> `from_seed` (FIPS 204 ML-DSA.KeyGen_internal).

use hmac::{Hmac, Mac};
use sha2::Sha512;
use zeroize::Zeroizing;

type H = Hmac<Sha512>;

/// Hardened offset.
pub const H_OFF: u32 = 0x8000_0000;

/// Derive (private key, chain code) for a hardened `path` (indices without the offset).
pub fn slip10_ed25519(seed: &[u8], path: &[u32]) -> (Zeroizing<[u8; 32]>, Zeroizing<[u8; 32]>) {
    let mut mac = H::new_from_slice(b"ed25519 seed").expect("hmac");
    mac.update(seed);
    let i = Zeroizing::new(<[u8; 64]>::from(mac.finalize().into_bytes()));
    let mut k = Zeroizing::new([0u8; 32]);
    let mut c = Zeroizing::new([0u8; 32]);
    k.copy_from_slice(&i[..32]);
    c.copy_from_slice(&i[32..]);
    for idx in path {
        let mut mac = H::new_from_slice(c.as_ref()).expect("hmac");
        mac.update(&[0u8]);
        mac.update(k.as_ref());
        mac.update(&(idx | H_OFF).to_be_bytes());
        let i = Zeroizing::new(<[u8; 64]>::from(mac.finalize().into_bytes()));
        k.copy_from_slice(&i[..32]);
        c.copy_from_slice(&i[32..]);
    }
    (k, c)
}

#[cfg(test)]
mod tests {
    use super::*;
    // SLIP-0010 test vector 1 for ed25519.
    #[test]
    fn slip10_vector1() {
        let seed = hex::decode("000102030405060708090a0b0c0d0e0f").unwrap();
        let (k, c) = slip10_ed25519(&seed, &[]);
        assert_eq!(
            hex::encode(*k),
            "2b4be7f19ee27bbf30c667b642d5f4aa69fd169872f8fc3059c08ebae2eb19e7"
        );
        assert_eq!(
            hex::encode(*c),
            "90046a93de5380a72b5e45010748567d5ea02bbf6522f979e05c0d8d8ca9fffb"
        );
        let (k, _) = slip10_ed25519(&seed, &[0]);
        assert_eq!(
            hex::encode(*k),
            "68e0fe46dfb67e368c75379acec591dad19df3cde26e63b93a8e704f1dade7a3"
        );
    }
}
