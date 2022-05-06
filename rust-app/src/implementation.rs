use crate::crypto_helpers::{with_public_keys, Ed25519, public_key_bytes};
use crate::interface::*;
use crate::*;
use arrayvec::ArrayVec;
use core::fmt::Write;
use core::fmt::Debug;
use ledger_parser_combinators::interp_parser::{
    Action, MoveAction, DynBind, DefaultInterp, DropInterp, InterpParser, DynInterpParser, ObserveLengthedBytes, SubInterp, OOB, set_from_thunk, ParseResult
};
use ledger_parser_combinators::json::Json;
use prompts_ui::{write_scroller, final_accept_prompt};
use core::str::from_utf8;
use core::convert::TryFrom;

use ledger_parser_combinators::define_json_struct_interp;
use ledger_parser_combinators::json::*;
use ledger_parser_combinators::json_interp::*;

use enum_init::InPlaceInit;

// A couple type ascription functions to help the compiler along.
const fn mkfn<A,B,C>(q: fn(&A,&mut B)->Option<C>) -> fn(&A,&mut B)->Option<C> {
  q
}
const fn mkmvfn<A,B,C>(q: fn(A,&mut B)->Option<C>) -> fn(A,&mut B)->Option<C> {
  q
}
const fn mktfn<A,B,C, D>(q: fn(&A,&mut B, DynamicStackBox<D>)->Option<C>) -> fn(&A,&mut B, DynamicStackBox<D>)->Option<C> {
  q
}
const fn mkvfn<A,C>(q: fn(&A,&mut Option<()>)->C) -> fn(&A,&mut Option<()>)->C {
  q
}
/*const fn mkbindfn<A,C>(q: fn(&A)->C) -> fn(&A)->C {
  q
}*/

pub type GetAddressImplT = impl InterpParser<Bip32Key, Returning = ArrayVec<u8, 128>>;

pub const GET_ADDRESS_IMPL: GetAddressImplT =
    Action(SubInterp(DefaultInterp), mkfn(|path: &ArrayVec<u32, 10>, destination: &mut Option<ArrayVec<u8, 128>>| -> Option<()> {
        with_public_keys(path, |key: &_, pkh: &_| {

        write_scroller("Provide Public Key", |w| Ok(write!(w, "For Address     {}", pkh)?))?;

        final_accept_prompt(&[])?;

        let key_bytes = public_key_bytes(key);
        let rv = destination.insert(ArrayVec::new());
        rv.try_push(u8::try_from(key_bytes.len()).ok()?).ok()?;
        rv.try_extend_from_slice(key_bytes).ok()?;
        rv.try_push(u8::try_from(pkh.0.len()).ok()?).ok()?;
        rv.try_extend_from_slice(&pkh.0).ok()?;
        Ok(())
        }).ok()
    }));

const FROM_ADDRESS_ACTION: impl JsonInterp<JsonString, State: Debug> = //Action<JsonStringAccumulate<64>,
                                  //fn(& ArrayVec<u8, 64>, &mut Option<()>) -> Option<()>> =
  Action(JsonStringAccumulate::<64>,
        mkvfn(| from_address: &ArrayVec<u8, 64>, destination | {
          write_scroller("Transfer from", |w| Ok(write!(w, "{}", from_utf8(from_address.as_slice())?)?))?;
          *destination = Some(());
          Some(())
        }));

const TO_ADDRESS_ACTION: Action<JsonStringAccumulate<64>,
                                  fn(& ArrayVec<u8, 64>, &mut Option<()>) -> Option<()>> =
  Action(JsonStringAccumulate::<64>,
        | to_address, destination | {
            write_scroller("Transfer To", |w| Ok(write!(w, "{}", from_utf8(to_address.as_slice())?)?))?;
            *destination = Some(());
            Some(())
        });

/* This would be used to show fees; not currently used.
const AMOUNT_ACTION: Action<AmountType<JsonStringAccumulate<64>, JsonStringAccumulate<64>>,
                                  fn(& AmountType<Option<ArrayVec<u8, 64>>, Option<ArrayVec<u8, 64>>>, &mut Option<()>) -> Option<()>> =
  Action(AmountType{field_amount: JsonStringAccumulate::<64>, field_denom: JsonStringAccumulate::<64>},
        | AmountType{field_amount: amount, field_denom: denom}, destination | {
          write_scroller("Amount:", |w| Ok(write!(w, "{} ({})", from_utf8(amount.as_ref()?)?, from_utf8(denom.as_ref()?)?)?))?;
          *destination = Some(());
          Some(())
        });
*/

