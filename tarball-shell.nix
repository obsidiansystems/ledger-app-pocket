let
  alamgu = import (fetchTarball "https://github.com/obsidiansystems/alamgu/archive/develop.tar.gz") {};
  pkgs = alamgu.pkgs;
  load-app = import ./.;
in
  pkgs.mkShell {
    buildInputs = [load-app];
  }
