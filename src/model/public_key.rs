use std::str::FromStr;


#[derive(Clone, Debug, PartialEq, Eq, Hash, serde_with::SerializeDisplay, serde_with::DeserializeFromStr)]
pub struct PublicKey([u8; 32]);

impl PublicKey {
    pub const EXAMPLE: PublicKey = PublicKey([0u8; 32]);

    pub fn random() -> Self {
        let mut xs = [0u8; 32];
        for x in &mut xs {
            *x = rand::random();
        }
        Self(xs)
    }
}

impl std::fmt::Display for PublicKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

impl FromStr for PublicKey {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = hex::decode(s).map_err(|e| format!("Failed to decode hex: {}", e))?;
        if bytes.len() != 32 {
            return Err(format!("Invalid length: expected 32 bytes, got {}", bytes.len()));
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }
}
