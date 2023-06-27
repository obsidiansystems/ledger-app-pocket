use crate::crypto_helpers::PKH;
use crate::interface::*;
use crate::utils::*;
use crate::*;
use arrayvec::ArrayVec;
use core::fmt::Debug;
use core::fmt::Write;
use ledger_crypto_helpers::common::{try_option, Address, CryptographyError};
use ledger_crypto_helpers::ed25519::*;
use ledger_crypto_helpers::eddsa::{ed25519_public_key_bytes, with_public_keys};
use ledger_parser_combinators::interp_parser::{
    set_from_thunk, Action, DefaultInterp, DropInterp, DynBind, DynParser, InterpParser,
    MoveAction, ObserveLengthedBytes, ParseResult, ParserCommon, Preaction, SubInterp, OOB,
};
use ledger_parser_combinators::json::Json;
use ledger_prompts_ui::{final_accept_prompt, ScrollerError};

use core::str::from_utf8;

use core::convert::TryFrom;

use ledger_parser_combinators::define_json_struct_interp;
use ledger_parser_combinators::json::*;
use ledger_parser_combinators::json_interp::*;

use enum_init::InPlaceInit;

const fn mktfn<A, B, C, D>(
    q: fn(&A, &mut B, DynamicStackBox<D>) -> Option<C>,
) -> fn(&A, &mut B, DynamicStackBox<D>) -> Option<C> {
    q
}

pub type GetAddressImplT = impl InterpParser<Bip32Key, Returning = ArrayVec<u8, 128>>;

// Need a path of length 5, as make_bip32_path panics with smaller paths
pub const BIP32_PREFIX: [u32; 2] = nanos_sdk::ecc::make_bip32_path(b"m/44'/635'");

pub const fn get_address_impl<const PROMPT: bool>() -> GetAddressImplT {
    Action(
        SubInterp(DefaultInterp),
        mkfn(
            |path: &ArrayVec<u32, 10>, destination: &mut Option<ArrayVec<u8, 128>>| -> Option<()> {
                if !path.starts_with(&BIP32_PREFIX[0..2]) {
                    // There isn't a _no_throw variation of the below, so avoid a throw on incorrect input.
                    return None;
                }
                with_public_keys(path, false, |key: &_, pkh: &PKH| {
                    try_option(|| -> Option<()> {
                        if PROMPT {
                            scroller("Provide Public Key", |_w| Ok(()))?;
                            scroller_paginated("Address", |w| Ok(write!(w, "{pkh}")?))?;
                            final_accept_prompt(&[])?;
                        }

                        let rv = destination.insert(ArrayVec::new());

                        // Should return the format that the chain customarily uses for public keys; for
                        // ed25519 that's usually r | s with no prefix, which isn't quite our internal
                        // representation.
                        let key_bytes = ed25519_public_key_bytes(key);

                        rv.try_push(u8::try_from(key_bytes.len()).ok()?).ok()?;
                        rv.try_extend_from_slice(key_bytes).ok()?;

                        // And we'll send the address along; in our case it happens to be the same as the
                        // public key, but in general it's something computed from the public key.
                        let binary_address = pkh.get_binary_address();
                        rv.try_push(u8::try_from(binary_address.len()).ok()?).ok()?;
                        rv.try_extend_from_slice(binary_address).ok()?;
                        Some(())
                    }())
                })
                .ok()
            },
        ),
    )
}

//const fn show_address<const TITLE: &'static str>() -> impl JsonInterp<JsonString, State: Debug, Returning: Debug>
const fn show_address<const TITLE: &'static str>(
) -> Action<JsonStringAccumulate<64>, fn(&ArrayVec<u8, 64>, &mut Option<()>) -> Option<()>> {
    Action(
        JsonStringAccumulate::<64>,
        mkvfn(move |from_address: &ArrayVec<u8, 64>, destination| {
            scroller(TITLE, |w| {
                Ok(write!(w, "{}", from_utf8(from_address.as_slice())?)?)
            })?;
            *destination = Some(());
            Some(())
        }),
    )
}

/* This would be used to show fees; not currently used.
const AMOUNT_ACTION: Action<AmountType<JsonStringAccumulate<64>, JsonStringAccumulate<64>>,
                                  fn(& AmountType<Option<ArrayVec<u8, 64>>, Option<ArrayVec<u8, 64>>>, &mut Option<()>) -> Option<()>> =
  Action(AmountType{field_amount: JsonStringAccumulate::<64>, field_denom: JsonStringAccumulate::<64>},
        | AmountType{field_amount: amount, field_denom: denom}, destination | {
          scroller("Amount:", |w| Ok(write!(w, "{} ({})", from_utf8(amount.as_ref()?)?, from_utf8(denom.as_ref()?)?)?))?;
          *destination = Some(());
          Some(())
        });
*/

