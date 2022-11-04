pub use ledger_crypto_helpers::common::*;
pub use ledger_crypto_helpers::hasher::*;
pub use ledger_crypto_helpers::eddsa::*;
pub use ledger_crypto_helpers::ed25519::*;

pub const BIP32_PREFIX: [u32; 3] = nanos_sdk::ecc::make_bip32_path(b"m/44'/635'");
