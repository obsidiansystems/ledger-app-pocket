use crate::crypto_helpers::{detecdsa_sign, get_pkh, get_private_key, get_pubkey, Hasher};
use crate::interface::*;
use arrayvec::{ArrayString, ArrayVec};
use core::fmt::Write;
use ledger_log::*;
use ledger_parser_combinators::interp_parser::{
    Action, DefaultInterp, DropInterp, InterpParser, ObserveLengthedBytes, SubInterp, OOB, set_from_thunk
};
use ledger_parser_combinators::json::Json;
use nanos_ui::ui;

use ledger_parser_combinators::define_json_struct_interp;
use ledger_parser_combinators::json::*;
use ledger_parser_combinators::json_interp::*;

pub type GetAddressImplT =
    Action<SubInterp<DefaultInterp>, fn(&ArrayVec<u32, 10>, &mut Option<ArrayVec<u8, 260>>) -> Option<()>>;

pub const GET_ADDRESS_IMPL: GetAddressImplT =
    Action(SubInterp(DefaultInterp), |path: &ArrayVec<u32, 10>, destination| {
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
            *destination = Some(rv);
            Some(())
        }
    });

type CmdInterp = KadenaCmd<
    DropInterp,
    DropInterp,
    DropInterp,
    DropInterp,
    SubInterp<Message<
      SendMessageActionT,
      DropInterp,
      StakeMessageActionT,
      UnstakeMessageActionT>>,
    DropInterp>;

type SendMessageActionT = SendValue<
    Action<JsonStringAccumulate<64>, fn(& ArrayVec<u8, 64>, &mut Option<()>) -> Option<()>>,
    Action<JsonStringAccumulate<64>, fn(& ArrayVec<u8, 64>, &mut Option<()>) -> Option<()>>,
    SubInterp<Action<AmountType<JsonStringAccumulate<64>, JsonStringAccumulate<64>>,
                     fn(& AmountType<Option<ArrayVec<u8, 64>>, Option<ArrayVec<u8, 64>>>, &mut Option<()>) -> Option<()>>>
>;

type StakeMessageActionT = StakeValue<
    Action<JsonStringAccumulate<64>, fn(& ArrayVec<u8, 64>, &mut Option<()>) -> Option<()>>,
    SubInterp<Action<JsonStringAccumulate<64>,
                     fn(& ArrayVec<u8, 64>, &mut Option<()>) -> Option<()>>>,
    Action<JsonStringAccumulate<64>, fn(& ArrayVec<u8, 64>, &mut Option<()>) -> Option<()>>,
    Action<JsonStringAccumulate<64>, fn(& ArrayVec<u8, 64>, &mut Option<()>) -> Option<()>>
>;

type UnstakeMessageActionT = UnstakeValue<
    Action<JsonStringAccumulate<64>, fn(& ArrayVec<u8, 64>, &mut Option<()>) -> Option<()>>
    >;

const NANOS_DISPLAY_LENGHT: usize = 15;

const FROM_ADDRESS_ACTION: Action<JsonStringAccumulate<64>,
                                  fn(& ArrayVec<u8, 64>, &mut Option<()>) -> Option<()>> =
  Action(JsonStringAccumulate::<64>,
        | from_address, destination | {

          let prompt = from_address
            .chunks(NANOS_DISPLAY_LENGHT)
            .map(| chunk | core::str::from_utf8(chunk))
            .collect::<Result<ArrayVec<&str, 5>, _>>()
            .ok()?;

          let mut concatenated = ArrayVec::<_, 10>::new();

          concatenated.try_push("Transfer from:").ok()?;
          concatenated.try_extend_from_slice(&prompt[..]).ok()?;

          if !ui::MessageValidator::new(&concatenated[..], &[&"Confirm"], &[&"Reject"]).ask() {
              None
          } else {
              *destination = Some(());
              Some(())
          }
        });

