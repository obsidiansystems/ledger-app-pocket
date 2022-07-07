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

pub const BIP32_PATH: [u32; 5] = nanos_sdk::ecc::make_bip32_path(b"m/44'/635'/0'/0/0");

pub const BIP32_PREFIX: [u32; 3] = nanos_sdk::ecc::make_bip32_path(b"m/44'/535348'");

/// Helper function that derives the seed over Ed25519
pub fn bip32_derive_eddsa(path: &[u32]) -> Result<[u8; 64], SyscallError> {

    if ! path.starts_with(&BIP32_PREFIX[0..2]) {
        // There isn't a _no_throw variation of the below, so avoid a throw on incorrect input.
        return Err(SyscallError::Security);
    }

    // Note: os_perso_derive_node_bip32 appears to write 64 bytes for CX_CURVE_Ed25519, despite the
    // private key for ed25519 being only 32 bytes. We still need to give it the space to write to,
    // of course.
    let mut raw_key = [0u8; 64];
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

macro_rules! call_c_api_function {
    ($($call:tt)*) => {
        {
            let err = unsafe {
                $($call)*
            };
            if err != 0 {
 //               error!("Syscall errored: {:?}", SyscallError::from(err));
                Err(SyscallError::from(err))
            } else {
                Ok(())
            }
        }
    }
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

#[derive(Debug)]
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
            32, // raw_key is 64 bytes because of system call weirdness, but we only want 32.
            (&mut ec_k).deref_mut().deref_mut() as *mut nanos_sdk::bindings::cx_ecfp_private_key_t
        )).ok()?;
    info!("Key generated");
    f(ec_k.deref_mut().deref_mut())
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

pub struct PKH(pub [u8; 20]);

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
            // info!("HASHING (Protocol): {}\n{:?}", HexSlice(bytes), core::str::from_utf8(bytes));
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


#[derive(Clone, Copy)]
pub struct SHA512(cx_sha512_s);

impl SHA512 {
    pub fn new() -> SHA512 {
        let mut rv = cx_sha512_s::default();
        unsafe { cx_sha512_init_no_throw(&mut rv) };
        Self(rv)
    }