type SendMessageAction = impl JsonInterp<SendValueSchema, State: Debug, Returning = ()>;
const SEND_MESSAGE_ACTION: SendMessageAction = Preaction(
    || scroller("Transfer", |w| Ok(write!(w, "POKT")?)),
    Action(
        SendValueInterp {
            field_amount: JsonStringAccumulate::<64>,
            field_from_address: JsonStringAccumulate::<64>,
            field_to_address: JsonStringAccumulate::<64>,
        },
        mkfn(
            |o: &SendValue<
                Option<ArrayVec<u8, 64>>,
                Option<ArrayVec<u8, 64>>,
                Option<ArrayVec<u8, 64>>,
            >,
             destination: &mut Option<()>| {
                scroller_paginated("From", |w| {
                    Ok(write!(
                        w,
                        "{}",
                        from_utf8(o.field_from_address.as_ref().ok_or(ScrollerError)?)?
                    )?)
                })?;
                scroller_paginated("To", |w| {
                    Ok(write!(
                        w,
                        "{}",
                        from_utf8(o.field_to_address.as_ref().ok_or(ScrollerError)?)?
                    )?)
                })?;
                scroller("Amount", |w| {
                    let x = get_amount_in_decimals(o.field_amount.as_ref().ok_or(ScrollerError)?)
                        .map_err(|_| ScrollerError)?;
                    Ok(write!(w, "POKT {}", from_utf8(&x)?)?)
                })?;
                *destination = Some(());
                Some(())
            },
        ),
    ),
); // TO_ADDRESS_ACTION});

// "Divides" the amount by 1000000
// Converts the input string in the following manner
// 1 -> 0.000001
// 10 -> 0.00001
// 11 -> 0.000011
// 1000000 -> 1.0
// 10000000 -> 10.0
// 10010000 -> 10.01
// 010010000 -> 10.01
fn get_amount_in_decimals(amount: &ArrayVec<u8, 64>) -> Result<ArrayVec<u8, 64>, ()> {
    let mut found_first_non_zero = false;
    let mut start_ix = 0;
    let mut last_non_zero_ix = 0;
    // check the amount for any invalid chars and get its length
    for (ix, c) in amount.as_ref().iter().enumerate() {
        if !(&b'0'..=&b'9').contains(&c) {
            return Err(());
        }
        if c != &b'0' {
            last_non_zero_ix = ix;
        }
        if !found_first_non_zero {
            if c == &b'0' {
                // Highly unlikely to hit this, but skip any leading zeroes
                continue;
            }
            start_ix = ix;
            found_first_non_zero = true;
        }
    }

    let mut dec_value: ArrayVec<u8, 64> = ArrayVec::new();
    let amt_len = amount.len() - start_ix;
    let chars_after_decimal = 6;
    if amt_len > chars_after_decimal {
        // value is more than 1
        dec_value
            .try_extend_from_slice(&amount.as_ref()[start_ix..(amount.len() - chars_after_decimal)])
            .map_err(|_| ())?;
        dec_value.try_push(b'.').map_err(|_| ())?;
        if amount.len() - chars_after_decimal <= last_non_zero_ix {
            // there is non-zero decimal value
            dec_value
                .try_extend_from_slice(
                    &amount.as_ref()[amount.len() - chars_after_decimal..(last_non_zero_ix + 1)],
                )
                .map_err(|_| ())?;
        } else {
            // add a zero at the end always "xyz.0"
            dec_value.try_push(b'0').map_err(|_| ())?;
        }
    } else {
        // value is less than 1
        dec_value.try_push(b'0').map_err(|_| ())?;
        dec_value.try_push(b'.').map_err(|_| ())?;
        for _i in 0..(chars_after_decimal - amt_len) {
            dec_value.try_push(b'0').map_err(|_| ())?;
        }
        dec_value
            .try_extend_from_slice(&amount.as_ref()[start_ix..(last_non_zero_ix + 1)])
            .map_err(|_| ())?;
    }
    Ok(dec_value)
}

