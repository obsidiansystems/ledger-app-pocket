# Developing an Alamgu ledger App

## Building the app from source

This application has been packaged up with [Nix](https://nixos.org/).

### Nix/Linux

Using Nix, from the root level of this repo, run:

```bash
nix-shell -A $DEVICE.rustShell
cd rust-app/
cargo-ledger ledger -l $DEVICE
````
where `DEVICE` is one of
 - `nanos` for Nano S
 - `nanox` for Nano X
 - `nanosplus` for Nano S+

Note as described in the main [read-me](./README.md),
it is currently not possible to side-load apps on the on Nano X, so one can only test in the emulator.

The [cargo-ledger](https://github.com/LedgerHQ/cargo-ledger.git) builds, outputs a `hex` file and a manifest file for `ledgerctl`, and loads it on a device in a single `cargo-ledger ledger -l nanos` command in the rust-app folder within app directory.

You do not need to install cargo-ledger outside of the nix-shell.

Before installing, please ensure that your device is plugged, unlocked, and on the device home screen.

## Running tests

Using Nix, from the root level of this repo, run:

```bash
nix-shell -A $DEVICE.rustShell
cd rust-app/
cargo test --target=$DEVICE.json
```

## Manual testing

One can for example use [speculos](https://github.com/LedgerHQ/speculos)

`cargo run --release` defaults to running speculos on the generated binary with the appropriate flags, if `speculos.py` is in your `PATH`.

The test suite can be run with `cargo test` in from the shell provided by nix-shell.

A shell with the generic-cli tool for interacting with ledger apps, a "load-app" command to load the app, and pocket-core on the path can be accessed with the appShell derivation, and generic-cli can be used to interact with the app:

```bash
nix-shell -A $DEVICE.appShell
```
