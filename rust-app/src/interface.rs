use ledger_parser_combinators::core_parsers::*;
use ledger_parser_combinators::define_json_struct;
use ledger_parser_combinators::endianness::*;
use ledger_parser_combinators::json::*;

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
  from_address: JsonString,
  to_address: JsonString,
  amount: JsonArray<AmountTypeSchema>
}}

define_json_struct! { UnjailValue 16 {
  address: JsonString
}}

define_json_struct! { StakeValue 16 {
  public_key: JsonString,
  chains: JsonArray<JsonString>,
  value: JsonString,
  service_url: JsonString
}}

pub struct MessageSchema;

define_json_struct! { KadenaCmd 16 {
  account_number: JsonString,
  chain_id: JsonString,
  fee: FeeSchema,
  memo: JsonString,
  msgs: JsonArray<MessageSchema>,
  sequence: JsonString
}}

// Payload for a signature request, content-agnostic.
pub type SignParameters = (
    LengthFallback<U32<{ Endianness::Little }>, Json<KadenaCmdSchema>>,
    Bip32Key,
);
