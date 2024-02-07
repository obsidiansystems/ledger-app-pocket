//use core::option::NoneError;
use core::fmt;
use ledger_crypto_helpers::common::*;
use ledger_crypto_helpers::eddsa::*;
use ledger_device_sdk::io::SyscallError;
use ledger_secure_sdk_sys::*;

// Public Key Hash type; update this to match the target chain's notion of an address and how to
// format one.

pub struct PKH(pub [u8; 20]);

impl Address<PKH, ledger_device_sdk::ecc::ECPublicKey<65, 'E'>> for PKH {
    fn get_address(
        key: &ledger_device_sdk::ecc::ECPublicKey<65, 'E'>,
    ) -> Result<Self, SyscallError> {
        get_pkh(key)
    }
    fn get_binary_address(&self) -> &[u8] {
        &self.0
    }
}

#[allow(dead_code)]
pub fn get_pkh(key: &ledger_device_sdk::ecc::ECPublicKey<65, 'E'>) -> Result<PKH, SyscallError> {
    let mut public_key_hash = [0; 32];
    let key_bytes = ed25519_public_key_bytes(key);
    unsafe {
        let _len = cx_hash_sha256(
            key_bytes.as_ptr(),
            key_bytes.len(),
            public_key_hash.as_mut_ptr(),
            public_key_hash.len(),
        );
    }
    let mut rv = PKH([0; 20]);
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