const SEND_MESSAGE_ACTION: impl JsonInterp<SendValueSchema, State: Debug> =
  Preaction(|| { write_scroller("Send", |w| Ok(write!(w, "Transaction")?)) },
  SendValueInterp{field_amount: VALUE_ACTION,
            field_from_address: FROM_ADDRESS_ACTION,
            field_to_address: TO_ADDRESS_ACTION});

const CHAIN_ACTION: Action<JsonStringAccumulate<64>,
                           fn(& ArrayVec<u8, 64>, &mut Option<()>) -> Option<()>> =
  Action(JsonStringAccumulate::<64>,
        | chain, destination | {
          write_scroller("Chain", |w| Ok(write!(w, "{}", from_utf8(chain.as_ref())?)?))?;
          *destination = Some(());
          Some(())
        });

const VALUE_ACTION: Action<JsonStringAccumulate<64>,
                                 fn(& ArrayVec<u8, 64>, &mut Option<()>) -> Option<()>> =
  Action(JsonStringAccumulate::<64>,
        | value, destination | {
          write_scroller("Value", |w| Ok(write!(w, "{}", from_utf8(value.as_ref())?)?))?;
          *destination = Some(());
          Some(())
        });

const PUBLICKEY_ACTION: impl JsonInterp<PublicKeySchema, State: Debug> =
  Action(PublicKeyInterp {
    field_type: JsonStringAccumulate::<64>,
    field_value:JsonStringAccumulate::<64>},
        mkfn(| PublicKey{field_type: ty, field_value: val}: &PublicKey<Option<ArrayVec<u8, 64>>, Option<ArrayVec<u8, 64>>>, destination | {
            write_scroller("Public Key", |w| Ok(write!(w, "{} ({})", from_utf8(val.as_ref()?)?, from_utf8(ty.as_ref()?)?)?))?;
            *destination = Some(());
            Some(())
        }));

const SERVICE_URL_ACTION: Action<JsonStringAccumulate<64>,
                                 fn(& ArrayVec<u8, 64>, &mut Option<()>) -> Option<()>> =
  Action(JsonStringAccumulate::<64>,
        | service_url, destination | {
          write_scroller("Service URL", |w| Ok(write!(w, "{}", from_utf8(service_url)?)?))?;
          *destination = Some(());
          Some(())
        });

const STAKE_MESSAGE_ACTION: impl JsonInterp<StakeValueSchema, State: Debug> =
  Preaction(|| { write_scroller("Stake", |w| Ok(write!(w, "Transaction")?)) }, StakeValueInterp{
    field_chains: SubInterp(CHAIN_ACTION),
    field_public_key: PUBLICKEY_ACTION,
    field_service_url: SERVICE_URL_ACTION,
    field_value: VALUE_ACTION});

const UNSTAKE_MESSAGE_ACTION: impl JsonInterp<UnstakeValueSchema, State: Debug> =
  Preaction(|| { write_scroller("Unstake", |w| Ok(write!(w, "Transaction")?)) },
  UnstakeValueInterp{field_validator_address: FROM_ADDRESS_ACTION});


pub struct DynamicStackBoxSlot<S>(S, bool);
pub struct DynamicStackBox<S>(*mut DynamicStackBoxSlot<S>);

impl<S> DynamicStackBoxSlot<S> {
    fn new(s: S) -> DynamicStackBoxSlot<S> {
        DynamicStackBoxSlot(s, false)
    }
    fn to_box(&mut self) -> DynamicStackBox<S> {
        if self.1 { panic!(); }
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
        if self.1 { panic!("Some DynamicStackBox outlived it's backing storage."); }
    }
}

impl<S> core::ops::Deref for DynamicStackBox<S> {
    type Target = S;
    fn deref(&self) -> &Self::Target {
        unsafe {
            let target = self.0.as_ref().expect("DynamicStackBox pointer must not be null");
            if !target.1 { panic!(); }
            &target.0
        }
    }
}

