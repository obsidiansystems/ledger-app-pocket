use ledger_parser_combinators::core_parsers::*;
use ledger_parser_combinators::endianness::*;
use ledger_parser_combinators::json::*;
use ledger_parser_combinators::define_json_struct;

// Payload for a public key request
pub type Bip32Key = DArray<Byte, U32::< { Endianness::Little } >, 10>;

define_json_struct!{ Meta 16 {
    chainId: JsonString,
    sender: JsonString,
    gasLimit: JsonNumber,
    gasPrice: JsonNumber,
    ttl: JsonNumber,
    creationTime: JsonNumber
}}

define_json_struct!{ Signer 16 {
    scheme: JsonString,
    pubKey: JsonString,
    addr: JsonString,
    caps: JsonArray<JsonString>
}}

define_json_struct!{ KadenaCmd 16 {
  nonce: JsonString,
  meta: MetaSchema,
  signers: JsonArray<SignerSchema>,
  payload: JsonAny,
  networkId: JsonString
}}

// Payload for a signature request, content-agnostic.
pub type SignParameters = (LengthFallback<U32::< { Endianness::Little }>, Json<KadenaCmdSchema> >, Bip32Key);


