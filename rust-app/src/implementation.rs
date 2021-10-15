use crate::crypto_helpers::{detecdsa_sign, get_pkh, get_private_key, get_pubkey, Hasher};
use crate::interface::*;
use arrayvec::{ArrayString, ArrayVec};
use core::fmt::Write;
use ledger_log::*;
use ledger_parser_combinators::interp_parser::{
    Action, DefaultInterp, DropInterp, InterpParser, ObserveLengthedBytes, SubInterp,
};
use ledger_parser_combinators::json::Json;
use nanos_ui::ui;
use nanos_sdk::pic_rs;

use ledger_parser_combinators::define_json_struct_interp;
use ledger_parser_combinators::json::*;
use ledger_parser_combinators::json_interp::*;

pub type GetAddressImplT =
    Action<SubInterp<DefaultInterp>, fn(&ArrayVec<u32, 10>) -> Option<ArrayVec<u8, 260>>>;

pub const GET_ADDRESS_IMPL: GetAddressImplT =
    Action(SubInterp(DefaultInterp), |path: &ArrayVec<u32, 10>| {
        let key = get_pubkey(path).ok()?;
        let mut rv = ArrayVec::<u8, 260>::new();
        rv.try_extend_from_slice(&[(key.W.len() as u8)][..]).ok()?;
        rv.try_extend_from_slice(&key.W[..]).ok()?;

        // At this point we have the value to send to the host; but there's a bit more to do to
        // ask permission from the user.

        let pkh = get_pkh(key);

        let mut pmpt = ArrayString::<128>::new();
        write!(pmpt, "{}", pkh).ok()?;

        if !ui::MessageValidator::new(&["Provide Public Key", &pmpt], &[&"Confirm"], &[]).ask() {
            trace!("User rejected\n");
            None
        } else {
            Some(rv)
        }
    });

pub type SignImplT = Action<
    (
        Action<
            ObserveLengthedBytes<
                Hasher,
                fn(&mut Hasher, &[u8]),
                Json<
                    KadenaCmd<
                        DropInterp,
                        DropInterp,
                        SubInterp<
                            Signer<
                                DropInterp,
                                DropInterp,
                                DropInterp,
                                SubInterp<
                                    Action<
                                        JsonStringAccumulate<64>,
                                        fn(&ArrayVec<u8, 64>) -> Option<()>,
                                    >,
                                >,
                            >,
                        >,
                        DropInterp,
                        DropInterp,
                    >,
                >,
            >,
            fn(
                &(
                    Result<
                        KadenaCmd<Option<()>, Option<()>, Option<()>, Option<()>, Option<()>>,
                        (),
                    >,
                    Hasher,
                ),
            ) -> Option<[u8; 32]>,
        >,
        Action<
            SubInterp<DefaultInterp>,
            fn(&ArrayVec<u32, 10>) -> Option<nanos_sdk::bindings::cx_ecfp_private_key_t>,
        >,
    ),
    fn(&([u8; 32], nanos_sdk::bindings::cx_ecfp_private_key_t)) -> Option<ArrayVec<u8, 260>>,
>;

pub const SIGN_IMPL: SignImplT = Action(
    (
        Action(
            // Calculate the hash of the transaction
            ObserveLengthedBytes(
                Hasher::new,
                Hasher::update,
                Json(KadenaCmd {
                    nonce: DropInterp,
                    meta: DropInterp,
                    signers: SubInterp(Signer {
                        scheme: DropInterp,
                        pub_key: DropInterp,
                        addr: DropInterp,
                        caps: SubInterp(Action(
                            JsonStringAccumulate,
                            |cap_str: &ArrayVec<u8, 64>| {
                                let pmpt = ArrayString::<128>::from(
                                    core::str::from_utf8(&cap_str[..]).ok()?,
                                )
                                .ok()?;
                                if !ui::MessageValidator::new(&["Transaction May", &pmpt], &[], &[])
                                    .ask()
                                {
                                    None
                                } else {
                                    Some(())
                                }
                            },
                        )),
                    }),
                    payload: DropInterp,
                    network_id: DropInterp,
                }),
            ),
            // Ask the user if they accept the transaction body's hash
            |(_, hash): &(_, Hasher)| {
                let the_hash = hash.clone().finalize();

                let mut pmpt = ArrayString::<128>::new();
                write!(pmpt, "{}", the_hash).ok()?;

                if !ui::MessageValidator::new(&["Sign Hash?", &pmpt], &[], &[]).ask() {
                    None
                } else {
                    Some(the_hash.0.into())
                }
            },
        ),
        Action(
            SubInterp(DefaultInterp),
            // And ask the user if this is the key the meant to sign with:
            |path: &ArrayVec<u32, 10>| {
                let privkey = get_private_key(path).ok()?;
                let pubkey = get_pubkey(path).ok()?; // Redoing work here; fix.
                let pkh = get_pkh(pubkey);

                let mut pmpt = ArrayString::<128>::new();
                write!(pmpt, "{}", pkh).ok()?;

                if !ui::MessageValidator::new(&["With PKH", &pmpt], &[], &[]).ask() {
                    None
                } else {
                    Some(privkey)
                }
            },
        ),
    ),
    |(hash, key): &([u8; 32], _)| {
        // By the time we get here, we've approved and just need to do the signature.
        let (sig, len) = detecdsa_sign(hash, key)?;
        let mut rv = ArrayVec::<u8, 260>::new();
        rv.try_extend_from_slice(&sig[0..len as usize]).ok()?;
        Some(rv)
    },
);

// The global parser state enum; any parser above that'll be used as the implementation for an APDU
// must have a field here.

pub enum ParsersState {
    NoState,
    GetAddressState(<GetAddressImplT as InterpParser<Bip32Key>>::State),
    SignState(<SignImplT as InterpParser<SignParameters>>::State),
}

define_json_struct_interp! { Meta 16 {
    chainId: JsonString,
    sender: JsonString,
    gasLimit: JsonNumber,
    gasPrice: JsonNumber,
    ttl: JsonNumber,
    creationTime: JsonNumber
}}
define_json_struct_interp! { Signer 16 {
    scheme: JsonString,
    pubKey: JsonString,
    addr: JsonString,
    caps: JsonArray<JsonString>
}}
define_json_struct_interp! { KadenaCmd 16 {
  nonce: JsonString,
  meta: MetaSchema,
  signers: JsonArray<SignerSchema>,
  payload: JsonAny,
  networkId: JsonAny
}}

pub fn get_get_address_state(
    s: &mut ParsersState,
) -> &mut <GetAddressImplT as InterpParser<Bip32Key>>::State {
    match s {
        ParsersState::GetAddressState(_) => {}
        _ => {
            trace!("Non-same state found; initializing state.");
            *s = ParsersState::GetAddressState(<GetAddressImplT as InterpParser<Bip32Key>>::init(
                &GET_ADDRESS_IMPL,
            ));
        }
    }
    match s {
        ParsersState::GetAddressState(ref mut a) => a,
        _ => {
            panic!("")
        }
    }
}

pub fn get_sign_state(
    s: &mut ParsersState,
) -> &mut <SignImplT as InterpParser<SignParameters>>::State {
    match s {
        ParsersState::SignState(_) => {}
        _ => {
            trace!("Non-same state found; initializing state.");
            *s = ParsersState::SignState(<SignImplT as InterpParser<SignParameters>>::init(
                &SIGN_IMPL,
            ));
        }
    }
    match s {
        ParsersState::SignState(ref mut a) => a,
        _ => {
            panic!("")
        }
    }
}