const TO_ADDRESS_ACTION: Action<JsonStringAccumulate<64>,
                                  fn(& ArrayVec<u8, 64>, &mut Option<()>) -> Option<()>> =
  Action(JsonStringAccumulate::<64>,
        | to_address, destination | {

          let prompt = to_address
            .chunks(NANOS_DISPLAY_LENGHT)
            .map(| chunk | core::str::from_utf8(chunk))
            .collect::<Result<ArrayVec<&str, 5>, _>>()
            .ok()?;

          let mut concatenated = ArrayVec::<_, 10>::new();

          concatenated.try_push("Transfer to:").ok()?;
          concatenated.try_extend_from_slice(&prompt[..]).ok()?;

          if !ui::MessageValidator::new(&concatenated[..], &[&"Confirm"], &[&"Reject"]).ask() {
              None
          } else {
              *destination = Some(());
              Some(())
          }
        });

const AMOUNT_ACTION: Action<AmountType<JsonStringAccumulate<64>, JsonStringAccumulate<64>>,
                                  fn(& AmountType<Option<ArrayVec<u8, 64>>, Option<ArrayVec<u8, 64>>>, &mut Option<()>) -> Option<()>> =
  Action(AmountType{field_amount: JsonStringAccumulate::<64>, field_denom: JsonStringAccumulate::<64>},
        | AmountType{field_amount: amount, field_denom: denom}, destination | {

          // let prompt =
          //   .chunks(NANOS_DISPLAY_LENGHT)
          //   .map(| chunk | core::str::from_utf8(chunk))
          //   .collect::<Result<ArrayVec<&str, 5>, _>>()
          //   .ok()?;

          let mut concatenated = ArrayVec::<_, 10>::new();

          concatenated.try_push("Amount:").ok()?;
          concatenated.try_push(core::str::from_utf8(amount.as_ref()?).ok()?).ok()?;
          concatenated.try_push("Denom:").ok()?;
          concatenated.try_push(core::str::from_utf8(denom.as_ref()?).ok()?).ok();

          if !ui::MessageValidator::new(&concatenated[..], &[&"Confirm"], &[&"Reject"]).ask() {
              None
          } else {
              *destination = Some(());
              Some(())
          }
        });

const SEND_MESSAGE_ACTION: SendMessageActionT =
  SendValue{field_from_address: FROM_ADDRESS_ACTION,
            field_to_address: TO_ADDRESS_ACTION,
            field_amount: SubInterp(AMOUNT_ACTION)};

const PUBLIC_KEY_ACTION: Action<JsonStringAccumulate<64>,
                                fn(& ArrayVec<u8, 64>, &mut Option<()>) -> Option<()>> =
  Action(JsonStringAccumulate::<64>,
        | public_key, destination | {

          let prompt = public_key
            .chunks(NANOS_DISPLAY_LENGHT)
            .map(| chunk | core::str::from_utf8(chunk))
            .collect::<Result<ArrayVec<&str, 5>, _>>()
            .ok()?;

          let mut concatenated = ArrayVec::<_, 10>::new();

          concatenated.try_push("Stake with public key:").ok()?;
          concatenated.try_extend_from_slice(&prompt[..]).ok()?;

          if !ui::MessageValidator::new(&concatenated[..], &[&"Confirm"], &[&"Reject"]).ask() {
              None
          } else {
              *destination = Some(());
              Some(())
          }
        });

const CHAIN_ACTION: Action<JsonStringAccumulate<64>,
                           fn(& ArrayVec<u8, 64>, &mut Option<()>) -> Option<()>> =
  Action(JsonStringAccumulate::<64>,
        | chain, destination | {

          let prompt = chain
            .chunks(NANOS_DISPLAY_LENGHT)
            .map(| chunk | core::str::from_utf8(chunk))
            .collect::<Result<ArrayVec<&str, 5>, _>>()
            .ok()?;

          let mut concatenated = ArrayVec::<_, 10>::new();

          concatenated.try_push("Chain:").ok()?;
          concatenated.try_extend_from_slice(&prompt[..]).ok()?;

          if !ui::MessageValidator::new(&concatenated[..], &[&"Confirm"], &[&"Reject"]).ask() {
              None
          } else {
              *destination = Some(());
              Some(())
          }
        });

