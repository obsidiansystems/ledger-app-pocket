use crate::implementation::*;
use crate::interface::*;
use crate::menu::*;
use crate::settings::*;

use core::fmt::Write;
use ledger_crypto_helpers::hasher::{Base64Hash, Hasher, SHA256};
use ledger_log::{info, trace};
use ledger_parser_combinators::interp_parser::call_me_maybe;
use ledger_parser_combinators::interp_parser::OOB;
use ledger_prompts_ui::{handle_menu_button_event, show_menu, write_scroller};
use nanos_sdk::io;

#[allow(dead_code)]
pub fn app_main() {
    let mut comm = io::Comm::new();
    let mut states = ParsersState::NoState;
    let mut block_state = BlockState::default();

    let mut idle_menu = IdleMenuWithSettings {
        idle_menu: IdleMenu::AppMain,
        settings: Settings::default(),
    };
    let mut busy_menu = BusyMenu::Working;

    // not_a_real_fn();

    info!("Pocket app {}", env!("CARGO_PKG_VERSION"));
    info!(
        "State sizes\ncomm: {}\nstates: {}\nblock_state: {}",
        core::mem::size_of::<io::Comm>(),
        core::mem::size_of::<ParsersState>(),
        core::mem::size_of::<BlockState>()
    );

    let menu = |states: &ParsersState, idle: &IdleMenuWithSettings, busy: &BusyMenu| match states {
        ParsersState::NoState => show_menu(idle),
        _ => show_menu(busy),
    };

    // Draw some 'welcome' screen
    menu(&states, &idle_menu, &busy_menu);
    loop {
        // Wait for either a specific button push to exit the app
        // or an APDU command
        match comm.next_event::<Ins>() {
            io::Event::Command(ins) => {
                trace!("Command received");
                match handle_apdu(
                    &mut comm,
                    ins,
                    &mut states,
                    &mut block_state,
                    idle_menu.settings,
                ) {
                    Ok(()) => {
                        trace!("APDU accepted; sending response");
                        comm.reply_ok();
                        trace!("Replied");
                    }
                    Err(sw) => comm.reply(sw),
                };
                // Reset BusyMenu if we are done handling APDU
                if let ParsersState::NoState = states {
                    busy_menu = BusyMenu::Working;
                }
                menu(&states, &idle_menu, &busy_menu);
                trace!("Command done");
            }
            io::Event::Button(btn) => {
                trace!("Button received");
                match states {
                    ParsersState::NoState => {
                        if let Some(DoExitApp) = handle_menu_button_event(&mut idle_menu, btn) {
                            info!("Exiting app at user direction via root menu");
                            nanos_sdk::exit_app(0)
                        }
                    }
                    _ => {
                        if let Some(DoCancel) = handle_menu_button_event(&mut busy_menu, btn) {
                            info!("Resetting at user direction via busy menu");
                            reset_parsers_state(&mut states)
                        }
                    }
                };
                menu(&states, &idle_menu, &busy_menu);
                trace!("Button done");
            }
            io::Event::Ticker => {
                //trace!("Ignoring ticker event");
            }
        }
    }
}

use arrayvec::ArrayVec;
use nanos_sdk::io::Reply;

use ledger_parser_combinators::interp_parser::InterpParser;

const HASH_LEN: usize = 32;
type BSHA256 = [u8; HASH_LEN];

const MAX_PARAMS: usize = 2;

// Replace with a proper implementation later; this is just to get enough to do the two-pass for
// Ed25519.
#[derive(Default)]
struct BlockState {
    params: ArrayVec<BSHA256, MAX_PARAMS>,
    requested_block: BSHA256,
    state: usize,
}

#[repr(u8)]
#[derive(Copy, Clone)]
enum LedgerToHostCmd {
    // ResultAccumulating = 0, // Not used yet in this app.
    ResultFinal = 1,
    GetChunk = 2,
    // PutChunk = 3
}

#[repr(u8)]
#[derive(Debug)]
enum HostToLedgerCmd {
    Start = 0,
    GetChunkResponseSuccess = 1,
    GetChunkResponseFailure = 2,
    PutChunkResponse = 3,
    ResultAccumulatingResponse = 4,
}

impl TryFrom<u8> for HostToLedgerCmd {
    type Error = Reply;
    fn try_from(a: u8) -> Result<HostToLedgerCmd, Reply> {
        match a {
            0 => Ok(HostToLedgerCmd::Start),
            1 => Ok(HostToLedgerCmd::GetChunkResponseSuccess),
            2 => Ok(HostToLedgerCmd::GetChunkResponseFailure),
            3 => Ok(HostToLedgerCmd::PutChunkResponse),
            4 => Ok(HostToLedgerCmd::ResultAccumulatingResponse),
            _ => Err(io::StatusWords::Unknown.into()),
        }
    }
}