    pub fn clear(&mut self) {
        trace!("Clearing Hasher");
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

struct BnLock;

impl BnLock {
    fn lock() -> Result<Self, CryptographyError> {
        call_c_api_function!( cx_bn_lock(32,0) )?;
        trace!("Locking BN");
        Ok(BnLock)
    }
}

impl Drop for BnLock {
    fn drop(&mut self) {
        trace!("Unlocking BN");
        call_c_api_function!( cx_bn_unlock() ).unwrap();
    }
}

#[derive(Clone)]
pub struct Ed25519 {
    hash: SHA512,
    path: ArrayVec<u32, 10>,
    r_pre: Zeroizing<[u8; 64]>,
    r: [u8; 32],
}
impl Default for Ed25519 {
    fn default() -> Ed25519 {
        Ed25519 {
            hash: SHA512::new(),
            path: ArrayVec::default(),
            r_pre: Zeroizing::new([0; 64]),
            r: [0; 32]
        }
    }
}

#[derive(Clone,Debug,PartialEq)]
pub struct Ed25519Signature(pub [u8; 64]);

impl Ed25519 {
    #[inline(never)]
    pub fn new(path : &ArrayVec<u32, 10>) -> Result<Ed25519,CryptographyError> {
        let mut rv = Self::default();
        rv.init(path)?;
        Ok(rv)
    }
    #[inline(never)]
    pub fn init(&mut self, path : &ArrayVec<u32, 10>) -> Result<(),CryptographyError> {
        self.hash.clear();

        with_private_key(path, |&mut key| {
            self.hash.update(&key.d[0..(key.d_len as usize)]);
            let temp = self.hash.finalize();
            self.hash.clear();
            self.hash.update(&temp[32..64]);
            Ok(())
        }).ok()?;

        self.path = path.clone();

        self.r_pre = Zeroizing::new([0; 64]);
        self.r = [0; 32];
        Ok(())
    }

    #[inline(never)]
    pub fn update(&mut self, bytes: &[u8]) {
        self.hash.update(bytes);
    }

    #[inline(never)]
    pub fn done_with_r(&mut self) -> Result<(), CryptographyError> {
        let mut sign = 0;
        {
            let _lock = BnLock::lock();
            trace!("done_with_r lock");
            let mut r = CX_BN_FLAG_UNSET;
            // call_c_api_function!( cx_bn_lock(32,0) ).ok()?;
            trace!("ping");
            self.r_pre = self.hash.finalize();
            self.r_pre.reverse();

            // Make r_pre into a BN
            call_c_api_function!( cx_bn_alloc_init(&mut r as *mut cx_bn_t, 64, self.r_pre.as_ptr(), self.r_pre.len() as u32) ).ok()?;
            trace!("ping");

            let mut ed_p = cx_ecpoint_t::default();
            // Get the generator for Ed25519's curve
            call_c_api_function!( cx_ecpoint_alloc(&mut ed_p as *mut cx_ecpoint_t, CX_CURVE_Ed25519) ).ok()?;
            trace!("ping");
            call_c_api_function!( cx_ecdomain_generator_bn(CX_CURVE_Ed25519, &mut ed_p) ).ok()?;
            trace!("ping");

            // Multiply r by generator, store in ed_p
            call_c_api_function!( cx_ecpoint_scalarmul_bn(&mut ed_p, r) ).ok()?;
            trace!("ping");

            // and copy/compress it to self.r
            call_c_api_function!( cx_ecpoint_compress(&ed_p, self.r.as_mut_ptr(), self.r.len() as u32, &mut sign) ).ok()?;
            trace!("ping");
        }

            trace!("ping");
        // and do the mandated byte order and bit twiddling.
        self.r.reverse();
        self.r[31] |= if sign != 0 { 0x80 } else { 0x00 };
            trace!("ping");

        // self.r matches the reference algorithm at this point.

        // Start calculating s.

        self.hash.clear();
            trace!("ping");
        self.hash.update(&self.r);
            trace!("ping");

        let path_tmp = self.path.clone();
            trace!("ping");
        with_public_keys(&path_tmp, |key, _| {
            // Note: public key has a byte in front of it in W, from how the ledger's system call
            // works; it's not for ed25519.
            trace!("ping");
            self.hash.update(&key.W[1..key.W_len as usize]);
            trace!("ping");
            Ok(())
        }).ok()?;
        Ok(())
    }

    // After done_with_r, we stream the message in again with "update".

    #[inline(never)]
    pub fn finalize(&mut self) -> Result<Ed25519Signature, CryptographyError> {
        
        // Need to make a variable for this.hash so that the closure doesn't capture all of self,
        // including self.path
        let hash_ref = &mut self.hash;
        let (h_a, _lock, ed25519_order) = with_private_key(&self.path, |key| {

            let _lock = BnLock::lock();
            trace!("finalize lock");

            let mut h_scalar = hash_ref.finalize();

            h_scalar.reverse();

            // Make k into a BN
            let mut h_scalar_bn = CX_BN_FLAG_UNSET;
            call_c_api_function!( cx_bn_alloc_init(&mut h_scalar_bn as *mut cx_bn_t, 64, h_scalar.as_ptr(), h_scalar.len() as u32) ).ok()?;

            // Get the group order
            let mut ed25519_order = CX_BN_FLAG_UNSET;
            call_c_api_function!( cx_bn_alloc(&mut ed25519_order, 64) ).ok()?;
            call_c_api_function!( cx_ecdomain_parameter_bn( CX_CURVE_Ed25519, CX_CURVE_PARAM_Order, ed25519_order) ).ok()?;

            // Generate the hashed private key
            let mut rv = CX_BN_FLAG_UNSET;
            hash_ref.clear();
            hash_ref.update(&key.d[0..(key.d_len as usize)]);
            let mut temp : Zeroizing<_> = hash_ref.finalize();

            // Bit twiddling for ed25519
            temp[0] &= 248;
            temp[31] &= 63;
            temp[31] |= 64;

            let key_slice = &mut temp[0..32];

            key_slice.reverse();
            let mut key_bn = CX_BN_FLAG_UNSET;

            // Load key into bn
            call_c_api_function!( cx_bn_alloc_init(&mut key_bn as *mut cx_bn_t, 64, key_slice.as_ptr(), key_slice.len() as u32) ).ok()?;
            hash_ref.clear();

            call_c_api_function!( cx_bn_alloc(&mut rv, 64) ).ok()?;

            // multiply h_scalar_bn by key_bn
            call_c_api_function!( cx_bn_mod_mul(rv, key_bn, h_scalar_bn, ed25519_order) ).ok()?;

            // Destroy the private key, so it doesn't leak from with_private_key even in the bn
            // area. temp will zeroize on drop already.
            call_c_api_function!( cx_bn_destroy(&mut key_bn) ).ok()?;
            Ok((rv, _lock, ed25519_order))
        })?;

        // Reload the r value into the bn area
        let mut r = CX_BN_FLAG_UNSET;
        call_c_api_function!( cx_bn_alloc_init(&mut r as *mut cx_bn_t, 64, self.r_pre.as_ptr(), self.r_pre.len() as u32)).ok()?;

        // finally, compute s:
        let mut s = CX_BN_FLAG_UNSET;
        call_c_api_function!( cx_bn_alloc(&mut s, 64) ).ok()?;
        call_c_api_function!( cx_bn_mod_add(s, h_a, r, ed25519_order)).ok()?;

        // and copy s back to normal memory to return.
        let mut s_bytes = [0; 32];
        call_c_api_function!(cx_bn_export(s, s_bytes.as_mut_ptr(), s_bytes.len() as u32)).ok()?;

        s_bytes.reverse();

        // And copy the signature into the output.
        let mut buf = [0; 64];

        buf[..32].copy_from_slice(&self.r);

        buf[32..].copy_from_slice(&s_bytes);

        Ok(Ed25519Signature(buf))
    }
}
