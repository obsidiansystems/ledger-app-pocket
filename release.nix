let self = import ./.;
in
  {
    inherit (self) tarball loadApp pocket-core test;
    generic-cli = self.ledger-platform.generic-cli;
  }