type StakeMessageAction = impl JsonInterp<StakeValueSchema, State: Debug>;
const STAKE_MESSAGE_ACTION: StakeMessageAction = Preaction(
    || scroller("Stake", |w| Ok(write!(w, "POKT")?)),
    Action(
        StakeValueInterp {
            field_chains: AccumulateArray(JsonStringAccumulate::<4>),
            field_public_key: PublicKeyInterp {
                field_type: JsonStringAccumulate::<64>,
                field_value: JsonStringAccumulate::<64>,
            },
            field_service_url: JsonStringAccumulate::<64>,
            field_value: JsonStringAccumulate::<64>,
            field_output_address: JsonStringAccumulate::<64>,
        },
        mkfn(
            |o: &StakeValue<
                Option<ArrayVec<ArrayVec<u8, 4>, 1>>,
                Option<PublicKey<Option<ArrayVec<u8, 64>>, Option<ArrayVec<u8, 64>>>>,
                Option<ArrayVec<u8, 64>>,
                Option<ArrayVec<u8, 64>>,
                Option<ArrayVec<u8, 64>>,
            >,
             destination: &mut Option<()>| {
                let chains = o.field_chains.as_ref()?.as_slice();
                if chains.len() != 1 {
                    return None;
                }
                unsafe {
                    scroller_paginated("From", |w| Ok(write!(w, "{}", SIGNING_ADDRESS)?))?;
                }
                scroller("Amount", |w| {
                    let x = get_amount_in_decimals(o.field_value.as_ref().ok_or(ScrollerError)?)
                        .map_err(|_| ScrollerError)?;
                    Ok(write!(w, "POKT {}", from_utf8(&x)?)?)
                })?;
                scroller_paginated("Public Key", |w| {
                    let x = o.field_public_key.as_ref().ok_or(ScrollerError)?;
                    Ok(write!(
                        w,
                        "{} ({})",
                        from_utf8(x.field_value.as_ref().ok_or(ScrollerError)?)?,
                        from_utf8(x.field_type.as_ref().ok_or(ScrollerError)?)?
                    )?)
                })?;
                scroller("Output Address", |w| {
                    Ok(write!(
                        w,
                        "{}",
                        from_utf8(o.field_output_address.as_ref().ok_or(ScrollerError)?)?
                    )?)
                })?;
                scroller("Service URL", |w| {
                    Ok(write!(
                        w,
                        "{}",
                        from_utf8(o.field_service_url.as_ref().ok_or(ScrollerError)?)?
                    )?)
                })?;
                scroller("Chain ID(s)", |w| {
                    Ok(write!(w, "{}", from_utf8(chains[0].as_ref())?)?)
                })?;
                *destination = Some(());
                Some(())
            },
        ),
    ),
);
type UnstakeMessageAction = impl JsonInterp<UnstakeValueSchema, State: Debug>;
const UNSTAKE_MESSAGE_ACTION: UnstakeMessageAction = Preaction(
    || scroller("Unstake", |w| Ok(write!(w, "POKT")?)),
    UnstakeValueInterp {
        field_validator_address: show_address::<"Unstake address">(),
        field_signer_address: SIGNER_ADDRESS_ACTION,
    },
);

type SignerAddressAction = impl JsonInterp<JsonString, State: Debug>;
const SIGNER_ADDRESS_ACTION: SignerAddressAction =
    Action(show_address::<"Signer address">(), mkvfn(|_, _| Some(())));

type UnjailMessageAction = impl JsonInterp<UnjailValueSchema, State: Debug>;
const UNJAIL_MESSAGE_ACTION: UnjailMessageAction = Preaction(
    || scroller("Unjail", |w| Ok(write!(w, "Transaction")?)),
    UnjailValueInterp {
        field_address: show_address::<"Address">(),
        field_signer_address: SIGNER_ADDRESS_ACTION,
    },
);

pub struct DynamicStackBoxSlot<S>(S, bool);
pub struct DynamicStackBox<S>(*mut DynamicStackBoxSlot<S>);

impl<S> DynamicStackBoxSlot<S> {
    fn new(s: S) -> DynamicStackBoxSlot<S> {
        DynamicStackBoxSlot(s, false)
    }
    fn to_box(&mut self) -> DynamicStackBox<S> {
        if self.1 {
            panic!();
        }
        self.1 = true;
        DynamicStackBox(self as *mut Self)
    }
}

impl<S> Default for DynamicStackBox<S> {
    fn default() -> Self {
        DynamicStackBox(core::ptr::null_mut())
    }
}

impl<S> Drop for DynamicStackBoxSlot<S> {
    fn drop(&mut self) {
        if self.1 {
            panic!("Some DynamicStackBox outlived it's backing storage.");
        }
    }
}

impl<S> core::ops::Deref for DynamicStackBox<S> {
    type Target = S;
    fn deref(&self) -> &Self::Target {
        unsafe {
            let target = self
                .0
                .as_ref()
                .expect("DynamicStackBox pointer must not be null");
            if !target.1 {
                panic!();
            }
            &target.0
        }
    }
}

impl<S> core::ops::DerefMut for DynamicStackBox<S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {
            let target = self
                .0
                .as_mut()
                .expect("DynamicStackBox pointer must not be null");
            if !target.1 {
                panic!();
            }
            &mut target.0
        }
    }
}

impl<S> Drop for DynamicStackBox<S> {
    fn drop(&mut self) {
        unsafe {
            let target = self
                .0
                .as_mut()
                .expect("DynamicStackBox pointer must not be null");
            if !target.1 {
                panic!();
            }
            target.1 = false;
        }
    }
}

pub struct WithStackBoxed<S>(S);

/*fn with_stack_boxed<T,S>(S) {
    WithStackBoxed(S, core::marker::PhantomData);
}*/

pub struct WithStackBoxedState<S, P>(S, DynamicStackBoxSlot<P>, bool);

