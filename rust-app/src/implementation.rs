use crate::crypto_helpers::{detecdsa_sign, get_pkh, get_private_key, get_pubkey, Hasher};
use crate::interface::*;
use crate::*;
use arrayvec::{ArrayString, ArrayVec};
use core::fmt::Write;
use core::fmt::Debug;
use ledger_parser_combinators::interp_parser::{
    Action, DefaultInterp, DropInterp, InterpParser, ObserveLengthedBytes, SubInterp, OOB, set_from_thunk
};
use ledger_parser_combinators::json::Json;
use nanos_ui::ui;
use prompts_ui::{write_scroller, final_accept_prompt};
use core::str::from_utf8;
use core::convert::TryFrom;

use ledger_parser_combinators::define_json_struct_interp;
use ledger_parser_combinators::json::*;
use ledger_parser_combinators::json_interp::*;

// A couple type ascription functions to help the compiler along.
const fn mkfn<A,B,C>(q: fn(&A,&mut B)->C) -> fn(&A,&mut B)->C {
  q
}
const fn mkvfn<A,C>(q: fn(&A,&mut Option<()>)->C) -> fn(&A,&mut Option<()>)->C {
  q
}

pub type GetAddressImplT = impl InterpParser<Bip32Key, Returning = ArrayVec<u8, 128>>;

pub const GET_ADDRESS_IMPL: GetAddressImplT =
    Action(SubInterp(DefaultInterp), mkfn(|path: &ArrayVec<u32, 10>, destination: &mut Option<ArrayVec<u8, 128>>| -> Option<()> {
        let key = get_pubkey(path).ok()?;

        let pkh = get_pkh(key).ok()?;

        write_scroller("Provide Public Key", |w| Ok(write!(w, "For Address     {}", pkh)?))?;

        final_accept_prompt(&[])?;

        let rv = destination.insert(ArrayVec::new());
        rv.try_push(u8::try_from(key.len()).ok()?).ok()?;
        rv.try_extend_from_slice(&key).ok()?;
        Some(())
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

pub type SignImplT = impl InterpParser<SignParameters, Returning = ArrayVec<u8, 128>>;

pub const SIGN_IMPL: SignImplT = Action(
    (
        Action(
            // Calculate the hash of the transaction
            ObserveLengthedBytes(
                Hasher::new,
                Hasher::update,
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
                true,
            ),
            // Ask the user if they accept the transaction body's hash
            mkfn(|(_, hash): &(_, Hasher), destination: &mut Option<[u8; 32]>| {
                let the_hash = hash.clone().finalize();
                write_scroller("Sign Hash?", |w| Ok(write!(w, "{}", the_hash)?))?;
                *destination = Some(the_hash.0.into());
                Some(())
            }),
        ),
        Action(
            SubInterp(DefaultInterp),
            // And ask the user if this is the key the meant to sign with:
            mkfn(|path: &ArrayVec<u32, 10>, destination| {
                let privkey = get_private_key(path).ok()?;
                let pubkey = get_pubkey(path).ok()?; // Redoing work here; fix.
                let pkh = get_pkh(pubkey).ok()?;

                write_scroller("For Account", |w| Ok(write!(w, "{}", pkh)?))?;

                *destination = Some(privkey);
                Some(())
            }),
        ),
    ),
    mkfn(|(hash, key): &(Option<[u8; 32]>, Option<_>), destination: &mut Option<ArrayVec<u8, 128>>| {
        // By the time we get here, we've approved and just need to do the signature.
        final_accept_prompt(&[])?;
        let sig = detecdsa_sign(hash.as_ref()?, key.as_ref()?)?;
        let rv = destination.insert(ArrayVec::new());
        rv.try_extend_from_slice(&sig).ok()?;
        Some(())
    }),
);

// The global parser state enum; any parser above that'll be used as the implementation for an APDU
// must have a field here.

pub enum ParsersState {
    NoState,
    GetAddressState(<GetAddressImplT as InterpParser<Bip32Key>>::State),
    SignState(<SignImplT as InterpParser<SignParameters>>::State),
}

pub fn reset_parsers_state(state: &mut ParsersState) {
    *state = ParsersState::NoState;
}

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

