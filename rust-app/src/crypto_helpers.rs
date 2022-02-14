use crate::info;
use core::convert::TryInto;
use core::default::Default;
use core::fmt;
use nanos_sdk::bindings::*;
use nanos_sdk::io::SyscallError;

use ledger_log::*;

pub const BIP32_PATH: [u32; 5] = nanos_sdk::ecc::make_bip32_path(b"m/44'/535348'/0'/0/0");

/// Helper function that derives the seed over Ed25519
pub fn bip32_derive_eddsa(path: &[u32]) -> Result<[u8; 32], SyscallError> {
    let mut raw_key = [0u8; 32];
    trace!("Calling os_perso_derive_node_bip32 with path {:?}", path);
    unsafe {
        os_perso_derive_node_bip32(
            CX_CURVE_Ed25519,
            path.as_ptr(),
            path.len() as u32,
            raw_key.as_mut_ptr(),
            core::ptr::null_mut()
        )
    };
    trace!("Success");
    Ok(raw_key)
}

pub struct EdDSASig(pub [u8; 64]);

macro_rules! call_c_api_function {
    ($($call:tt)*) => {
        {
            let err = unsafe {
                $($call)*
            };
            if err != 0 {
                Err(SyscallError::from(err))
            } else {
                Ok(())
            }
        }
    }
}

#[inline(never)]
pub fn eddsa_sign(
    m: &[u8],
    ec_k: &cx_ecfp_private_key_t,
) -> Option<EdDSASig> {
    let mut sig:[u8;64]=[0; 64];
    trace!("Signing");
    call_c_api_function!(
         cx_eddsa_sign_no_throw(
            ec_k,
            CX_SHA512,
            m.as_ptr(),
            m.len() as u32,
            sig.as_mut_ptr(),
            sig.len() as u32)
    ).ok()?;
    trace!("Signed");
    Some(EdDSASig(sig))
}

#[inline(never)]
pub fn get_pubkey(path: &[u32]) -> Result<nanos_sdk::bindings::cx_ecfp_public_key_t, SyscallError> {
    info!("Getting private key");
    let mut ec_k = get_private_key(path)?;
    info!("Getting public key");
    get_pubkey_from_privkey(&mut ec_k)
}

pub fn get_pubkey_from_privkey(ec_k: &mut nanos_sdk::bindings::cx_ecfp_private_key_t) -> Result<nanos_sdk::bindings::cx_ecfp_public_key_t, SyscallError> {
    let mut pubkey = cx_ecfp_public_key_t::default();

    info!("Calling generate_pair_no_throw");
    call_c_api_function!(cx_ecfp_generate_pair_no_throw(CX_CURVE_Ed25519, &mut pubkey, ec_k, true))?;
    info!("Calling compress_point_no_throw");
    call_c_api_function!(cx_edwards_compress_point_no_throw(CX_CURVE_Ed25519, pubkey.W.as_mut_ptr(), pubkey.W_len))?;
    pubkey.W_len = 33;

    Ok(pubkey)
}

// #[inline(always)]
pub fn get_private_key(
    path: &[u32],
) -> Result<nanos_sdk::bindings::cx_ecfp_private_key_t, SyscallError> {
    info!("Deriving path");
    let raw_key = bip32_derive_eddsa(path)?;
    let mut ec_k = cx_ecfp_private_key_t::default();
    info!("Generating key");
    call_c_api_function!(cx_ecfp_init_private_key_no_throw(
            CX_CURVE_Ed25519,
            raw_key.as_ptr(),
            raw_key.len() as u32,
            &mut ec_k
        ))?;
    info!("Key generated");
    Ok(ec_k)
}

// Public Key Hash type; update this to match the target chain's notion of an address and how to
// format one.

pub struct PKH([u8; 20]);

#[allow(dead_code)]
pub fn get_pkh(key: nanos_sdk::bindings::cx_ecfp_public_key_t) -> PKH {
    let mut public_key_hash = [0; 32];
    /*unsafe {
        let _len: size_t = cx_hash_sha256(
            key.W.as_ptr(),
            33,
            public_key_hash.as_mut_ptr(),
            public_key_hash.len() as u32,
        );
    }*/
    let mut rv=PKH([0; 20]);
    // rv.0.clone_from_slice(&public_key_hash[0..20]);
    rv
}

impl fmt::Display for PKH {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "")?;
        for byte in self.0 {
            write!(f, "{:02X}", byte)?;
        }
        Ok(())
    }
}

struct HexSlice<'a>(&'a [u8]);

// You can choose to implement multiple traits, like Lower and UpperHex
impl fmt::Display for HexSlice<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            // Decide if you want to pad the value or have spaces inbetween, etc.
            write!(f, "{:02X}", byte)?;
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct Hasher(cx_sha256_s);

impl Hasher {
    pub fn new() -> Hasher {
        let mut rv = cx_sha256_s::default();
        unsafe { cx_sha256_init_no_throw(&mut rv) };
        Self(rv)
    }

    pub fn update(&mut self, bytes: &[u8]) {
        unsafe {
            info!("HASHING: {}\n{:?}", HexSlice(bytes), core::str::from_utf8(bytes));
            cx_hash_update(
                &mut self.0 as *mut cx_sha256_s as *mut cx_hash_t,
                bytes.as_ptr(),
                bytes.len() as u32,
            );
        }
    }

    pub fn finalize(&mut self) -> Hash {
        let mut rv = <[u8; 32]>::default();
        unsafe {
            cx_hash_final(
                &mut self.0 as *mut cx_sha256_s as *mut cx_hash_t,
                rv.as_mut_ptr(),
            )
        };
        Hash(rv)
    }
}

pub struct Hash(pub [u8; 32]);

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(f, "{:02X}", byte)?;
        }
        Ok(())
    }
}

extern "C" {
  pub fn cx_ecfp_decode_sig_der(input: *const u8, input_len: size_t,
      max_size: size_t,
      r: *mut *const u8, r_len: *mut size_t,
      s: *mut *const u8, s_len: *mut size_t,
      ) -> u32;
}