impl<Q: Default, T, S: DynParser<T, Parameter = DynamicStackBox<Q>>> ParserCommon<T>
    for WithStackBoxed<S>
{
    type State = WithStackBoxedState<S::State, Q>;
    type Returning = S::Returning;
    fn init(&self) -> Self::State {
        WithStackBoxedState(self.0.init(), DynamicStackBoxSlot::new(Q::default()), false)
    }
}

impl<Q: Default, T, S: DynParser<T, Parameter = DynamicStackBox<Q>> + InterpParser<T>>
    InterpParser<T> for WithStackBoxed<S>
{
    fn parse<'a, 'b>(
        &self,
        state: &'b mut Self::State,
        chunk: &'a [u8],
        destination: &mut Option<Self::Returning>,
    ) -> ParseResult<'a> {
        if !state.2 {
            self.0
                .init_param(state.1.to_box(), &mut state.0, destination);
        }
        state.2 = true;
        self.0.parse(&mut state.0, chunk, destination)
    }
}

pub const SIGN_SEQ: [usize; 3] = [1, 0, 0];

enum SignTempError {
    ScrollerError(ScrollerError),
    CryptographyError(CryptographyError),
}

impl From<ScrollerError> for SignTempError {
    fn from(e: ScrollerError) -> Self {
        SignTempError::ScrollerError(e)
    }
}
impl From<CryptographyError> for SignTempError {
    fn from(e: CryptographyError) -> Self {
        SignTempError::CryptographyError(e)
    }
}

// Only one fees field is supported at the moment, so no string -> number conversion/summation required
#[derive(Clone, Debug)]
struct TotalFees(pub Option<ArrayVec<u8, 64>>);

impl Summable<TotalFees> for TotalFees {
    fn zero() -> Self {
        TotalFees(None)
    }
    fn add_and_set(&mut self, other: &TotalFees) {
        *self = TotalFees(other.0.clone());
    }
}

static mut SIGNING_ADDRESS: PKH = PKH([0; 20]);

pub type SignImplT = impl InterpParser<DoubledSignParameters, Returning = ArrayVec<u8, 128>>;

pub const SIGN_IMPL: SignImplT = WithStackBoxed(DynBind(
    Action(
        SubInterp(DefaultInterp),
        // And ask the user if this is the key the meant to sign with:
        mktfn(
            |path: &ArrayVec<u32, 10>, destination, mut ed: DynamicStackBox<Ed25519>| {
                with_public_keys(path, false, |_, pkh: &PKH| {
                    unsafe {
                        SIGNING_ADDRESS.0 = pkh.0.clone();
                    }
                    ed.init(path.clone())?;
                    // *destination = Some(ed);
                    set_from_thunk(destination, || Some(ed)); //  Ed25519::new(path).ok());
                    Ok::<_, SignTempError>(())
                })
                .ok()?;
                Some(())
            },
        ),
    ),
    DynBind(
        MoveAction(
            ObserveLengthedBytes(
                DynamicStackBox::<Ed25519>::default, // move || edward.clone(),
                |s: &mut DynamicStackBox<Ed25519>, b: &[u8]| s.update(b),
                Action(
                    Json(Action(
                        PoktCmdInterp {
                            field_chain_id: DropInterp,
                            field_entropy: DropInterp,
                            field_fee: SubInterpMFold::new(Action(
                                AmountTypeInterp {
                                    field_amount: JsonStringAccumulate::<64>,
                                    field_denom: JsonStringAccumulate::<64>,
                                },
                                mkfnc(
                                    |o: &AmountType<
                                        Option<ArrayVec<u8, 64>>,
                                        Option<ArrayVec<u8, 64>>,
                                    >,
                                     destination: &mut Option<TotalFees>,
                                     _| {
                                        *destination = Some(TotalFees(o.field_amount.clone()));
                                        Some(())
                                    },
                                ),
                            )),
                            field_memo: DropInterp,
                            field_msg: Message {
                                send_message: SEND_MESSAGE_ACTION,
                                unjail_message: UNJAIL_MESSAGE_ACTION,
                                stake_message: STAKE_MESSAGE_ACTION,
                                unstake_message: UNSTAKE_MESSAGE_ACTION,
                            },
                        },
                        mkfn(
                            |o: &PoktCmd<
                                Option<()>,
                                Option<()>,
                                Option<TotalFees>,
                                Option<()>,
                                Option<MessageReturnT>,
                            >,
                             ret: &mut Option<()>| {
                                if let Some(fee) = &o.field_fee {
                                    scroller("Fee", |w| {
                                        let x = get_amount_in_decimals(
                                            fee.0.as_ref().ok_or(ScrollerError)?,
                                        )
                                        .map_err(|_| ScrollerError)?;
                                        Ok(write!(w, "POKT {}", from_utf8(&x)?)?)
                                    })?;
                                }
                                *ret = Some(());
                                Some(())
                            },
                        ),
                    )),
                    mkvfn(|_, ret| {
                        *ret = Some(());
                        Some(())
                    }),
                ),
                true,
            ),
            mkmvfn(
                |(_, initial_edward): (Option<()>, DynamicStackBox<Ed25519>),
                 destination: &mut Option<DynamicStackBox<Ed25519>>|
                 -> Option<()> {
                    *destination = Some(initial_edward);
                    destination.as_mut()?.done_with_r().ok()?;
                    Some(())
                },
            ),
        ),
        MoveAction(
            ObserveLengthedBytes(
                DynamicStackBox::<Ed25519>::default, // move || edward.clone(),
                |s: &mut DynamicStackBox<Ed25519>, b: &[u8]| s.update(b),
                /*  || Ed25519::default(), // move || edward.clone(),
                Ed25519::update,*/
                Json(DropInterp),
                true,
            ),
            mkmvfn(
                |(_, mut final_edward): (_, DynamicStackBox<Ed25519>),
                 destination: &mut Option<ArrayVec<u8, 128>>| {
                    final_accept_prompt(&["Sign Transaction?"])?;
                    // let mut final_edward_copy = final_edward.clone();
                    let sig = final_edward.finalize();
                    *destination = Some(ArrayVec::new());
                    destination
                        .as_mut()?
                        .try_extend_from_slice(&sig.ok()?.0)
                        .ok()?;
                    Some(())
                },
            ),
        ),
    ),
));

