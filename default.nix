rec {
  ledger-platform = import ./dep/ledger-platform {};

  inherit (ledger-platform) pkgs ;
  ledger-app = ledger-platform.ledger-app {
    appName = "pocket";
    appGif = ./rust-app/pocket.gif;
    appToml = ./rust-app/Cargo.toml;
    cargoNix = import ./Cargo.nix;
    testPackage = (import ./ts-tests/override.nix { inherit pkgs; }).package;
  };

  inherit (ledger-app) loadApp tarball test;

  appShell = pkgs.mkShell {
    packages = [ loadApp ledger-platform.generic-cli pkgs.jq pocket-core pocket-cli-cmd-renamed ];
  };

  inherit (pkgs.nodePackages) node2nix;

  pocket-core = pkgs.buildGoModule {
    name = "pocket-core";
    src = pkgs.fetchFromGitHub {
      owner = "pokt-network";
      repo = "pocket-core";
      rev = "98a12e0f1ecb98e40cd2012e081de842daf43e90";
      sha256 = "0h6yl6rv8xkc81gzs1xs1gl6aw5k2xaz63avg0rxbj6nnl7qdr8l";
    };
    patches = [ ./pocket-core.patch ];
    vendorSha256 = "04rwxmmk2za27ylyxidd499bb2c0ssrishgnfnq7wm6f1b99vbs0";
    doCheck = false;
  };
  pocket-cli-cmd-renamed = pkgs.linkFarm "pocket-cmd" [ {
    name = "bin/pocket";
    path = "${pocket-core}/bin/pocket_core";
  } ];
}
