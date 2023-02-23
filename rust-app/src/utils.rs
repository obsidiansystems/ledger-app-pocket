use core::char;

/// Convert to hex. Returns a static buffer of 64 bytes
#[inline]
pub fn to_hex(m: &[u8]) -> Result<[u8; 64], ()> {
    if 2 * m.len() > 64 {
        return Err(());
    }
    let mut hex = [0u8; 64];
    let mut i = 0;
    for c in m {
        let c0 = char::from_digit((c >> 4).into(), 16).unwrap();
        let c1 = char::from_digit((c & 0xf).into(), 16).unwrap();
        hex[i] = c0 as u8;
        hex[i + 1] = c1 as u8;
        i += 2;
    }
    Ok(hex)
}

use ledger_prompts_ui::{PromptWrite, ScrollerError};

// A couple type ascription functions to help the compiler along.
pub const fn mkfn<A, B, C>(q: fn(&A, &mut B) -> Option<C>) -> fn(&A, &mut B) -> Option<C> {
    q
}
pub const fn mkmvfn<A, B, C>(q: fn(A, &mut B) -> Option<C>) -> fn(A, &mut B) -> Option<C> {
    q
}
pub const fn mkvfn<A, C>(q: fn(&A, &mut Option<()>) -> C) -> fn(&A, &mut Option<()>) -> C {
    q
}
pub const fn mkfnc<A, B, C>(q: fn(&A, &mut B, C) -> Option<()>) -> fn(&A, &mut B, C) -> Option<()> {
    q
}
/*pub const fn mkbindfn<A,C>(q: fn(&A)->C) -> fn(&A)->C {
  q
}*/
/*
pub const fn mkvfn<A>(q: fn(&A,&mut Option<()>)->Option<()>) -> fn(&A,&mut Option<()>)->Option<()> {
    q
}
*/

#[cfg(not(target_os = "nanos"))]
#[inline(never)]
pub fn scroller<F: for<'b> Fn(&mut PromptWrite<'b, 16>) -> Result<(), ScrollerError>>(
    title: &str,
    prompt_function: F,
) -> Option<()> {
    ledger_prompts_ui::write_scroller_three_rows(false, title, prompt_function)
}

#[cfg(target_os = "nanos")]
#[inline(never)]
pub fn scroller<F: for<'b> Fn(&mut PromptWrite<'b, 16>) -> Result<(), ScrollerError>>(
    title: &str,
    prompt_function: F,
) -> Option<()> {
    ledger_prompts_ui::write_scroller(false, title, prompt_function)
}

#[cfg(not(target_os = "nanos"))]
#[inline(never)]
pub fn scroller_paginated<F: for<'b> Fn(&mut PromptWrite<'b, 16>) -> Result<(), ScrollerError>>(
    title: &str,
    prompt_function: F,
) -> Option<()> {
    ledger_prompts_ui::write_scroller_three_rows(true, title, prompt_function)
}

#[cfg(target_os = "nanos")]
#[inline(never)]
pub fn scroller_paginated<F: for<'b> Fn(&mut PromptWrite<'b, 16>) -> Result<(), ScrollerError>>(
    title: &str,
    prompt_function: F,
) -> Option<()> {
    ledger_prompts_ui::write_scroller(true, title, prompt_function)
}