pub type BlindSignImplT =
    impl InterpParser<DoubledBlindSignParameters, Returning = ArrayVec<u8, 128_usize>>;

pub static BLIND_SIGN_IMPL: BlindSignImplT = Preaction(
    || -> Option<()> {
        scroller("WARNING", |w| {
            Ok(write!(w, "Blind Signing a Transaction is a very unusual operation. Do not continue unless you know what you are doing")?)
        })
    },
    WithStackBoxed(DynBind(
        Action(
            SubInterp(DefaultInterp),
            // And ask the user if this is the key the meant to sign with:
            mktfn(
                |path: &ArrayVec<u32, 10>, destination, mut ed: DynamicStackBox<Ed25519>| {
                    with_public_keys(path, false, |_, pkh: &PKH| {
                        ed.init(path.clone())?;
                        try_option(|| -> Option<()> {
                            scroller("Sign for Address", |w| Ok(write!(w, "{pkh}")?))?;
                            Some(())
                        }())?;
                        // *destination = Some(ed);
                        set_from_thunk(destination, || Some(ed)); //  Ed25519::new(path).ok());
                        Ok::<_, SignTempError>(())
                    })
                    .ok()?;
                    Some(())
                },
            ),
        ),
        DynBind(
            MoveAction(
                ObserveLengthedBytes(
                    DynamicStackBox::<Ed25519>::default, // move || edward.clone(),
                    |s: &mut DynamicStackBox<Ed25519>, b: &[u8]| s.update(b),
                    Json(DropInterp),
                    true,
                ),
                mkmvfn(
                    |(_, initial_edward): (Option<()>, DynamicStackBox<Ed25519>),
                     destination: &mut Option<DynamicStackBox<Ed25519>>|
                     -> Option<()> {
                        *destination = Some(initial_edward);
                        destination.as_mut()?.done_with_r().ok()?;
                        Some(())
                    },
                ),
            ),
            MoveAction(
                ObserveLengthedBytes(
                    DynamicStackBox::<Ed25519>::default, // move || edward.clone(),
                    |s: &mut DynamicStackBox<Ed25519>, b: &[u8]| s.update(b),
                    /*  || Ed25519::default(), // move || edward.clone(),
                    Ed25519::update,*/
                    Json(DropInterp),
                    true,
                ),
                mkmvfn(
                    |(_, mut final_edward): (_, DynamicStackBox<Ed25519>),
                     destination: &mut Option<ArrayVec<u8, 128>>| {
                        final_accept_prompt(&["Blind Sign Transaction?"])?;
                        // let mut final_edward_copy = final_edward.clone();
                        let sig = final_edward.finalize();
                        *destination = Some(ArrayVec::new());
                        destination
                            .as_mut()?
                            .try_extend_from_slice(&sig.ok()?.0)
                            .ok()?;
                        Some(())
                    },
                ),
            ),
        ),
    )),
);

// The global parser state enum; any parser above that'll be used as the implementation for an APDU
// must have a field here.

#[derive(InPlaceInit)]
#[repr(u8)]
pub enum ParsersStateInner<A, B, C> {
    NoState,
    GetAddressState(A),
    SignState(B),
    BlindSignState(C),
    /*GetAddressState(<GetAddressImplT as ParserCommon<Bip32Key>>::State),
    SignState(<SignImplT as ParserCommon<DoubledSignParameters>>::State),*/
}

pub type ParsersState = ParsersStateInner<
    <GetAddressImplT as ParserCommon<Bip32Key>>::State,
    <SignImplT as ParserCommon<DoubledSignParameters>>::State,
    <BlindSignImplT as ParserCommon<DoubledBlindSignParameters>>::State,
>;

pub fn reset_parsers_state(state: &mut ParsersState) {
    *state = ParsersState::NoState;
}

