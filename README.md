# Pocket Network Nano S, Nano S+ and Nano X Application

An application for signing Pocket Network transactions.

## Device Compatability

This application is compatible with
- Ledger Nano S, running firmware 2.1.0 and above
- Ledger Nano S+, running firmware 1.0.3
- Ledger Nano X

Note: Compatibility with Ledger Nano X is only possible to check on [Speculos](https://github.com/ledgerHQ/speculos/) emulator,
because the Nano X does not support side-loading apps under development.

## Installing the app

If you don't want to develop the app but just use it, installation should be very simple.
The first step is to obtain a release tarball.
The second step is to load that app from the tarball.

### Obtaining a release tarball

#### Download an official build

Check the [releases page](https://github.com/alamgu/alamgu-example/releases) of this app to see if an official build has been uploaded for this release.
There is a separate tarball for each device.

#### Build one yourself, with Nix

There is a separate tarball for each device.
To build one, run:
```bash
nix-build -A $DEVICE.tarball
```
where `DEVICE` is one of
 - `nanos`, for Nano S
 - `nanox`, for Nano X
 - `nanosplus`, for Nano S+

The last line printed out will be the path of the tarball.

### Installation using the pre-packaged tarball

Before installing please ensure that your device is plugged, unlocked, and on the device home screen.
Installing the app from a tarball can be done using [`ledgerctl`](https://github.com/ledgerHQ/ledgerctl).

#### With Nix

By using Nix, this can be done simply by using the `load-app` command, without manually installing the `ledgerctl` on your system.

```bash
tar xzf release.tar.gz
cd rust-app
nix-shell
load-app
```

#### Without Nix

Without using Nix, the `ledgerctl` can be used directly to install the app with the following commands.
For more information on how to install and use that tool see the [instructions from LedgerHQ](https://github.com/LedgerHQ/ledgerctl).

```bash
tar xzf release.tar.gz
cd rust-app
ledgerctl install -f app.json
```

## Using the app with generic CLI tool

The bundled `generic-cli` tool can be used to obtaining the public key and do signing.

To use this tool using Nix, from the root level of this repo, run this command to enter a shell with all the tools you'll need:
```bash
nix-shell -A $DEVICE.appShell
```
where `DEVICE` is one of
 - `nanos`, for Nano S
 - `nanox`, for Nano X
 - `nanosplus`, for Nano S+

Then, one can use `generic-cli` like this:

- Get a public key for a BIP-32 derivation:
  ```shell-session
  $ generic-cli getAddress --useBlock "44'/635'/0'/0/0" --json
  {
    "publicKey": "3f903a00b0b9634b61de1fedee53dcd02ef6c94ec63529a20565b17f306ff0a9",
    "address": "e8ed4e23ebb4d59444fa8fe1a0e3a1171dfe6af2"
  }
  ```

- Sign a transaction:
  ```shell-session
  $ generic-cli sign --useBlock "44'/635'/0'/0/0" --json '{"chain_id":"testnet","entropy":"-7780543831205109370","fee":[{"amount":"10000","denom":"upokt"}],"memo":"","msg":{"type":"pos/Send","value":{"amount":"1000000","from_address":"51568b979c4c017735a743e289dd862987143290","to_address":"51568b979c4c017735a743e289dd862987143290"}}}'
  Signing:  <Buffer 7b 22 63 68 61 69 6e 5f 69 64 22 3a 22 74 65 73 74 6e 65 74 22 2c 22 65 6e 74 72 6f 70 79 22 3a 22 2d 37 37 38 30 35 34 33 38 33 31 32 30 35 31 30 39 ... 227 more bytes>
  0217976c898df122bf71fe3f29fb9f5d61e6ea26f0fd327c89ea5a754df843e1044a5a6aad50df9ab7fd8a3a1ed78083b2925ab973168ed25c8556b7fcf3e500
  ```

The exact output you see will vary, since Ledger devices should not be configured to have the same private key!

the `--useBlock` argument to generic-cli is required for the pocket app to select the correct ledger/host protocol. Producing a transaction to sign, and assembling the resulting ed25519 signature with the transaction to send, are done with the pocket commandline.

## Development

See [CONTRIBUTING.md](./CONTRIBUTING.md).
