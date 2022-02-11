//! Public key structures and algorithms
//!
//! Definition of public key structure and helpers

use ark_ff::{BigInteger, PrimeField};
use bs58;
use core::fmt;
use sha2::{Digest, Sha256};
use std::ops::Neg;

use crate::{BaseField, CurvePoint};
use o1_utils::FieldHelpers;

/// Length of Mina addresses
pub const MINA_ADDRESS_LEN: usize = 55;
const MINA_ADDRESS_RAW_LEN: usize = 40;

/// Public key
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct PubKey(CurvePoint);

impl PubKey {
    /// Create a public key from curve point
    /// Note: Does not check point is on curve
    pub fn from_point_unsafe(point: CurvePoint) -> Self {
        Self(point)
    }

    /// Deserialize Mina address into public key
    pub fn from_address(address: &str) -> Result<Self, &'static str> {
        if address.len() != MINA_ADDRESS_LEN {
            return Err("Invalid address length");
        }

        let bytes = bs58::decode(address)
            .into_vec()
            .map_err(|_| "Invalid address encoding")?;

        if bytes.len() != MINA_ADDRESS_RAW_LEN {
            return Err("Invalid raw address bytes length");
        }

        let (raw, checksum) = (&bytes[..bytes.len() - 4], &bytes[bytes.len() - 4..]);
        let hash = Sha256::digest(&Sha256::digest(raw)[..]);
        if checksum != &hash[..4] {
            return Err("Invalid address checksum");
        }

        let (version, x_bytes, y_parity) = (
            &raw[..3],
            &raw[3..bytes.len() - 5],
            raw[bytes.len() - 5] == 0x01,
        );
        if version != [0xcb, 0x01, 0x01] {
            return Err("Invalid address version info");
        }

        let x = BaseField::from_bytes(x_bytes).map_err(|_| "invalid x-coordinate bytes")?;
        let mut pt =
            CurvePoint::get_point_from_x(x, y_parity).ok_or("Invalid address x-coordinate")?;

        if pt.y.into_repr().is_even() == y_parity {
            pt.y = pt.y.neg();
        }

        if !pt.is_on_curve() {
            return Err("Point is not on curve");
        }

        // Safe now because we checked point pt is on curve
        Ok(PubKey::from_point_unsafe(pt))
    }

    /// Convert public key into curve point
    pub fn into_point(self) -> CurvePoint {
        self.0
    }

    /// Convert public key into compressed public key
    pub fn into_compressed(self) -> CompressedPubKey {
        let point = self.into_point();
        CompressedPubKey {
            x: point.x,
            is_odd: point.y.into_repr().is_odd(),
        }
    }

    /// Serialize public key into corresponding Mina address
    pub fn into_address(self) -> String {
        let point = self.into_point();
        into_address(point.x, point.y.into_repr().is_odd())
    }
}

impl fmt::Display for PubKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let point = self.into_point();
        let mut x_bytes = point.x.to_bytes();
        let mut y_bytes = point.y.to_bytes();
        x_bytes.reverse();
        y_bytes.reverse();

        write!(f, "{}{}", hex::encode(x_bytes), hex::encode(y_bytes))
    }
}

/// Compressed public keys consist of x-coordinate and y-coordinate parity.
#[derive(Clone, Copy)]
pub struct CompressedPubKey {
    /// X-coordinate
    pub x: BaseField,

    /// Parity of y-coordinate
    pub is_odd: bool,
}

fn into_address(x: BaseField, is_odd: bool) -> String {
    let mut raw: Vec<u8> = vec![
        0xcb, // version for base58 check
        0x01, // non_zero_curve_point version
        0x01, // compressed_poly version
    ];

    // pub key x-coordinate
    raw.extend(x.to_bytes());

    // pub key y-coordinate parity
    raw.push(is_odd as u8);

    // 4-byte checksum
    let hash = Sha256::digest(&Sha256::digest(&raw[..])[..]);
    raw.extend(&hash[..4]);

    // The raw buffer is MINA_ADDRESS_RAW_LEN (= 40) bytes in length
    bs58::encode(raw).into_string()
}

impl CompressedPubKey {
    /// Serialize compressed public key into corresponding Mina address
    pub fn into_address(self) -> String {
        into_address(self.x, self.is_odd)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_address() {
        macro_rules! assert_from_address_check {
            ($address:expr) => {
                let pk = PubKey::from_address($address).expect("failed to create pubkey");
                assert_eq!(pk.into_address(), $address);
            };
        }

        assert_from_address_check!("B62qnzbXmRNo9q32n4SNu2mpB8e7FYYLH8NmaX6oFCBYjjQ8SbD7uzV");
        assert_from_address_check!("B62qicipYxyEHu7QjUqS7QvBipTs5CzgkYZZZkPoKVYBu6tnDUcE9Zt");
        assert_from_address_check!("B62qoG5Yk4iVxpyczUrBNpwtx2xunhL48dydN53A2VjoRwF8NUTbVr4");
        assert_from_address_check!("B62qrKG4Z8hnzZqp1AL8WsQhQYah3quN1qUj3SyfJA8Lw135qWWg1mi");
        assert_from_address_check!("B62qoqiAgERjCjXhofXiD7cMLJSKD8hE8ZtMh4jX5MPNgKB4CFxxm1N");
        assert_from_address_check!("B62qkiT4kgCawkSEF84ga5kP9QnhmTJEYzcfgGuk6okAJtSBfVcjm1M");
    }
}
