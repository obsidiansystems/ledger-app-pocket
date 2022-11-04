let self = import ./.;
in
  {
    inherit (self) tarball loadApp pocket-core test;
    generic-cli = self.alamgu.generic-cli;
    build-docker-recompressed = self.alamgu.build-docker-recompressed;
  }
