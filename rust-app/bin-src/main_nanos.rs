use pocket::implementation::*;
use ledger_prompts_ui::RootMenu;
use core::convert::{TryFrom, TryInto};
use ledger_parser_combinators::interp_parser::{set_from_thunk, call_me_maybe};
use crypto_helpers::{Hash, Hasher};
use nanos_sdk::io;

nanos_sdk::set_panic!(nanos_sdk::exiting_panic);

use ledger_parser_combinators::interp_parser::OOB;
use pocket::*;

#[cfg(not(test))]
#[no_mangle]
extern "C" fn sample_main() {
    let mut comm = io::Comm::new();
    let mut states = ParsersState::NoState;
    let mut block_state = BlockState::default();

    let mut idle_menu = RootMenu::new([ concat!("Pocket ", env!("CARGO_PKG_VERSION")), "Exit" ]);
    let mut busy_menu = RootMenu::new([ "Working...", "Cancel" ]);

    // not_a_real_fn();

    info!("Pocket app {}", env!("CARGO_PKG_VERSION"));
    info!("State sizes\ncomm: {}\nstates: {}\nblock_state: {}", core::mem::size_of::<io::Comm>(), core::mem::size_of::<ParsersState>(), core::mem::size_of::<BlockState>());

    let // Draw some 'welcome' screen
        menu = |states : &ParsersState, idle : & mut RootMenu<2>, busy : & mut RootMenu<2>| {
            match states {
                ParsersState::NoState => idle.show(),
                _ => busy.show(),
            }
        };

    menu(&states, & mut idle_menu, & mut busy_menu);
    loop {
        // Wait for either a specific button push to exit the app
        // or an APDU command
        match comm.next_event() {
            io::Event::Command(ins) => {
                trace!("Command received");
                match handle_apdu(&mut comm, ins, &mut states, &mut block_state) {
                    Ok(()) => {
                        trace!("APDU accepted; sending response");
                        comm.reply_ok();
                        trace!("Replied");
                    }
                    Err(sw) => comm.reply(sw),
                };
                menu(&states, & mut idle_menu, & mut busy_menu);
                trace!("Command done");
            }
            io::Event::Button(btn) => {
                trace!("Button received");
                match states {
                    ParsersState::NoState => {match idle_menu.update(btn) {
                        Some(1) => { info!("Exiting app at user direction via root menu"); nanos_sdk::exit_app(0) },
                        _ => (),
                    } }
                    _ => { match busy_menu.update(btn) {
                        Some(1) => { info!("Resetting at user direction via busy menu"); set_from_thunk(&mut states, || ParsersState::NoState); }
                        _ => (),
                    } }
                };
                menu(&states, & mut idle_menu, & mut busy_menu);
                trace!("Button done");
            }
            io::Event::Ticker => {
                //trace!("Ignoring ticker event");
            },
        }
    }
}

#[repr(u8)]
#[derive(Debug)]
enum Ins {
    GetVersion,
    GetPubkey,
    Sign,
    GetVersionStr,
    Exit
}

impl TryFrom<u8> for Ins {
    type Error = ();
    fn try_from(ins: u8) -> Result<Ins, ()> {
        match ins {
            0 => Ok(Ins::GetVersion),
            2 => Ok(Ins::GetPubkey),
            3 => Ok(Ins::Sign),
            0xfe => Ok(Ins::GetVersionStr),
            0xff => Ok(Ins::Exit),
            _ => Err(()),
        }
    }
}

use arrayvec::ArrayVec;
use nanos_sdk::io::Reply;

use ledger_parser_combinators::interp_parser::InterpParser;

const HASH_LEN: usize = 32;
type SHA256 = [u8; HASH_LEN];

const MAX_PARAMS: usize = 2;

// Replace with a proper implementation later; this is just to get enough to do the two-pass for
// Ed25519.
#[derive(Default)]
struct BlockState {
    params: ArrayVec<SHA256, MAX_PARAMS>,
    requested_block: SHA256,
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
    START = 0,
    GetChunkResponseSuccess = 1,
    GetChunkResponseFailure = 2,
    PutChunkResponse = 3,
    ResultAccumulatingResponse = 4
}

