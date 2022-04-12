use crate::info;
use core::default::Default;
use core::option::NoneError;
use core::fmt;
use nanos_sdk::bindings::*;
use nanos_sdk::io::SyscallError;
use zeroize::{DefaultIsZeroes, Zeroizing};
use core::ops::{Deref,DerefMut};
use arrayvec::{CapacityError,ArrayVec};
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

#[inline(always)]
pub fn get_pubkey_from_privkey(ec_k: &mut nanos_sdk::bindings::cx_ecfp_private_key_t, pubkey: &mut nanos_sdk::bindings::cx_ecfp_public_key_t) -> Result<(), SyscallError> {
    info!("Calling generate_pair_no_throw");
    call_c_api_function!(cx_ecfp_generate_pair_no_throw(CX_CURVE_Ed25519, pubkey, ec_k, true))?;
    info!("Calling compress_point_no_throw");
    call_c_api_function!(cx_edwards_compress_point_no_throw(CX_CURVE_Ed25519, pubkey.W.as_mut_ptr(), pubkey.W_len))?;
    pubkey.W_len = 33;
    Ok(())
}

#[derive(Default,Copy,Clone)]
// Would like to use ZeroizeOnDrop here, but the zeroize_derive crate doesn't build. We also would
// need Zeroize on cx_ecfp_private_key_t instead of using DefaultIsZeroes; we can't implement both
// Drop and Copy.
struct PrivateKey(nanos_sdk::bindings::cx_ecfp_private_key_t);
impl DefaultIsZeroes for PrivateKey {}
impl Deref for PrivateKey {
  type Target = nanos_sdk::bindings::cx_ecfp_private_key_t;
  fn deref(&self) -> &Self::Target {
      &self.0
  }
}
impl DerefMut for PrivateKey {
  fn deref_mut(&mut self) -> &mut Self::Target {
      &mut self.0
  }
}

pub enum CryptographyError {
  NoneError,
  SyscallError(SyscallError),
  CapacityError(CapacityError)
}

impl From<SyscallError> for CryptographyError {
    fn from(e: SyscallError) -> Self {
        CryptographyError::SyscallError(e)
    }
}
impl From<CapacityError> for CryptographyError {
    fn from(e: CapacityError) -> Self {
        CryptographyError::CapacityError(e)
    }
}
impl From<NoneError> for CryptographyError {
    fn from(_: NoneError) -> Self {
        CryptographyError::NoneError
    }
}

// #[inline(always)]
pub fn with_private_key<A>(
    path: &[u32],
    f: impl FnOnce(&mut nanos_sdk::bindings::cx_ecfp_private_key_t) -> Result<A, CryptographyError>
) -> Result<A, CryptographyError> {
    info!("Deriving path");
    let raw_key = bip32_derive_eddsa(path)?;
    let mut ec_k : Zeroizing<PrivateKey> = Default::default();
    info!("Generating key");
    call_c_api_function!(cx_ecfp_init_private_key_no_throw(
            CX_CURVE_Ed25519,
            raw_key.as_ptr(),
            raw_key.len() as u32,
            (&mut ec_k).deref_mut().deref_mut() as *mut nanos_sdk::bindings::cx_ecfp_private_key_t
        ))?;
    info!("Key generated");
    f(&mut ec_k as &mut nanos_sdk::bindings::cx_ecfp_private_key_t)
}

pub fn with_public_keys<A>(
  path: &[u32],
  f: impl FnOnce(&nanos_sdk::bindings::cx_ecfp_public_key_t, &PKH) -> Result<A, CryptographyError>
) -> Result<A, CryptographyError> {
    let mut pubkey = Default::default();
    with_private_key(path, |ec_k| {
        info!("Getting private key");
        get_pubkey_from_privkey(ec_k, &mut pubkey).ok()?;
        Ok(())
    })?;
    let pkh = get_pkh(&pubkey)?;
    f(&pubkey, &pkh)
}

pub fn with_keys<A>(
  path: &[u32],
  f: impl FnOnce(&nanos_sdk::bindings::cx_ecfp_private_key_t, &nanos_sdk::bindings::cx_ecfp_public_key_t, &PKH) -> Result<A, CryptographyError>
) -> Result<A, CryptographyError> {
    let mut pubkey = Default::default();
    with_private_key(path, |ec_k| {
        info!("Getting private key");
        get_pubkey_from_privkey(ec_k, &mut pubkey)?;
        let pkh = get_pkh(&pubkey)?;
        f(ec_k, &pubkey, &pkh)
    })
}

pub fn public_key_bytes(key: &nanos_sdk::bindings::cx_ecfp_public_key_t) -> &[u8] {
    &key.W[1..33]
}

// Public Key Hash type; update this to match the target chain's notion of an address and how to
// format one.

pub struct PKH([u8; 20]);

