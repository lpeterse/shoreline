use serde::{Deserialize, Serialize};

use bencode_minimal::{TryFromValue, Value};
use std::{num::ParseIntError, str::FromStr};

#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct Id([u8; Self::BYTES]);

impl Id {
    pub const BYTES: usize = 20;
    pub const UNKNOWN: Self = Self([0; Self::BYTES]);

    pub fn from_bytes(bytes: &[u8; Self::BYTES]) -> Self {
        Self(*bytes)
    }

    pub fn try_from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != Self::BYTES {
            return None;
        }
        let mut x = [0; Self::BYTES];
        x.copy_from_slice(bytes);
        Some(Self(x))
    }

    pub fn xor(&self, b: &Self) -> Self {
        let mut x = [0; Self::BYTES];
        for i in 0..Self::BYTES {
            x[i] = self.0[i] ^ b.0[i];
        }
        Self(x)
    }

    pub fn not(&self) -> Self {
        let mut x = [0; Self::BYTES];
        for i in 0..Self::BYTES {
            x[i] = !self.0[i];
        }
        Self(x)
    }

    pub fn similarity(&self, other: &Self) -> usize {
        let mut cnt = 0;
        for i in 0..Self::BYTES {
            let x = self.0[i] ^ other.0[i];
            if x == 0 {
                cnt += 8;
            } else {
                cnt += x.leading_zeros() as usize;
                break;
            }
        }
        cnt
    }

    pub fn distance(&self, other: &Self) -> usize {
        Self::BYTES * 8 - self.similarity(other)
    }

    pub fn random() -> Self {
        Self(rand::random())
    }

    pub fn random_in_bucket(&self, n: usize) -> Self {
        let mut x = Self::random().0;
        let n = n.min(Self::BYTES * 8);
        let q = n / 8;
        let r = n % 8;

        x[0..q].copy_from_slice(&self.0[..q]);

        let m0 = (0xFFu16 << (8 - r)) as u8;
        let m1 = 1 << (7 - r);
        let m2 = (0xFFu16 >> (r + 1)) as u8;

        let a = self.0[q];
        let b = x[q];

        x[q] = (a & m0) | (!a & m1) | (b & m2);

        Self(x)
    }

    pub fn is_null(&self) -> bool {
        self.0.iter().all(|&x| x == 0)
    }
}

impl AsRef<[u8]> for Id {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl<'a> TryFromValue<'a> for Id {
    fn try_from(value: &'a Value<'a>) -> Option<Self> {
        value.try_into::<[u8; 20]>().map(Self)
    }
}

impl FromStr for Id {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut x = [0; 20];
        for (a, b) in (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16)).zip(x.iter_mut()) {
            *b = a?;
        }

        Ok(Self(x))
    }
}

impl std::fmt::Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for x in self.as_ref() {
            write!(f, "{:02x}", x)?;
        }
        Ok(())
    }
}

impl std::fmt::Debug for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for x in self.as_ref() {
            write!(f, "{:08b}", x)?;
        }
        Ok(())
    }
}

impl<'a> bencode_minimal::IntoStr<'a> for Id {
    fn into_str(self) -> bencode_minimal::Str<'static> {
        self.0.to_vec().into_str()
    }
}

impl<'a> bencode_minimal::IntoStr<'a> for &'a Id {
    fn into_str(self) -> bencode_minimal::Str<'a> {
        self.0.as_ref().into_str()
    }
}

impl Serialize for Id {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Id {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Id::from_str(&s).map_err(serde::de::Error::custom)
    }
}