impl TryFrom<u8> for HostToLedgerCmd {
    type Error = Reply;
    fn try_from(a: u8) -> Result<HostToLedgerCmd, Reply> {
        match a {
            0 => Ok(HostToLedgerCmd::START),
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
    fn next_block<'a, 'b>(&'a mut self, params : &'b ArrayVec<SHA256, MAX_PARAMS>) -> Result<&'b [u8], Reply>;
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
    fn next_block<'a, 'b>(&'a mut self, params : &'b ArrayVec<SHA256, MAX_PARAMS>) -> Result<&'b [u8], Reply> -> {
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

#[inline(never)]
fn run_parser_apdu<P: InterpParser<A, Returning = ArrayVec<u8,128>>, A, const N: usize>(
    states: &mut ParsersState,
    get_state: fn(&mut ParsersState) -> &mut <P as InterpParser<A>>::State,
    block_state: &mut BlockState,
    seq: &[usize; N],
    parser: &P,
    comm: &mut io::Comm,
) -> Result<(), Reply> {

    trace!("Entered run_parser_apdu_signing");
    let block: &[u8] = comm.get_data()?;

    let host_cmd : HostToLedgerCmd = HostToLedgerCmd::try_from(*block.get(0).ok_or(io::StatusWords::Unknown)?)?;

    trace!("Host cmd: {:?}", host_cmd);
    match host_cmd {
        HostToLedgerCmd::START => {
            *block_state = BlockState::default();
            reset_parsers_state(states);
            block_state.params.clear();
            for param in block[1..].chunks_exact(HASH_LEN) {
                block_state.params.try_push(param.try_into().or(Err(io::StatusWords::Unknown))?).or(Err(io::StatusWords::Unknown))?;
            }
            trace!("Params: {:x?}", block_state.params);
            block_state.state = 0;
            if block_state.params.len() <= *seq.iter().max().unwrap() { return Err(io::StatusWords::Unknown.into()); }
            block_state.requested_block.copy_from_slice(&block_state.params[seq[block_state.state]][..]);
            comm.append(&[LedgerToHostCmd::GetChunk as u8]);
            comm.append(&block_state.requested_block);
            Ok(())
        }
        HostToLedgerCmd::GetChunkResponseSuccess => {
            if block.len() < HASH_LEN+1 { return Err(io::StatusWords::Unknown.into()); }

            // Check the hash, so the host can't lie.
            call_me_maybe( || {
                let mut hasher = Hasher::new();
                hasher.update(&block[1..]);
                let Hash(hashed) = hasher.finalize();
                if hashed != block_state.requested_block {
                    None
                } else {
                    Some(())
                }
            }).ok_or(io::StatusWords::Unknown)?;

            let next_block = &block[1..1+HASH_LEN];
            let cursor = &block[1+HASH_LEN..];

            trace!("Parsing APDU input: {:?}\n", cursor);
            let mut parse_destination = None;
            let gs = get_state(states);
            trace!("State got, calling parser");
            let parse_rv = <P as InterpParser<A>>::parse(parser, gs, cursor, &mut parse_destination);
            trace!("Parser result: {:?}\n", parse_rv);
            trace!("Parse destination: {:?}\n", parse_destination);
            match parse_rv {
                // Explicit rejection; reset the parser. Possibly send error message to host?
                Err((Some(OOB::Reject), _)) => {
                    reset_parsers_state(states);
                    return Err(io::StatusWords::Unknown.into());
                }
                // Deliberately no catch-all on the Err((Some case; we'll get error messages if we
                // add to OOB's out-of-band actions and forget to implement them.
                //
                // Finished the chunk with no further actions pending, but not done.
                Err((None, [])) => {
                    trace!("Parser needs more; get more.");
                    // Request the next chunk of our input.
                    let our_next_block : &[u8] = if next_block == [0; 32] {
                        block_state.state = block_state.state + 1;
                        if block_state.state > seq.len() { return Err(io::StatusWords::Unknown.into()); }
                        if block_state.params.len() <= seq[block_state.state] { return Err(io::StatusWords::Unknown.into()); }
                        &block_state.params[seq[block_state.state]]
                    } else {
                        &next_block
                    };
                    trace!("Next block: {:x?}", our_next_block);

                    block_state.requested_block.copy_from_slice(our_next_block);
                    comm.append(&[LedgerToHostCmd::GetChunk as u8]);
                    comm.append(&block_state.requested_block);
                    trace!("Requesting next block from host");
                    return Ok(());
                }
                // Didn't consume the whole chunk; reset and error message.
                Err((None, _)) => {
                    reset_parsers_state(states);
                    return Err(io::StatusWords::Unknown.into());
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
                    return Ok(());
                }
                // Parse ended before the chunk did; reset.
                Ok(_) => {
                    reset_parsers_state(states);
                    return Err(io::StatusWords::Unknown.into());
                }
            }

        }
        _ => Err(io::StatusWords::Unknown.into()),
    }

}

// fn handle_apdu<P: for<'a> FnMut(ParserTag, &'a [u8]) -> RX<'a, ArrayVec<u8, 260> > >(comm: &mut io::Comm, ins: Ins, parser: &mut P) -> Result<(), Reply> {
#[inline(never)]
fn handle_apdu(comm: &mut io::Comm, ins: Ins, parser: &mut ParsersState, block_state: &mut BlockState) -> Result<(), Reply> {
    info!("entering handle_apdu with command {:?}", ins);
    if comm.rx == 0 {
        return Err(io::StatusWords::NothingReceived.into());
    }

    match ins {
        Ins::GetVersion => {
            comm.append(&[env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap(), env!("CARGO_PKG_VERSION_MINOR").parse().unwrap(), env!("CARGO_PKG_VERSION_PATCH").parse().unwrap()]);
            comm.append(b"Pocket");
        }
        Ins::GetPubkey => {
            run_parser_apdu(parser, get_get_address_state, block_state, &[0], &GET_ADDRESS_IMPL, comm)?
        }
        Ins::Sign => {
            run_parser_apdu(parser, get_sign_state, block_state, &SIGN_SEQ, &SIGN_IMPL, comm)?
        }
        Ins::GetVersionStr => {
            comm.append(concat!("Pocket ", env!("CARGO_PKG_VERSION")).as_ref());
        }
        Ins::Exit => nanos_sdk::exit_app(0),
    }
    Ok(())
}
