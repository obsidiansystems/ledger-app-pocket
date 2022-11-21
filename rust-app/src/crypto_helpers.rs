use crate::info;
use core::default::Default;
//use core::option::NoneError;
use core::fmt;
use nanos_sdk::bindings::*;
use nanos_sdk::io::SyscallError;
use zeroize::{DefaultIsZeroes, Zeroizing};
use core::ops::{Deref,DerefMut};
use arrayvec::{CapacityError,ArrayVec};
use ledger_log::*;
use ledger_crypto_helpers::common::*;
use ledger_crypto_helpers::eddsa::*;

pub const BIP32_PREFIX: [u32; 3] = nanos_sdk::ecc::make_bip32_path(b"m/44'/635'");

// Public Key Hash type; update this to match the target chain's notion of an address and how to
// format one.

pub struct PKH(pub [u8; 20]);

impl Address<PKH, nanos_sdk::ecc::ECPublicKey<65, 'E'>> for PKH {
    fn get_address(key: &nanos_sdk::ecc::ECPublicKey<65, 'E'>) -> Result<Self, SyscallError> {
        get_pkh(key)
    }
    fn get_binary_address(&self) -> &[u8] {
        &self.0
    }
}

#[allow(dead_code)]
pub fn get_pkh(key: &nanos_sdk::ecc::ECPublicKey<65, 'E'>) -> Result<PKH, SyscallError> {
    let mut public_key_hash = [0; 32];
    let key_bytes = ed25519_public_key_bytes(key);
    unsafe {
        let _len: size_t = cx_hash_sha256(
            key_bytes.as_ptr(),
            key_bytes.len() as u32,
            public_key_hash.as_mut_ptr(),
            public_key_hash.len() as u32,
        );
    }
    let mut rv=PKH([0; 20]);
    rv.0.clone_from_slice(&public_key_hash[0..20]);
    Ok(rv)
}

impl fmt::Display for PKH {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "")?;
        for byte in self.0 {
            write!(f, "{:02x}", byte)?;
        }
        Ok(())
    }
}