impl<S> core::ops::DerefMut for DynamicStackBox<S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {
            let target = self.0.as_mut().expect("DynamicStackBox pointer must not be null");
            if !target.1 { panic!(); }
            &mut target.0
        }
    }
}

impl<S> Drop for DynamicStackBox<S> {
    fn drop(&mut self) {
        unsafe {
            let target = self.0.as_mut().expect("DynamicStackBox pointer must not be null");
            if !target.1 { panic!(); }
            target.1 = false;
        }
    }
}

pub struct WithStackBoxed<S>(S);

/*fn with_stack_boxed<T,S>(S) {
    WithStackBoxed(S, core::marker::PhantomData);
}*/

pub struct WithStackBoxedState<S, P>(S, DynamicStackBoxSlot<P>, bool);

impl<Q: Default, T, S: DynInterpParser<T, Parameter = DynamicStackBox<Q>>> InterpParser<T> for WithStackBoxed<S> {
    type State = WithStackBoxedState<S::State, Q>;
    type Returning = S::Returning;
    fn init(&self) -> Self::State {
        let rv = WithStackBoxedState(self.0.init(), DynamicStackBoxSlot::new(Q::default()), false);
        rv
    }
    fn parse<'a, 'b>(&self, state: &'b mut Self::State, chunk: &'a [u8], destination: &mut Option<Self::Returning>) -> ParseResult<'a> {
        if ! state.2 {
            self.0.init_param(state.1.to_box(), &mut state.0, destination);
        }
        state.2 = true;
        self.0.parse(&mut state.0, chunk, destination)
    }
}

pub type SignImplT = impl InterpParser<DoubledSignParameters, Returning = ArrayVec<u8,128>>;

pub const SIGN_SEQ: [usize; 3] = [1, 0, 0];

pub const SIGN_IMPL: SignImplT =
    WithStackBoxed(DynBind (
      Action(
          SubInterp(DefaultInterp),
          // And ask the user if this is the key the meant to sign with:
          mktfn(|path: &ArrayVec<u32, 10>, destination, mut ed: DynamicStackBox<Ed25519>| {
              write_scroller("Signing", |w| Ok(write!(w, "Transaction")?))?;
              with_public_keys(path, |_, pkh| {
                  write_scroller("For Account", |w| Ok(write!(w, "{}", pkh)?))?;
                  ed.init(path).ok()?;
                  // *destination = Some(ed);
                  set_from_thunk(destination, || Some(ed)); //  Ed25519::new(path).ok());
                  Ok(())
              }).ok()?;
              Some(())
          }),
      ),
            DynBind (
              MoveAction(ObserveLengthedBytes(
                || DynamicStackBox::<Ed25519>::default(), // move || edward.clone(),
                |s : &mut DynamicStackBox<Ed25519>, b: &[u8]| s.update(b),
                Action(
                  Json(PoktCmdInterp {
                      field_chain_id: DropInterp,
                      field_entropy: DropInterp,
                      field_fee: DropInterp,
                      field_memo: DropInterp,
                      field_msg: Message {send_message: SEND_MESSAGE_ACTION,
                                          unjail_message: DropInterp,
                                          stake_message: STAKE_MESSAGE_ACTION,
                                          unstake_message: UNSTAKE_MESSAGE_ACTION},
                  }),
                  mkvfn(| _, ret | {
                    *ret = Some(());
                    Some(())
                  })
                ),
                true),
                mkmvfn(| (_, initial_edward) : (Option<()>, DynamicStackBox<Ed25519>), destination: &mut Option<DynamicStackBox<Ed25519>>| -> Option<()> {
                    *destination = Some(initial_edward);
                    destination.as_mut()?.done_with_r().ok()?;
                    Some(())
          })
              ),
                  MoveAction(
                    ObserveLengthedBytes(
                        || DynamicStackBox::<Ed25519>::default(), // move || edward.clone(),
                        |s : &mut DynamicStackBox<Ed25519>, b: &[u8]| s.update(b),
                      /*  || Ed25519::default(), // move || edward.clone(),
                      Ed25519::update,*/
                      Json(DropInterp),
                      true),
                    mkmvfn(| (_, mut final_edward): (_, DynamicStackBox<Ed25519>), destination : &mut Option<ArrayVec<u8,128>> | {
                      final_accept_prompt(&[])?;
                      // let mut final_edward_copy = final_edward.clone();
                      let sig = final_edward.finalize();
                      *destination=Some(ArrayVec::new());
                      destination.as_mut()?.try_extend_from_slice(&sig.ok()?.0).ok()?;
                      Some(())
                    })
                  )
    )));