const VALUE_ACTION: Action<JsonStringAccumulate<64>,
                                 fn(& ArrayVec<u8, 64>, &mut Option<()>) -> Option<()>> =
  Action(JsonStringAccumulate::<64>,
        | value, destination | {

          let prompt = value
            .chunks(NANOS_DISPLAY_LENGHT)
            .map(| chunk | core::str::from_utf8(chunk))
            .collect::<Result<ArrayVec<&str, 5>, _>>()
            .ok()?;

          let mut concatenated = ArrayVec::<_, 10>::new();

          concatenated.try_push("Value:").ok()?;
          concatenated.try_extend_from_slice(&prompt[..]).ok()?;

          if !ui::MessageValidator::new(&concatenated[..], &[&"Confirm"], &[&"Reject"]).ask() {
              None
          } else {
              *destination = Some(());
              Some(())
          }
        });

const SERVICE_URL_ACTION: Action<JsonStringAccumulate<64>,
                                 fn(& ArrayVec<u8, 64>, &mut Option<()>) -> Option<()>> =
  Action(JsonStringAccumulate::<64>,
        | service_url, destination | {

          let prompt = service_url
            .chunks(NANOS_DISPLAY_LENGHT)
            .map(| chunk | core::str::from_utf8(chunk))
            .collect::<Result<ArrayVec<&str, 5>, _>>()
            .ok()?;

          let mut concatenated = ArrayVec::<_, 10>::new();

          concatenated.try_push("Service url:").ok()?;
          concatenated.try_extend_from_slice(&prompt[..]).ok()?;

          if !ui::MessageValidator::new(&concatenated[..], &[&"Confirm"], &[&"Reject"]).ask() {
              None
          } else {
              *destination = Some(());
              Some(())
          }
        });

const STAKE_MESSAGE_ACTION: StakeMessageActionT =
  StakeValue{field_public_key: PUBLIC_KEY_ACTION,
             field_chains: SubInterp(CHAIN_ACTION),
             field_value: VALUE_ACTION,
             field_service_url: SERVICE_URL_ACTION};

const UNSTAKE_MESSAGE_ACTION: UnstakeMessageActionT =
  UnstakeValue{field_validator_address: FROM_ADDRESS_ACTION};

pub type SignImplT = Action<
    (
        Action<
            ObserveLengthedBytes<
                Hasher,
                fn(&mut Hasher, &[u8]),
                Json<CmdInterp>
            >,
            fn(
                &(
                    Option<<CmdInterp as JsonInterp<KadenaCmdSchema>>::Returning>,
                    Hasher,
                ),
                &mut Option<[u8; 32]>
            ) -> Option<()>,
        >,
        Action<
            SubInterp<DefaultInterp>,
            fn(&ArrayVec<u32, 10>, &mut Option<nanos_sdk::bindings::cx_ecfp_private_key_t>) -> Option<()>,
        >,
    ),
    fn(&(Option<[u8; 32]>, Option<nanos_sdk::bindings::cx_ecfp_private_key_t>), &mut Option<ArrayVec<u8, 260>>) -> Option<()>,
>;

