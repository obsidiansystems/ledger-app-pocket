let self = import ./.;
in
  {
    inherit (self) tarball loadApp pocket-core test;
    generic-cli = self.ledger-platform.generic-cli;
    build-docker-recompressed = self.ledger-platform.build-docker-recompressed;
  }