// The global parser state enum; any parser above that'll be used as the implementation for an APDU
// must have a field here.

#[derive(InPlaceInit)]
#[repr(u8)]
pub enum ParsersStateInner<A, B> {
    NoState,
    GetAddressState(A),
    SignState(B),
    /*GetAddressState(<GetAddressImplT as InterpParser<Bip32Key>>::State),
    SignState(<SignImplT as InterpParser<DoubledSignParameters>>::State),*/
}

pub type ParsersState = ParsersStateInner<<GetAddressImplT as InterpParser<Bip32Key>>::State, <SignImplT as InterpParser<DoubledSignParameters>>::State>;

pub fn reset_parsers_state(state: &mut ParsersState) {
    *state = ParsersState::NoState;
}
/*
pub fn not_a_real_fn() {
    trace!("foo: {:?}",ParsersState_internal::ParsersStateTag::GetAddressState);
}
*/

meta_definition!{}
signer_definition!{}
amount_type_definition!{}
fee_definition!{}
send_value_definition!{}
unjail_value_definition!{}
public_key_definition!{}
stake_value_definition!{}
unstake_value_definition!{}

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
  UnstakeInterp: JsonInterp<UnstakeValueSchema>> {
  pub send_message: SendInterp,
  pub unjail_message: UnjailInterp,
  pub stake_message: StakeInterp,
  pub unstake_message: UnstakeInterp
}

type TemporaryStringState<const N: usize>  = <JsonStringAccumulate<N> as JsonInterp<JsonString>>::State;
type TemporaryStringReturn<const N: usize> = Option<<JsonStringAccumulate<N> as JsonInterp<JsonString>>::Returning>;

#[derive(Debug)]
pub enum MessageState<SendMessageState, UnjailMessageState, StakeMessageState, UnstakeMessageState> {
  Start,
  TypeLabel(TemporaryStringState<4>, TemporaryStringReturn<4>),
  KeySep1,
  Type(TemporaryStringState<64>, TemporaryStringReturn<64>),
  ValueSep(MessageType),
  ValueLabel(MessageType, TemporaryStringState<5>, TemporaryStringReturn<5>),
  KeySep2(MessageType),
  SendMessageState(SendMessageState),
  UnjailMessageState(UnjailMessageState),
  StakeMessageState(StakeMessageState),
  UnstakeMessageState(UnstakeMessageState),
  End,
}

fn init_str<const N: usize>() -> <JsonStringAccumulate<N> as JsonInterp<JsonString>>::State {
    <JsonStringAccumulate<N> as JsonInterp<JsonString>>::init(&JsonStringAccumulate)
}
fn call_str<'a, const N: usize>(ss: &mut <JsonStringAccumulate<N> as JsonInterp<JsonString>>::State, token: JsonToken<'a>, dest: &mut Option<<JsonStringAccumulate<N> as JsonInterp<JsonString>>::Returning>) -> Result<(), Option<OOB>> {
    <JsonStringAccumulate<N> as JsonInterp<JsonString>>::parse(&JsonStringAccumulate, ss, token, dest)
}

pub enum MessageReturn<
    SendMessageReturn,
    UnjailMessageReturn,
    StakeMessageReturn,
    UnstakeMessageReturn> {
  SendMessageReturn(Option<SendMessageReturn>),
  UnjailMessageReturn(Option<UnjailMessageReturn>),
  StakeMessageReturn(Option<StakeMessageReturn>),
  UnstakeMessageReturn(Option<UnstakeMessageReturn>)
}

impl JsonInterp<MessageSchema> for DropInterp {
    type State = <DropInterp as JsonInterp<JsonAny>>::State;
    type Returning = <DropInterp as JsonInterp<JsonAny>>::Returning;
    fn init(&self) -> Self::State {
        <DropInterp as JsonInterp<JsonAny>>::init(&DropInterp)
    }
    fn parse<'a>(&self, state: &mut Self::State, token: JsonToken<'a>, destination: &mut Option<Self::Returning>) -> Result<(), Option<OOB>> {
        <DropInterp as JsonInterp<JsonAny>>::parse(&DropInterp, state, token, destination)
    }
}