pub const SIGN_IMPL: SignImplT = Action(
    (
        Action(
            // Calculate the hash of the transaction
            ObserveLengthedBytes(
                Hasher::new,
                Hasher::update,
                Json(KadenaCmd {
                    field_account_number: DropInterp,
                    field_chain_id: DropInterp,
                    field_fee: DropInterp,
                    field_memo: DropInterp,
                    field_msgs: SubInterp(Message {send_message: SEND_MESSAGE_ACTION,
                                                   unjail_message: DropInterp,
                                                   stake_message: STAKE_MESSAGE_ACTION,
                                                   unstake_message: UNSTAKE_MESSAGE_ACTION}),
                    field_sequence: DropInterp,
                }),
                true,
            ),
            // Ask the user if they accept the transaction body's hash
            |(_, hash): &(_, Hasher), destination| {
                let the_hash = hash.clone().finalize();

                let mut pmpt = ArrayString::<128>::new();
                write!(pmpt, "{}", the_hash).ok()?;

                if !ui::MessageValidator::new(&["Sign Hash?", &pmpt], &[&"Confirm"], &[&"Reject"]).ask() {
                    None
                } else {
                    *destination = Some(the_hash.0.into());
                    Some(())
                }
            },
        ),
        Action(
            SubInterp(DefaultInterp),
            // And ask the user if this is the key the meant to sign with:
            |path: &ArrayVec<u32, 10>, destination| {
                let privkey = get_private_key(path).ok()?;
                let pubkey = get_pubkey(path).ok()?; // Redoing work here; fix.
                let pkh = get_pkh(pubkey);

                let mut pmpt = ArrayString::<128>::new();
                write!(pmpt, "{}", pkh).ok()?;

                if !ui::MessageValidator::new(&["With PKH", &pmpt], &[&"Confirm"], &[&"Reject"]).ask() {
                    None
                } else {
                    *destination = Some(privkey);
                    Some(())
                }
            },
        ),
    ),
    |(hash, key): &(Option<[u8; 32]>, _), destination: &mut Option<ArrayVec<u8, 260>>| {
        // By the time we get here, we've approved and just need to do the signature.
        let (sig, len) = detecdsa_sign(hash.as_ref()?, key.as_ref()?)?;
        let mut rv = ArrayVec::<u8, 260>::new();
        rv.try_extend_from_slice(&sig[0..len as usize]).ok()?;
        *destination = Some(rv);
        Some(())
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

// This should just be called Amount, but we have a name collition between
// field names and type names
define_json_struct_interp! { AmountType 16 {
  amount: JsonString,
  denom: JsonString
}}

define_json_struct_interp! { Fee 16 {
  amount: JsonArray<AmountTypeSchema>,
  gas: JsonString
}}

define_json_struct_interp! { SendValue 16 {
  from_address: JsonString,
  to_address: JsonString,
  amount: JsonArray<AmountTypeSchema>
}}

define_json_struct_interp! { UnjailValue 16 {
  address: JsonString
}}

define_json_struct_interp! { StakeValue 16 {
  public_key: JsonString,
  chains: JsonArray<JsonString>,
  value: JsonString,
  service_url: JsonString
}}

define_json_struct_interp! { UnstakeValue 20 {
  validator_address: JsonString
}}

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
        set_from_thunk(state, ||MessageState::TypeLabel(JsonStringAccumulate.init(), None));
      }
      MessageState::TypeLabel(ref mut temp_string_state, ref mut temp_string_return) => {
        JsonStringAccumulate.parse(temp_string_state, token, temp_string_return)?;
        if temp_string_return.as_ref().unwrap().as_slice() == b"type" {
          set_from_thunk(state, ||MessageState::KeySep1);
        } else {
          return Err(Some(OOB::Reject));
        }
      }
      MessageState::KeySep1 if token == JsonToken::NameSeparator => {
        set_from_thunk(state, ||MessageState::Type(JsonStringAccumulate.init(), None));
      }
      MessageState::Type(ref mut temp_string_state, ref mut temp_string_return) => {
        JsonStringAccumulate.parse(temp_string_state, token, temp_string_return)?;
        match temp_string_return.as_ref().unwrap().as_slice() {
          b"cosmos-sdk/MsgSend" =>  {
            set_from_thunk(state, ||MessageState::ValueSep(MessageType::SendMessage));
          }
          b"cosmos-sdk/MsgUnjail" =>  {
            set_from_thunk(state, ||MessageState::ValueSep(MessageType::UnjailMessage));
          }
          b"cosmos-sdk/MsgStake" =>  {
            set_from_thunk(state, ||MessageState::ValueSep(MessageType::StakeMessage));
          }
          b"cosmos-sdk/MsgUnstake" =>  {
            set_from_thunk(state, ||MessageState::ValueSep(MessageType::UnstakeMessage));
          }
          _ => return Err(Some(OOB::Reject)),
        }
      }
      MessageState::ValueSep(msg_type) if token == JsonToken::ValueSeparator => {
        let new_msg_type = *msg_type;
        set_from_thunk(state, ||MessageState::ValueLabel(new_msg_type, JsonStringAccumulate.init(), None));
      }
      MessageState::ValueLabel(msg_type, temp_string_state, temp_string_return) => {
        JsonStringAccumulate.parse(temp_string_state, token, temp_string_return)?;
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

define_json_struct_interp! { KadenaCmd 16 {
  account_number: JsonString,
  chain_id: JsonString,
  fee: FeeSchema,
  memo: JsonString,
  msgs: JsonArray<MessageSchema>,
  sequence: JsonString

}}

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