/*
pub fn not_a_real_fn() {
    trace!("foo: {:?}",ParsersState_internal::ParsersStateTag::GetAddressState);
}
*/

meta_definition! {}
signer_definition! {}
amount_type_definition! {}
fee_definition! {}
send_value_definition! {}
unjail_value_definition! {}
public_key_definition! {}
stake_value_definition! {}
unstake_value_definition! {}

#[derive(Copy, Clone, Debug)]
pub enum MessageType {
    SendMessage,
    UnjailMessage,
    StakeMessage,
    UnstakeMessage,
}

#[derive(Debug)]
pub struct Message<
    SendInterp: JsonInterp<SendValueSchema>,
    UnjailInterp: JsonInterp<UnjailValueSchema>,
    StakeInterp: JsonInterp<StakeValueSchema>,
    UnstakeInterp: JsonInterp<UnstakeValueSchema>,
> {
    pub send_message: SendInterp,
    pub unjail_message: UnjailInterp,
    pub stake_message: StakeInterp,
    pub unstake_message: UnstakeInterp,
}

type TemporaryStringState<const N: usize> =
    <JsonStringAccumulate<N> as ParserCommon<JsonString>>::State;
type TemporaryStringReturn<const N: usize> =
    Option<<JsonStringAccumulate<N> as ParserCommon<JsonString>>::Returning>;

#[derive(Debug)]
pub enum MessageState<SendMessageState, UnjailMessageState, StakeMessageState, UnstakeMessageState>
{
    Start,
    TypeLabel(TemporaryStringState<4>, TemporaryStringReturn<4>),
    KeySep1,
    Type(TemporaryStringState<64>, TemporaryStringReturn<64>),
    ValueSep(MessageType),
    ValueLabel(
        MessageType,
        TemporaryStringState<5>,
        TemporaryStringReturn<5>,
    ),
    KeySep2(MessageType),
    SendMessageState(SendMessageState),
    UnjailMessageState(UnjailMessageState),
    StakeMessageState(StakeMessageState),
    UnstakeMessageState(UnstakeMessageState),
    End,
}

fn init_str<const N: usize>() -> <JsonStringAccumulate<N> as ParserCommon<JsonString>>::State {
    <JsonStringAccumulate<N> as ParserCommon<JsonString>>::init(&JsonStringAccumulate)
}
fn call_str<'a, const N: usize>(
    ss: &mut <JsonStringAccumulate<N> as ParserCommon<JsonString>>::State,
    token: JsonToken<'a>,
    dest: &mut Option<<JsonStringAccumulate<N> as ParserCommon<JsonString>>::Returning>,
) -> Result<(), Option<OOB>> {
    <JsonStringAccumulate<N> as JsonInterp<JsonString>>::parse(
        &JsonStringAccumulate,
        ss,
        token,
        dest,
    )
}

pub enum MessageReturn<
    SendMessageReturn,
    UnjailMessageReturn,
    StakeMessageReturn,
    UnstakeMessageReturn,
> {
    SendMessageReturn(Option<SendMessageReturn>),
    UnjailMessageReturn(Option<UnjailMessageReturn>),
    StakeMessageReturn(Option<StakeMessageReturn>),
    UnstakeMessageReturn(Option<UnstakeMessageReturn>),
}
type MessageReturnT = MessageReturn<
    <SendMessageAction as ParserCommon<SendValueSchema>>::Returning,
    <UnjailMessageAction as ParserCommon<UnjailValueSchema>>::Returning,
    <StakeMessageAction as ParserCommon<StakeValueSchema>>::Returning,
    <UnstakeMessageAction as ParserCommon<UnstakeValueSchema>>::Returning,
>;

impl ParserCommon<MessageSchema> for DropInterp {
    type State = <DropInterp as ParserCommon<JsonAny>>::State;
    type Returning = <DropInterp as ParserCommon<JsonAny>>::Returning;
    fn init(&self) -> Self::State {
        <DropInterp as ParserCommon<JsonAny>>::init(&DropInterp)
    }
}

impl JsonInterp<MessageSchema> for DropInterp {
    fn parse<'a>(
        &self,
        state: &mut Self::State,
        token: JsonToken<'a>,
        destination: &mut Option<Self::Returning>,
    ) -> Result<(), Option<OOB>> {
        <DropInterp as JsonInterp<JsonAny>>::parse(&DropInterp, state, token, destination)
    }
}

impl<
        SendInterp: JsonInterp<SendValueSchema>,
        UnjailInterp: JsonInterp<UnjailValueSchema>,
        StakeInterp: JsonInterp<StakeValueSchema>,
        UnstakeInterp: JsonInterp<UnstakeValueSchema>,
    > ParserCommon<MessageSchema> for Message<SendInterp, UnjailInterp, StakeInterp, UnstakeInterp>
