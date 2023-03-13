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

Check the [releases page](https://github.com/alamgu/pocket/releases) of this app to see if an official build has been uploaded for this release.
There is a separate tarball for each device.

#### Build one yourself, with Nix

##### Set up build caches (optional)

In addition to this app itself, the entire toolchain is packaged from source with Nix.
That means that with usuing a pre-populated source of build artifacts, the first build will take a **very long time** as everything specific to Alamgu is built from source.
(Other packages could also be built from source, but Nix by default ships configured to use the official `cache.nixos.org` build artifacts cache.)

If you are comfortable trusting Obsidian Systems's build farm, you can use our public open source cache for this purpose:

  - Store URL: `s3://obsidian-open-source`
  - Public key (for build artifact signatures): `obsidian-open-source:KP1UbL7OIibSjFo9/2tiHCYLm/gJMfy8Tim7+7P4o0I=`

To do this:

1. First you want to include these two in your `/etc/nix/nix.conf` settings file.
   After doing so, it should have two lines like this:
   ```
   substituters = https://cache.nixos.org/ s3://obsidian-open-source
   trusted-public-keys = cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY= obsidian-open-source:KP1UbL7OIibSjFo9/2tiHCYLm/gJMfy8Tim7+7P4o0I=
   ```
   (The new values are each appended at the end of a space-separated list.)

2. After updating that file, you probably need to restart your Nix daemon:

   - On macOS:
     - `sudo launchctl stop org.nixos.nix-daemon`
     - `sudo launchctl start org.nixos.nix-daemon`
   - On Linux:
     - `sudo systemctl stop nix-daemon`
     - `sudo systemctl start nix-daemon`

(On NixOS these tasks are done differently, consult the NixOS documentation for how to update your system configuration which includes these settings and will restart the daemon.)

##### Building

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

### Preparing Your Linux Machine for Ledger Device Communication

On Linux, the "udev" rules must be set up to allow your user to communicate with the ledger device. MacOS devices do not need any configuration to communicate with a Ledger device, so if you are using Mac you can ignore this section.

#### NixOS

On NixOS, one can easily do this with by adding the following to configuration.nix:

``` nix
{
  # ...
  hardware.ledger.enable = true;
  # ...
}
```

#### Non-NixOS Linux Distros

For non-NixOS Linux distros, LedgerHQ provides a [script](https://raw.githubusercontent.com/LedgerHQ/udev-rules/master/add_udev_rules.sh) for this purpose, in its own [specialized repo](https://github.com/LedgerHQ/udev-rules). Download this script, read it, customize it, and run it as root:

```shell
wget https://raw.githubusercontent.com/LedgerHQ/udev-rules/master/add_udev_rules.sh
chmod +x add_udev_rules.sh
```

**We recommend against running the next command without reviewing the script** and modifying it to match your configuration.

```shell
sudo ./add_udev_rules.sh
```

Subsequently, unplug your ledger hardware wallet, and plug it in again for the changes to take effect.

For more details, see [Ledger's documentation](https://support.ledger.com/hc/en-us/articles/115005165269-Fix-connection-issues).

### Installation using the pre-packaged tarball

Before installing please ensure that your device is plugged, unlocked, and on the device home screen.
Installing the app from a tarball can be done using [`ledgerctl`](https://github.com/ledgerHQ/ledgerctl).

#### With Nix

By using Nix, this can be done simply by using the `load-app` command, without manually installing the `ledgerctl` on your system.

```bash
tar xzf /path/to/release.tar.gz
cd pocket-$DEVICE
nix-shell
load-app
```

`/path/to/release.tar.gz` you should replace with the actual path to the tarball.
For example, it might be `~/Downloads/release.tar.gz` if you downloaded a pre-built official release from GitHub, or `/nix/store/adsfijadslifjaslif-release.tar.gz` if you built it yourself with Nix.

#### Without Nix

Without using Nix, the `ledgerctl` can be used directly to install the app with the following commands.
For more information on how to install and use that tool see the [instructions from LedgerHQ](https://github.com/LedgerHQ/ledgerctl).

```bash
tar xzf release.tar.gz
cd pocket-$DEVICE
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
