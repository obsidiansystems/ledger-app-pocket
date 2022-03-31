use pocket::implementation::*;
use pocket::interface::*;
use prompts_ui::RootMenu;
use core::convert::TryFrom;
use ledger_parser_combinators::interp_parser::set_from_thunk;
use crypto_helpers::Ed25519;
use nanos_sdk::io;

nanos_sdk::set_panic!(nanos_sdk::exiting_panic);

use ledger_parser_combinators::interp_parser::OOB;
use pocket::*;

#[cfg(not(test))]
#[no_mangle]
extern "C" fn sample_main() {
    ledger_parser_combinators::interp_parser::print_sp();
    let mut comm = io::Comm::new();
    let mut states = ParsersState::NoState;
    let mut block_state = BlockState::default();

    let mut idle_menu = RootMenu::new([ concat!("Pocket ", env!("CARGO_PKG_VERSION")), "Exit" ]);
    let mut busy_menu = RootMenu::new([ "Working...", "Cancel" ]);

    info!("Pocket app {}", env!("CARGO_PKG_VERSION"));

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

impl From<u8> for Ins {
    fn from(ins: u8) -> Ins {
        match ins {
            0 => Ins::GetVersion,
            2 => Ins::GetPubkey,
            3 => Ins::Sign,
            0xfe => Ins::GetVersionStr,
            0xff => Ins::Exit,
            _ => panic!(),
        }
    }
}

use arrayvec::ArrayVec;
use nanos_sdk::io::Reply;

use ledger_parser_combinators::interp_parser::InterpParser;

#[inline(never)]
fn run_parser_apdu<P: InterpParser<A, Returning = ArrayVec<u8, 128>>, A>(
    states: &mut ParsersState,
    get_state: fn(&mut ParsersState) -> &mut <P as InterpParser<A>>::State,
    parser: &P,
    comm: &mut io::Comm,
) -> Result<(), Reply> {
    let cursor: &[u8] = comm.get_data()?;

    loop {
        trace!("Parsing APDU input: {:?}\n", cursor);
        let mut parse_destination = None;
        let parse_rv = <P as InterpParser<A>>::parse(parser, get_state(states), cursor, &mut parse_destination);
        trace!("Parser result: {:?}\n", parse_rv);
        trace!("Parse destination: {:?}\n", parse_destination);
        match parse_rv {
            // Explicit rejection; reset the parser. Possibly send error message to host?
            Err((Some(OOB::Reject), _)) => {
                reset_parsers_state(states);
                break Err(io::StatusWords::Unknown.into());
            }
            // Deliberately no catch-all on the Err((Some case; we'll get error messages if we
            // add to OOB's out-of-band actions and forget to implement them.
            //
            // Finished the chunk with no further actions pending, but not done.
            Err((None, [])) => break Ok(()),
            // Didn't consume the whole chunk; reset and error message.
            Err((None, _)) => {
                reset_parsers_state(states);
                break Err(io::StatusWords::Unknown.into());
            }
            // Consumed the whole chunk and parser finished; send response.
            Ok([]) => {
                trace!("Parser finished, resetting state\n");
                match parse_destination.as_ref() {
                    Some(rv) => comm.append(&rv[..]),
                    None => break Err(io::StatusWords::Unknown.into()),
                }
                // Parse finished; reset.
                reset_parsers_state(states);
                break Ok(());
            }
            // Parse ended before the chunk did; reset.
            Ok(_) => {
                reset_parsers_state(states);
                break Err(io::StatusWords::Unknown.into());
            }
        }
    }
}
const HASH_LEN: usize = 32;
type SHA256 = [u8; HASH_LEN];

enum BlockStateEnum {
    FirstPassPath,
    FirstPassTxn,
    SecondPassTxn,
    SecondPassPath
}
impl Default for BlockStateEnum {
    fn default() -> BlockStateEnum {
        BlockStateEnum::FirstPassPath
    }
}

// Replace with a proper implementation later; this is just to get enough to do the two-pass for
// Ed25519.
#[derive(Default)]
struct BlockState {
    txn_head: SHA256,
    path_head: SHA256,
    requested_block: SHA256,
    state: BlockStateEnum,
}

#[repr(u8)]
#[derive(Copy, Clone)]
enum LedgerToHostCmd {
    RESULT_ACCUMULATING = 0,
    RESULT_FINAL = 1,
    GET_CHUNK = 2,
    PUT_CHUNK = 3
}

#[repr(u8)]
#[derive(Debug)]
enum HostToLedgerCmd {
    START = 0,
    GET_CHUNK_RESPONSE_SUCCESS = 1,
    GET_CHUNK_RESPONSE_FAILURE = 2,
    PUT_CHUNK_RESPONSE = 3,
    RESULT_ACCUMULATING_RESPONSE = 4
}

impl TryFrom<u8> for HostToLedgerCmd {
    type Error = Reply;
    fn try_from(a: u8) -> Result<HostToLedgerCmd, Reply> {
        match a {
            0 => Ok(HostToLedgerCmd::START),
            1 => Ok(HostToLedgerCmd::GET_CHUNK_RESPONSE_SUCCESS),
            2 => Ok(HostToLedgerCmd::GET_CHUNK_RESPONSE_FAILURE),
            3 => Ok(HostToLedgerCmd::PUT_CHUNK_RESPONSE),
            4 => Ok(HostToLedgerCmd::RESULT_ACCUMULATING_RESPONSE),
            _ => Err(io::StatusWords::Unknown.into()),
        }
    }
}

#[inline(never)]
fn run_parser_apdu_signing<P: InterpParser<A, Returning = ArrayVec<u8,128>>, A>(
    states: &mut ParsersState,
    get_state: fn(&mut ParsersState) -> &mut <P as InterpParser<A>>::State,
    block_state: &mut BlockState,
    parser: &P,
    comm: &mut io::Comm,
) -> Result<(), Reply> {

    trace!("Entered run_parser_apdu_signing");
    let block: &[u8] = comm.get_data()?;

    let host_cmd : HostToLedgerCmd = HostToLedgerCmd::try_from(*block.get(0).ok_or(io::StatusWords::Unknown)?)?;

    trace!("Host cmd: {:?}", host_cmd);
    match host_cmd {
        HostToLedgerCmd::START => {
            trace!("Block len: {}", block.len());
            if block.len() < HASH_LEN*2+1 { return Err(io::StatusWords::Unknown.into()); }
            trace!("Block len: {}", block.len());
            block_state.txn_head.copy_from_slice(&block[1..1+HASH_LEN]);
            block_state.path_head.copy_from_slice(&block[1+HASH_LEN..1+HASH_LEN*2]);
            block_state.requested_block.copy_from_slice(&block_state.txn_head[..]);
            block_state.state = BlockStateEnum::FirstPassPath;
            comm.append(&[LedgerToHostCmd::GET_CHUNK as u8]);
            comm.append(&block_state.path_head);
            Ok(())
        }
        HostToLedgerCmd::GET_CHUNK_RESPONSE_SUCCESS => {
            if block.len() < HASH_LEN+1 { return Err(io::StatusWords::Unknown.into()); }
            // TODO: Important: Verify the hash here!
            //
            //
            let next_block = &block[1..1+HASH_LEN];
            let cursor = &block[1+HASH_LEN..];

            trace!("Parsing APDU input: {:?}\n", cursor);
            let mut parse_destination = None;
            let parse_rv = <P as InterpParser<A>>::parse(parser, get_state(states), cursor, &mut parse_destination);
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
                        match block_state.state {
                            BlockStateEnum::FirstPassPath => {
                                block_state.state = BlockStateEnum::FirstPassTxn;
                                &block_state.txn_head
                            }
                            BlockStateEnum::FirstPassTxn => {
                                block_state.state = BlockStateEnum::SecondPassPath;
                                &block_state.path_head
                            }
                            BlockStateEnum::SecondPassPath => {
                                block_state.state = BlockStateEnum::SecondPassTxn;
                                &block_state.txn_head
                            }
                            BlockStateEnum::SecondPassTxn => {
                                return Err(io::StatusWords::Unknown.into());
                            }
                        }
                    } else {
                        &next_block
                    };
                    trace!("Next block: {:x?}", our_next_block);

                    block_state.requested_block.copy_from_slice(our_next_block);
                    comm.append(&[LedgerToHostCmd::GET_CHUNK as u8]);
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
                            comm.append(&[LedgerToHostCmd::RESULT_FINAL as u8]);
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
            run_parser_apdu::<_, Bip32Key>(parser, get_get_address_state, &GET_ADDRESS_IMPL, comm)?
        }
        Ins::Sign => {
            run_parser_apdu_signing::<_, DoubledSignParameters>(parser, get_sign_state, block_state, &SIGN_IMPL, comm)?
        }
        Ins::GetVersionStr => {
            comm.append(concat!("Pocket ", env!("CARGO_PKG_VERSION")).as_ref());
        }
        Ins::Exit => nanos_sdk::exit_app(0),
    }
    Ok(())
}