/*
trait BlockyAdapterScheme {
    const first_param : usize;
    fn next_block<'a, 'b>(&'a mut self, params : &'b ArrayVec<BSHA256, MAX_PARAMS>) -> Result<&'b [u8], Reply>;
}

enum SignStateEnum {
    FirstPassPath,
    FirstPassTxn,
    SecondPassTxn,
    SecondPassPath
}

impl Default for SignStateEnum {
    fn default() -> SignStateEnum {
        SignStateEnum::FirstPassPath
    }
}

impl BlockyAdapterScheme for SignStateEnum {
    const first_param: usize = 0;
    fn next_block<'a, 'b>(&'a mut self, params : &'b ArrayVec<BSHA256, MAX_PARAMS>) -> Result<&'b [u8], Reply> -> {
        match self {
            BlockStateEnum::FirstPassPath => {
                *self = BlockStateEnum::FirstPassTxn;
                Ok(&params[0])
            }
            BlockStateEnum::FirstPassTxn => {
                *self = BlockStateEnum::SecondPassPath;
                Ok(&params[1])
            }
            BlockStateEnum::SecondPassPath => {
                *self = BlockStateEnum::SecondPassTxn;
                Ok(&params[0])
            }
            BlockStateEnum::SecondPassTxn => {
                return Err(io::StatusWords::Unknown.into());
            }
        }
        Ok(())
    }
}

#[derive(Default)]
struct OneParamOnceState;

impl BlockyAdapterScheme for OneParamOnceState {
    const first_param: usize = 0;
    fn next_block(&mut self, &mut next_block_out : &[u8]) -> Result<&[u8], ()> {
        Err(io::StatusWords::Unknown.into())
    }
}
*/

use ledger_parser_combinators::interp_parser::ParserCommon;

#[inline(never)]
fn run_parser_apdu<P: InterpParser<A, Returning = ArrayVec<u8, 128>>, A, const N: usize>(
    states: &mut ParsersState,
    get_state: fn(&mut ParsersState) -> &mut <P as ParserCommon<A>>::State,
    block_state: &mut BlockState,
    seq: &[usize; N],
    parser: &P,
    comm: &mut io::Comm,
) -> Result<(), Reply> {
    trace!("Entered run_parser_apdu_signing");
    let block: &[u8] = comm.get_data()?;

    let host_cmd: HostToLedgerCmd =
        HostToLedgerCmd::try_from(*block.get(0).ok_or(io::StatusWords::Unknown)?)?;

    trace!("Host cmd: {:?}", host_cmd);
    match host_cmd {
        HostToLedgerCmd::Start => {
            *block_state = BlockState::default();
            reset_parsers_state(states);
            block_state.params.clear();
            for param in block[1..].chunks_exact(HASH_LEN) {
                block_state
                    .params
                    .try_push(param.try_into().or(Err(io::StatusWords::Unknown))?)
                    .or(Err(io::StatusWords::Unknown))?;
            }
            trace!("Params: {:x?}", block_state.params);
            block_state.state = 0;
            if block_state.params.len() <= *seq.iter().max().unwrap() {
                return Err(io::StatusWords::Unknown.into());
            }
            block_state
                .requested_block
                .copy_from_slice(&block_state.params[seq[block_state.state]][..]);
            comm.append(&[LedgerToHostCmd::GetChunk as u8]);
            comm.append(&block_state.requested_block);
            Ok(())
        }
        HostToLedgerCmd::GetChunkResponseSuccess => {
            if block.len() < HASH_LEN + 1 {
                return Err(io::StatusWords::Unknown.into());
            }

            // Check the hash, so the host can't lie.
            call_me_maybe(|| {
                let mut hasher = SHA256::new();
                hasher.update(&block[1..]);
                let hashed = hasher.finalize::<Base64Hash<{ SHA256::N }>>();
                if hashed.0 != block_state.requested_block {
                    None
                } else {
                    Some(())
                }
            })
            .ok_or(io::StatusWords::Unknown)?;

            let next_block = &block[1..1 + HASH_LEN];
            let cursor = &block[1 + HASH_LEN..];

            trace!("Parsing APDU input: {:?}\n", cursor);
            let mut parse_destination = None;
            let gs = get_state(states);
            trace!("State got, calling parser");
            let parse_rv =
                <P as InterpParser<A>>::parse(parser, gs, cursor, &mut parse_destination);
            trace!("Parser result: {:?}\n", parse_rv);
            trace!("Parse destination: {:?}\n", parse_destination);
            match parse_rv {
                // Explicit rejection; reset the parser. Possibly send error message to host?
                Err((Some(OOB::Reject), _)) => {
                    reset_parsers_state(states);
                    Err(io::StatusWords::Unknown.into())
                }
                // Deliberately no catch-all on the Err((Some case; we'll get error messages if we
                // add to OOB's out-of-band actions and forget to implement them.
                //
                // Finished the chunk with no further actions pending, but not done.
                Err((None, [])) => {
                    trace!("Parser needs more; get more.");
                    // Request the next chunk of our input.
                    let our_next_block: &[u8] = if next_block == [0; 32] {
                        block_state.state += 1;
                        if block_state.state > seq.len() {
                            return Err(io::StatusWords::Unknown.into());
                        }
                        if block_state.params.len() <= seq[block_state.state] {
                            return Err(io::StatusWords::Unknown.into());
                        }
                        &block_state.params[seq[block_state.state]]
                    } else {
                        next_block
                    };
                    trace!("Next block: {:x?}", our_next_block);

                    block_state.requested_block.copy_from_slice(our_next_block);
                    comm.append(&[LedgerToHostCmd::GetChunk as u8]);
                    comm.append(&block_state.requested_block);
                    trace!("Requesting next block from host");
                    Ok(())
                }
                // Didn't consume the whole chunk; reset and error message.
                Err((None, _)) => {
                    reset_parsers_state(states);
                    Err(io::StatusWords::Unknown.into())
                }
                // Consumed the whole chunk and parser finished; send response.
                Ok([]) => {
                    trace!("Parser finished, resetting state\n");
                    match parse_destination.as_ref() {
                        Some(rv) => {
                            comm.append(&[LedgerToHostCmd::ResultFinal as u8]);
                            comm.append(&rv[..]);
                        }
                        None => return Err(io::StatusWords::Unknown.into()),
                    }
                    // Parse finished; reset.
                    reset_parsers_state(states);
                    Ok(())
                }
                // Parse ended before the chunk did; reset.
                Ok(_) => {
                    reset_parsers_state(states);
                    Err(io::StatusWords::Unknown.into())
                }
            }
        }
        _ => Err(io::StatusWords::Unknown.into()),
    }
}

