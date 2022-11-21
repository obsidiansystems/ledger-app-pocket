let self = import ./.;
    lib = self.pkgs.lib;
in
  {
    inherit (self) pocket-core;
    generic-cli = self.alamgu.generic-cli;
    #build-docker-recompressed = self.alamgu.build-docker-recompressed;
  }
  // lib.mapAttrs' (n: lib.nameValuePair ("nanos--" + n)) self.nanos
  // lib.mapAttrs' (n: lib.nameValuePair ("nanox--" + n)) self.nanox
  // lib.mapAttrs' (n: lib.nameValuePair ("nanosplus--" + n)) self.nanosplus