where
    <SendInterp as ParserCommon<SendValueSchema>>::State: core::fmt::Debug,
    <UnjailInterp as ParserCommon<UnjailValueSchema>>::State: core::fmt::Debug,
    <StakeInterp as ParserCommon<StakeValueSchema>>::State: core::fmt::Debug,
    <UnstakeInterp as ParserCommon<UnstakeValueSchema>>::State: core::fmt::Debug,
{
    type State = MessageState<
        <SendInterp as ParserCommon<SendValueSchema>>::State,
        <UnjailInterp as ParserCommon<UnjailValueSchema>>::State,
        <StakeInterp as ParserCommon<StakeValueSchema>>::State,
        <UnstakeInterp as ParserCommon<UnstakeValueSchema>>::State,
    >;
    type Returning = MessageReturn<
        <SendInterp as ParserCommon<SendValueSchema>>::Returning,
        <UnjailInterp as ParserCommon<UnjailValueSchema>>::Returning,
        <StakeInterp as ParserCommon<StakeValueSchema>>::Returning,
        <UnstakeInterp as ParserCommon<UnstakeValueSchema>>::Returning,
    >;
    fn init(&self) -> Self::State {
        MessageState::Start
    }
}

impl<
        SendInterp: JsonInterp<SendValueSchema>,
        UnjailInterp: JsonInterp<UnjailValueSchema>,
        StakeInterp: JsonInterp<StakeValueSchema>,
        UnstakeInterp: JsonInterp<UnstakeValueSchema>,
    > JsonInterp<MessageSchema> for Message<SendInterp, UnjailInterp, StakeInterp, UnstakeInterp>
where
    <SendInterp as ParserCommon<SendValueSchema>>::State: core::fmt::Debug,
    <UnjailInterp as ParserCommon<UnjailValueSchema>>::State: core::fmt::Debug,
    <StakeInterp as ParserCommon<StakeValueSchema>>::State: core::fmt::Debug,
    <UnstakeInterp as ParserCommon<UnstakeValueSchema>>::State: core::fmt::Debug,
{
    #[inline(never)]
    fn parse<'a>(
        &self,
        state: &mut Self::State,
        token: JsonToken<'a>,
        destination: &mut Option<Self::Returning>,
    ) -> Result<(), Option<OOB>> {
        match state {
            MessageState::Start if token == JsonToken::BeginObject => {
                set_from_thunk(state, || MessageState::TypeLabel(init_str::<4>(), None));
            }
            MessageState::TypeLabel(ref mut temp_string_state, ref mut temp_string_return) => {
                call_str::<4>(temp_string_state, token, temp_string_return)?;
                if temp_string_return
                    .as_ref()
                    .expect("should be set by now")
                    .as_slice()
                    == b"type"
                {
                    set_from_thunk(state, || MessageState::KeySep1);
                } else {
                    return Err(Some(OOB::Reject));
                }
            }
            MessageState::KeySep1 if token == JsonToken::NameSeparator => {
                set_from_thunk(state, || MessageState::Type(init_str::<64>(), None));
            }
            MessageState::Type(ref mut temp_string_state, ref mut temp_string_return) => {
                call_str::<64>(temp_string_state, token, temp_string_return)?;
                match temp_string_return
                    .as_ref()
                    .expect("should be set by now")
                    .as_slice()
                {
                    b"pos/Send" => {
                        set_from_thunk(state, || MessageState::ValueSep(MessageType::SendMessage));
                    }
                    b"pos/8.0MsgUnjail" => {
                        set_from_thunk(state, || {
                            MessageState::ValueSep(MessageType::UnjailMessage)
                        });
                    }
                    b"pos/8.0MsgStake" => {
                        set_from_thunk(state, || MessageState::ValueSep(MessageType::StakeMessage));
                    }
                    b"pos/8.0MsgBeginUnstake" => {
                        set_from_thunk(state, || {
                            MessageState::ValueSep(MessageType::UnstakeMessage)
                        });
                    }
                    _ => return Err(Some(OOB::Reject)),
                }
            }
            MessageState::ValueSep(msg_type) if token == JsonToken::ValueSeparator => {
                let new_msg_type = *msg_type;
                set_from_thunk(state, || {
                    MessageState::ValueLabel(new_msg_type, init_str::<5>(), None)
                });
            }
            MessageState::ValueLabel(msg_type, temp_string_state, temp_string_return) => {
                call_str::<5>(temp_string_state, token, temp_string_return)?;
                if temp_string_return
                    .as_ref()
                    .expect("should be set by now")
                    .as_slice()
                    == b"value"
                {
                    let new_msg_type = *msg_type;
                    set_from_thunk(state, || MessageState::KeySep2(new_msg_type));
                } else {
                    return Err(Some(OOB::Reject));
                }
            }
            MessageState::KeySep2(msg_type) if token == JsonToken::NameSeparator => {
                match msg_type {
                    MessageType::SendMessage => {
                        *destination = Some(MessageReturn::SendMessageReturn(None));
                        set_from_thunk(state, || {
                            MessageState::SendMessageState(self.send_message.init())
                        });
                    }
                    MessageType::UnjailMessage => {
                        *destination = Some(MessageReturn::UnjailMessageReturn(None));
                        set_from_thunk(state, || {
                            MessageState::UnjailMessageState(self.unjail_message.init())
                        });
                    }
                    MessageType::StakeMessage => {
                        *destination = Some(MessageReturn::StakeMessageReturn(None));
                        set_from_thunk(state, || {
                            MessageState::StakeMessageState(self.stake_message.init())
                        });
                    }
                    MessageType::UnstakeMessage => {
                        *destination = Some(MessageReturn::UnstakeMessageReturn(None));
                        set_from_thunk(state, || {
                            MessageState::UnstakeMessageState(self.unstake_message.init())
                        });
                    }
                }
            }
            MessageState::SendMessageState(ref mut send_message_state) => {
                let sub_destination = &mut destination.as_mut().ok_or(Some(OOB::Reject))?;
                match sub_destination {
                    MessageReturn::SendMessageReturn(send_message_return) => {
                        self.send_message
                            .parse(send_message_state, token, send_message_return)?;
                        set_from_thunk(state, || MessageState::End);
                    }
                    _ => return Err(Some(OOB::Reject)),
                }
            }
            MessageState::UnjailMessageState(ref mut unjail_message_state) => {
                let sub_destination = &mut destination.as_mut().ok_or(Some(OOB::Reject))?;
                match sub_destination {
                    MessageReturn::UnjailMessageReturn(unjail_message_return) => {
                        self.unjail_message.parse(
                            unjail_message_state,
                            token,
                            unjail_message_return,
                        )?;
                        set_from_thunk(state, || MessageState::End);
                    }
                    _ => return Err(Some(OOB::Reject)),
                }
            }
            MessageState::StakeMessageState(ref mut stake_message_state) => {
                let sub_destination = &mut destination.as_mut().ok_or(Some(OOB::Reject))?;
                match sub_destination {
                    MessageReturn::StakeMessageReturn(stake_message_return) => {
                        self.stake_message.parse(
                            stake_message_state,
                            token,
                            stake_message_return,
                        )?;
                        set_from_thunk(state, || MessageState::End);
                    }
                    _ => return Err(Some(OOB::Reject)),
                }
            }
            MessageState::UnstakeMessageState(ref mut unstake_message_state) => {
                let sub_destination = &mut destination.as_mut().ok_or(Some(OOB::Reject))?;
                match sub_destination {
                    MessageReturn::UnstakeMessageReturn(unstake_message_return) => {
                        self.unstake_message.parse(
                            unstake_message_state,
                            token,
                            unstake_message_return,
                        )?;
                        set_from_thunk(state, || MessageState::End);
                    }
                    _ => return Err(Some(OOB::Reject)),
                }
            }
            MessageState::End if token == JsonToken::EndObject => return Ok(()),
            _ => return Err(Some(OOB::Reject)),
        };
        Err(None)
    }
}