// fn handle_apdu<P: for<'a> FnMut(ParserTag, &'a [u8]) -> RX<'a, ArrayVec<u8, 260> > >(comm: &mut io::Comm, ins: Ins, parser: &mut P) -> Result<(), Reply> {
#[inline(never)]
fn handle_apdu(
    comm: &mut io::Comm,
    ins: Ins,
    parser: &mut ParsersState,
    block_state: &mut BlockState,
    settings: Settings,
) -> Result<(), Reply> {
    info!("entering handle_apdu with command {:?}", ins);
    if comm.rx == 0 {
        return Err(io::StatusWords::NothingReceived.into());
    }

    match ins {
        Ins::GetVersion => {
            comm.append(&[LedgerToHostCmd::ResultFinal as u8]);
            comm.append(&[
                env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap(),
                env!("CARGO_PKG_VERSION_MINOR").parse().unwrap(),
                env!("CARGO_PKG_VERSION_PATCH").parse().unwrap(),
            ]);
            comm.append(b"Pocket");
        }
        Ins::VerifyAddress => run_parser_apdu::<_, Bip32Key, _>(
            parser,
            get_get_address_state::<true>,
            block_state,
            &[0],
            &get_address_impl::<true>(),
            comm,
        )?,
        Ins::GetPubkey => run_parser_apdu::<_, Bip32Key, _>(
            parser,
            get_get_address_state::<false>,
            block_state,
            &[0],
            &get_address_impl::<false>(),
            comm,
        )?,
        Ins::Sign => run_parser_apdu::<_, DoubledSignParameters, _>(
            parser,
            get_sign_state,
            block_state,
            &SIGN_SEQ,
            &SIGN_IMPL,
            comm,
        )?,
        Ins::BlindSign => {
            if settings.get() != 1 {
                write_scroller(false, "Blind Signing must", |w| {
                    Ok(write!(w, "be enabled")?)
                });
                return Err(io::SyscallError::NotSupported.into());
            } else {
                run_parser_apdu::<_, DoubledBlindSignParameters, _>(
                    parser,
                    get_blind_sign_state,
                    block_state,
                    &SIGN_SEQ,
                    &BLIND_SIGN_IMPL,
                    comm,
                )?
            }
        }
        Ins::GetVersionStr => {
            comm.append(&[LedgerToHostCmd::ResultFinal as u8]);
            comm.append(concat!("Pocket ", env!("CARGO_PKG_VERSION")).as_ref());
        }
        Ins::Exit => nanos_sdk::exit_app(0),
    }
    Ok(())
}
