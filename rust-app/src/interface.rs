use core::convert::TryFrom;
use ledger_parser_combinators::core_parsers::*;
use ledger_parser_combinators::define_json_struct;
use ledger_parser_combinators::endianness::*;
use ledger_parser_combinators::json::*;
use nanos_sdk::io::ApduMeta;
use num_enum::TryFromPrimitive;

// Payload for a public key request
pub type Bip32Key = DArray<Byte, U32<{ Endianness::Little }>, 10>;

define_json_struct! { Meta 16 {
    chainId: JsonString,
    sender: JsonString,
    gasLimit: JsonNumber,
    gasPrice: JsonNumber,
    ttl: JsonNumber,
    creationTime: JsonNumber
}}

define_json_struct! { Signer 16 {
    scheme: JsonString,
    pubKey: JsonString,
    addr: JsonString,
    caps: JsonArray<JsonString>
}}

// This should just be called Amount, but we have a name collition between
// field names and type names
define_json_struct! { AmountType 16 {
  amount: JsonString,
  denom: JsonString
}}

define_json_struct! { Fee 16 {
  amount: JsonArray<AmountTypeSchema>,
  gas: JsonString
}}

define_json_struct! { SendValue 16 {
  amount: JsonString,
  from_address: JsonString,
  to_address: JsonString
}}

define_json_struct! { UnjailValue 16 {
  address: JsonString,
  signer_address: JsonString
}}

define_json_struct! { PublicKey 16 {
  type: JsonString,
  value: JsonString
}}

define_json_struct! { StakeValue 16 {
  chains: JsonArray<JsonString>,
  public_key: PublicKeySchema,
  service_url: JsonString,
  value: JsonString,
  output_address: JsonString
}}

define_json_struct! { UnstakeValue 17 {
  signer_address: JsonString,
  validator_address: JsonString
}}

pub struct MessageSchema;

define_json_struct! { PoktCmd 16 {
  chain_id: JsonString,
  entropy: JsonString,
  fee: JsonArray<AmountTypeSchema>,
  memo: JsonString,
  msg: MessageSchema
}}

// Payload for a signature request, content-agnostic.
pub type SignParameters = (
    Bip32Key,
    LengthFallback<U32<{ Endianness::Little }>, Json<PoktCmdSchema>>,
);

pub type DoubledSignParameters = (
    Bip32Key,
    (
        LengthFallback<U32<{ Endianness::Little }>, Json<PoktCmdSchema>>,
        LengthFallback<U32<{ Endianness::Little }>, Json<PoktCmdSchema>>,
    ),
);

#[repr(u8)]
#[derive(Debug, TryFromPrimitive)]
pub enum Ins {
    GetVersion = 0,
    VerifyAddress = 1,
    GetPubkey = 2,
    Sign = 3,
    GetVersionStr = 0xfe,
    Exit = 0xff,
}

impl TryFrom<ApduMeta> for Ins {
    type Error = ();
    fn try_from(m: ApduMeta) -> Result<Ins, Self::Error> {
        match m {
            ApduMeta {
                cla: 0,
                ins,
                p1: 0,
                p2: 0,
            } => Self::try_from(ins).map_err(|_| ()),
            _ => Err(()),
        }
    }
}
