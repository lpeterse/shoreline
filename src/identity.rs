use pem_rfc7468::LineEnding;
use rand::{rand_core::UnwrapErr, rngs::SysRng};
use ed25519_dalek::{SigningKey};
use serde::{Deserialize, Serialize};
use ed25519_dalek::pkcs8::EncodePublicKey;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeyPair(SigningKey);

impl KeyPair {
    pub fn new() -> Self {
        let mut csprng = UnwrapErr(SysRng);
        let signing_key: SigningKey = SigningKey::generate(&mut csprng);
        Self(signing_key)
    }

    pub fn pubkey_as_pem(&self) -> String {
        self.0.verifying_key().to_public_key_pem(LineEnding::LF).unwrap()
    }

    pub fn pubkey_as_dht_id(&self) -> shoreline_dht::Id {
        let pubkey = self.0.verifying_key();
        let id_bytes = &pubkey.as_bytes()[0..20].try_into().unwrap();
        shoreline_dht::Id::from_bytes(id_bytes)
    }
}