impl <SendInterp: JsonInterp<SendValueSchema>,
      UnjailInterp: JsonInterp<UnjailValueSchema>,
      StakeInterp: JsonInterp<StakeValueSchema>,
      UnstakeInterp: JsonInterp<UnstakeValueSchema>>
  JsonInterp<MessageSchema> for Message<SendInterp, UnjailInterp, StakeInterp, UnstakeInterp>
  where
  <SendInterp as JsonInterp<SendValueSchema>>::State: core::fmt::Debug,
  <UnjailInterp as JsonInterp<UnjailValueSchema>>::State: core::fmt::Debug,
  <StakeInterp as JsonInterp<StakeValueSchema>>::State: core::fmt::Debug,
  <UnstakeInterp as JsonInterp<UnstakeValueSchema>>::State: core::fmt::Debug {
  type State = MessageState<<SendInterp as JsonInterp<SendValueSchema>>::State,
                            <UnjailInterp as JsonInterp<UnjailValueSchema>>::State,
                            <StakeInterp as JsonInterp<StakeValueSchema>>::State,
                            <UnstakeInterp as JsonInterp<UnstakeValueSchema>>::State>;
  type Returning = MessageReturn<<SendInterp as JsonInterp<SendValueSchema>>::Returning,
                                 <UnjailInterp as JsonInterp<UnjailValueSchema>>::Returning,
                                 <StakeInterp as JsonInterp<StakeValueSchema>>::Returning,
                                 <UnstakeInterp as JsonInterp<UnstakeValueSchema>>::Returning>;
  fn init(&self) -> Self::State {
    MessageState::Start
  }
  #[inline(never)]
  fn parse<'a>(&self,
               state: &mut Self::State,
               token: JsonToken<'a>,
               destination: &mut Option<Self::Returning>)
               -> Result<(), Option<OOB>> {
    match state {
      MessageState::Start if token == JsonToken::BeginObject => {
        set_from_thunk(state, ||MessageState::TypeLabel(init_str::<4>(), None));
      }
      MessageState::TypeLabel(ref mut temp_string_state, ref mut temp_string_return) => {
        call_str::<4>(temp_string_state, token, temp_string_return)?;
        if temp_string_return.as_ref().unwrap().as_slice() == b"type" {
          set_from_thunk(state, ||MessageState::KeySep1);
        } else {
          return Err(Some(OOB::Reject));
        }
      }
      MessageState::KeySep1 if token == JsonToken::NameSeparator => {
        set_from_thunk(state, ||MessageState::Type(init_str::<64>(), None));
      }
      MessageState::Type(ref mut temp_string_state, ref mut temp_string_return) => {
        call_str::<64>(temp_string_state, token, temp_string_return)?;
        match temp_string_return.as_ref().unwrap().as_slice() {
          b"pos/Send" =>  {
            set_from_thunk(state, ||MessageState::ValueSep(MessageType::SendMessage));
          }
          b"pos/MsgUnjail" =>  {
            set_from_thunk(state, ||MessageState::ValueSep(MessageType::UnjailMessage));
          }
          b"pos/MsgStake" =>  {
            set_from_thunk(state, ||MessageState::ValueSep(MessageType::StakeMessage));
          }
          b"pos/MsgBeginUnstake" =>  {
            set_from_thunk(state, ||MessageState::ValueSep(MessageType::UnstakeMessage));
          }
          _ => return Err(Some(OOB::Reject)),
        }
      }
      MessageState::ValueSep(msg_type) if token == JsonToken::ValueSeparator => {
        let new_msg_type = *msg_type;
        set_from_thunk(state, ||MessageState::ValueLabel(new_msg_type, init_str::<5>(), None));
      }
      MessageState::ValueLabel(msg_type, temp_string_state, temp_string_return) => {
        call_str::<5>(temp_string_state, token, temp_string_return)?;
        if temp_string_return.as_ref().unwrap().as_slice() == b"value" {
          let new_msg_type = *msg_type;
          set_from_thunk(state, ||MessageState::KeySep2(new_msg_type));
        } else {
          return Err(Some(OOB::Reject));
        }
      }
      MessageState::KeySep2(msg_type) if token == JsonToken::NameSeparator => {
        match msg_type {
          MessageType::SendMessage => {
            *destination = Some(MessageReturn::SendMessageReturn(None));
            set_from_thunk(state, ||MessageState::SendMessageState(self.send_message.init()));
          }
          MessageType::UnjailMessage => {
            *destination = Some(MessageReturn::UnjailMessageReturn(None));
            set_from_thunk(state, ||MessageState::UnjailMessageState(self.unjail_message.init()));
          }
          MessageType::StakeMessage => {
            *destination = Some(MessageReturn::StakeMessageReturn(None));
            set_from_thunk(state, ||MessageState::StakeMessageState(self.stake_message.init()));
          }
          MessageType::UnstakeMessage => {
            *destination = Some(MessageReturn::UnstakeMessageReturn(None));
            set_from_thunk(state, ||MessageState::UnstakeMessageState(self.unstake_message.init()));
          }
        }
      }
      MessageState::SendMessageState(ref mut send_message_state) => {
        let sub_destination = &mut destination.as_mut().ok_or(Some(OOB::Reject))?;
        match sub_destination {
          MessageReturn::SendMessageReturn(send_message_return) => {
            self.send_message.parse(send_message_state, token, send_message_return)?;
            set_from_thunk(state, ||MessageState::End);
          }
          _ => {
            return Err(Some(OOB::Reject))
          }
        }
      }
      MessageState::UnjailMessageState(ref mut unjail_message_state) => {
        let sub_destination = &mut destination.as_mut().ok_or(Some(OOB::Reject))?;
        match sub_destination {
          MessageReturn::UnjailMessageReturn(unjail_message_return) => {
            self.unjail_message.parse(unjail_message_state, token, unjail_message_return)?;
            set_from_thunk(state, ||MessageState::End);
          }
          _ => {
            return Err(Some(OOB::Reject))
          }
        }
      }
      MessageState::StakeMessageState(ref mut stake_message_state) => {
        let sub_destination = &mut destination.as_mut().ok_or(Some(OOB::Reject))?;
        match sub_destination {
          MessageReturn::StakeMessageReturn(stake_message_return) => {
            self.stake_message.parse(stake_message_state, token, stake_message_return)?;
            set_from_thunk(state, ||MessageState::End);
          }
          _ => {
            return Err(Some(OOB::Reject))
          }
        }
      }
      MessageState::UnstakeMessageState(ref mut unstake_message_state) => {
        let sub_destination = &mut destination.as_mut().ok_or(Some(OOB::Reject))?;
        match sub_destination {
          MessageReturn::UnstakeMessageReturn(unstake_message_return) => {
            self.unstake_message.parse(unstake_message_state, token, unstake_message_return)?;
            set_from_thunk(state, ||MessageState::End);
          }
          _ => {
            return Err(Some(OOB::Reject))
          }
        }
      }
      MessageState::End if token == JsonToken::EndObject => {
          return Ok(())
      }
      _ => return Err(Some(OOB::Reject)),
    };
    Err(None)
  }
}

pokt_cmd_definition!{}

#[inline(never)]
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

#[inline(never)]
pub fn get_sign_state(
    s: &mut ParsersState,
) -> &mut <SignImplT as InterpParser<DoubledSignParameters>>::State {
    match s {
        ParsersState::SignState(_) => {}
        _ => {
            trace!("Non-same state found; initializing state.");
            unsafe { 
                let s_ptr = s as *mut ParsersState;
                core::ptr::drop_in_place(s_ptr);
                // casting s_ptr to MaybeUninit here _could_ produce UB if init_in_place doesn't
                // fill it; we rely on init_in_place to not panic.
                ParsersState::init_sign_state(core::mem::transmute(s_ptr), |a| { <SignImplT as InterpParser<DoubledSignParameters>>::init_in_place(&SIGN_IMPL, a); });
                trace!("Get_sign_stated");
            }
            /*
            *s = ParsersState::SignState(<SignImplT as InterpParser<DoubledSignParameters>>::init(
                &SIGN_IMPL,
            )); */
        }
    }
    match s {
        ParsersState::SignState(ref mut a) => a,
        _ => {
            trace!("PANICKING");
            panic!("DOOO PANIC")
        }
    }
}