#[allow(dead_code)]
pub fn get_pkh(key: &nanos_sdk::bindings::cx_ecfp_public_key_t) -> Result<PKH, SyscallError> {
    let mut public_key_hash = [0; 32];
    let key_bytes = public_key_bytes(key);
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


#[derive(Clone)]
pub struct SHA512(cx_sha512_s);

impl SHA512 {
    pub fn new() -> SHA512 {
        let mut rv = cx_sha512_s::default();
        unsafe { cx_sha512_init_no_throw(&mut rv) };
        Self(rv)
    }

    pub fn clear(&mut self) {
        unsafe { cx_sha512_init_no_throw(&mut self.0) };
    }

    pub fn update(&mut self, bytes: &[u8]) {
        unsafe {
            info!("HASHING: {}\n{:?}", HexSlice(bytes), core::str::from_utf8(bytes));
            cx_hash_update(
                &mut self.0 as *mut cx_sha512_s as *mut cx_hash_t,
                bytes.as_ptr(),
                bytes.len() as u32,
            );
        }
    }

    pub fn finalize(&mut self) -> Zeroizing<[u8; 64]> {
        let mut rv = Zeroizing::new([0; 64]);
        unsafe {
            cx_hash_final(
                &mut self.0 as *mut cx_sha512_s as *mut cx_hash_t,
                rv.as_mut_ptr(),
            )
        };
        rv
    }
}

#[derive(Clone)]
pub struct Ed25519 {
    hash: SHA512,
    path: ArrayVec<u32, 10>,
    r_pre: [u8; 32],
    r: [u8; 32],
}

#[derive(Clone)]
pub struct Ed25519Signature([u8; 64]);

impl Ed25519 {
    pub fn new(path : &ArrayVec<u32, 10>) -> Result<Ed25519,()> {
        let mut hash = SHA512::new();

        let nonce = with_private_key(path, |&mut key| {
            hash.update(&key.d[0..(key.d_len as usize)]);
            let temp = Zeroizing::new(hash.finalize());
            hash.clear();
            hash.update(&temp[32..64]);
            Ok(())
        });
        
        Ok(Self {
            hash: hash,
            path: path.clone(),
            r_pre: [0; 32],
            r: [0; 32]
        })
    }

    pub fn update(&mut self, bytes: &[u8]) {
        self.hash.update(bytes);
    }

    pub fn done_with_r(&mut self) {
        call_c_api_function!( cx_bn_lock(32,0) ).ok()?;
        self.r_pre = self.hash.finalize();
        self.r_pre.reverse();
        let r = CX_BN_FLAG_UNSET;
        
        // Make r_raw into a BN
        call_c_api_function!( cx_bn_alloc_init(&r as *mut cx_bn_t, 64, self.r_pre.as_ptr(), self.r_pre.len()) ).ok()?;
        
        let ed_P = cx_ecpoint_t::default();
        // Get the generator for Ed25519's curve
        call_c_api_function!( cx_ecdomain_generator_bn(CX_CURVE_Ed25519, ed_P) ).ok()?;

        // Multiply r by generator, store in ed_P
        call_c_api_function!( cx_ecpoint_rnd_scalarmul_bn(ed_P, r) );

        let big_r : [u8;32] = [0; 32];
        let mut sign;
        
        call_c_api_function!( cx_ecpoint_compress(ed_P, self.r, self.r.len(), sign) ).ok()?;
        
        call_c_api_function!( cx_bn_unlock() ).ok()?;
       
        self.r.reverse();
        self.r[31] |= 0x80 * sign;

        // Start calculating s.

        self.hash.clear();
        self.hash.update(&self.r);

        with_public_keys(&self.path, |key, _| {
            self.hash.update(&key.W[..key.W_len]);
        });
    }

    pub fn finalize(&mut self) -> Ed25519Signature {

        call_c_api_function!( cx_bn_lock(32,0) ).ok()?;
        let k_raw = self.hash.finalize();

        // Make k into a BN
        let k;
        call_c_api_function!( cx_bn_alloc_init(&k as *mut cx_bn_t, 64, k_raw.as_ptr(), k_raw.len()) ).ok()?;

        let rv = with_private_key(&self.path, |key| {
            call_c_api_function!( cx_bn_alloc_init(&k as *mut cx_bn_t, 64, self.r.as_ptr(), self.r.len()) ).ok()?;
            let rv;
            let ed25519_order;
            call_c_api_function!( cx_ecdomain_parameter_bn( CX_CURVE_Ed25519, CX_CURVE_PARAM_Order, ed25519_order) ).ok()?;
            call_c_api_function!( cx_bn_mod_mul(rv, key, k, ed25519_order) ).ok()?;
            rv
        });

        let r;
        call_c_api_function!( cx_bn_alloc_init(&r as *mut cx_bn_t, 64, self.r_pre.as_ptr(), self.r_pre.len())).ok()?;

        let s;
        call_c_api_function!( cx_bn_mod_add(s, rv, r)).ok()?;
        
        let s_bytes = [0; 64];
        call_c_api_function!(cx_bn_export(s, s_bytes.as_ptr(), s_bytes.len())).ok()?;

        s_bytes.reverse();

        let buf = [0; 64];

        buf[..32].copy_from_slice(&self.r);
        buf[32..].copy_from_slice(&s);

        Ok(Ed25519Signature(buf))
    }
}