pokt_cmd_definition! {}

#[inline(never)]
pub fn get_get_address_state<const PROMPT: bool>(
    s: &mut ParsersState,
) -> &mut <GetAddressImplT as ParserCommon<Bip32Key>>::State {
    match s {
        ParsersState::GetAddressState(_) => {}
        _ => {
            trace!("Non-same state found; initializing state.");
            *s = ParsersState::GetAddressState(<GetAddressImplT as ParserCommon<Bip32Key>>::init(
                &get_address_impl::<PROMPT>(),
            ));
        }
    }
    match s {
        ParsersState::GetAddressState(ref mut a) => a,
        _ => {
            unreachable!("Should be impossible because assignment right above")
        }
    }
}

#[inline(never)]
pub fn get_sign_state(
    s: &mut ParsersState,
) -> &mut <SignImplT as ParserCommon<DoubledSignParameters>>::State {
    match s {
        ParsersState::SignState(_) => {}
        _ => {
            trace!("Non-same state found; initializing state.");
            *s = ParsersState::SignState(<SignImplT as ParserCommon<DoubledSignParameters>>::init(
                &SIGN_IMPL,
            ));
        }
    }
    match s {
        ParsersState::SignState(ref mut a) => a,
        _ => {
            unreachable!("Should be impossible because assignment right above")
        }
    }
}

#[inline(never)]
pub fn get_blind_sign_state(
    s: &mut ParsersState,
) -> &mut <BlindSignImplT as ParserCommon<DoubledBlindSignParameters>>::State {
    match s {
        ParsersState::BlindSignState(_) => {}
        _ => {
            trace!("Non-same state found; initializing state.");
            *s = ParsersState::BlindSignState(<BlindSignImplT as ParserCommon<
                DoubledBlindSignParameters,
            >>::init(&BLIND_SIGN_IMPL));
        }
    }
    match s {
        ParsersState::BlindSignState(ref mut a) => a,
        _ => {
            unreachable!("Should be impossible because assignment right above")
        }
    }
}
