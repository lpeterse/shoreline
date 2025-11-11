
pub struct IdentityProvider {
    identity: Option<Identity>,
}

impl IdentityProvider {
    pub fn new() -> Self {
        Self { identity: None }
    }

    pub fn get(&self) -> Option<&Identity> {
        self.identity.as_ref()
    }

    pub fn create(&mut self) {
        self.identity = Some(Identity::create());
    }
}

pub struct Identity {
    pubkey: PublicKey,
    seckey: SecretKey,
}

impl Identity {
    pub fn new() -> Self {
        Self {
            pubkey: PublicKey([0u8; 32]),
            seckey: SecretKey([0u8; 64]),
        }
    }

    pub fn load() -> Option<Self> {
        None
    }

    pub fn create() -> Self {
        Self::new()
    }
}


/// Ed25519 Public Key
pub struct PublicKey([u8; 32]);

/// Ed25519 Secret Key
pub struct SecretKey([u8; 64]);
