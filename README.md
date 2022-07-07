# Pocket Network Nano S Application

An application for signing Pocket Network transactions.

## Building

This application has been packaged up with [Nix](https://nixos.org/).
If you are on Linux and have Nix installed, builds and development environments are one command away.

### Prerequisites

This project requires ledger firmware version: 2.1.0 or greater

Building and installing this app from source currently requires the [nix](https://nixos.org/) package manager and a linux machine to construct the appropriate rust environment. Loading a pre-built tarball can be done with the ledgerctl command.

## Loading

You can use [cargo-ledger](https://github.com/LedgerHQ/cargo-ledger.git) which builds, outputs a `hex` file and a manifest file for `ledgerctl`, and loads it on a device in a single `cargo-ledger load` command in the rust-app folder within app directory.

This application is compatible with Ledger Nano S devices running FW 2.1.0 and above. Before installing, please ensure that your device is plugged, unlocked, and on the device home screen. 

### Nix/Linux

Using Nix, from the root level of this repo, run:
```bash
nix-shell -A ledger-platform.rustShell
cd rust-app/
cargo-ledger load
````
You do not need to install cargo-ledger outside of the nix-shell.

### Using the pre-packaged tarball (any OS)
Installing the app from a tarball can be done using `ledgerctl`. For more information on how to install and use that tool see the [instructions from LedgerHQ](https://github.com/LedgerHQ/ledgerctl).

```bash
tar xzf nano-s-release.tar.gz
cd nano-s-release
ledgerctl install -f app.json
```

alternately, with nix installed,

```bash
nix-shell https://github.com/obsidiansystems/ledger-app-pocket/releases/tag/v0.0.4/release.tar.gz --run load-app
```

will fetch ledgerctl and run the install for release 0.0.4 of the app.

## Testing

One can for example use [speculos](https://github.com/LedgerHQ/speculos)

`cargo run --release` defaults to running speculos on the generated binary with the appropriate flags, if `speculos.py` is in your `PATH`.

The test suite can be run with `cargo test` in from the shell provided by nix-shell.

A shell with the generic-cli tool for interacting with ledger apps, a "load-app" command to load the app, and pocket-core on the path can be accessed with the appShell derivation, and generic-cli can be used to interact with the app:

```bash
nix-shell -A appShell

generic-cli getAddress --useBlock "44'/535348'/0'/0/0" --json

generic-cli sign --useBlock "44'/535348'/0'/0/0" --json '{"chain_id":"testnet","entropy":"-7780543831205109370","fee":[{"amount":"10000","denom":"upokt"}],"memo":"","msg":{"type":"pos/Send","value":{"amount":"1000000","from_address":"51568b979c4c017735a743e289dd862987143290","to_address":"51568b979c4c017735a743e289dd862987143290"}}}'

```

the --useBlock argument to generic-cli is required for the pocket app to select the correct ledger/host protocol. Producing a transaction to sign, and assembling the resulting ed25519 signature with the transaction to send, are done with the pocket commandline.

