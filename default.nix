rec {
  ledger-platform = import ./dep/ledger-platform {};

  inherit (ledger-platform)
    lib
    pkgs ledgerPkgs
    crate2nix
    buildRustCrateForPkgsLedger
    buildRustCrateForPkgsWrapper
    ;

  makeApp = { rootFeatures ? [ "default" ], release ? true }: import ./Cargo.nix {
    inherit rootFeatures release;
    pkgs = ledgerPkgs;
    buildRustCrateForPkgs = pkgs: let
      fun = buildRustCrateForPkgsWrapper
        pkgs
        ((buildRustCrateForPkgsLedger pkgs).override {
          defaultCrateOverrides = pkgs.defaultCrateOverrides // {
            pocket = attrs: let
              sdk = lib.findFirst (p: lib.hasPrefix "rust_nanos_sdk" p.name) (builtins.throw "no sdk!") attrs.dependencies;
            in {
              preHook = ledger-platform.gccLibsPreHook;
              extraRustcOpts = attrs.extraRustcOpts or [] ++ [
                "-C" "link-arg=-T${sdk.lib}/lib/nanos_sdk.out/script.ld"
                "-C" "linker=${pkgs.stdenv.cc.targetPrefix}clang"
              ];
            };
          };
        });
    in
      args: fun (args // lib.optionalAttrs pkgs.stdenv.hostPlatform.isAarch32 {
        dependencies = map (d: d // { stdlib = true; }) [
          ledger-platform.ledgerCore
          ledger-platform.ledgerCompilerBuiltins
        ] ++ args.dependencies;
      });
  };

  app = makeApp {};
  app-with-logging = makeApp {
    release = false;
    rootFeatures = [ "default" "speculos" "extra_debug" ];
  };

  # For CI
  rootCrate = app.rootCrate.build;
  rootCrate-with-logging = app-with-logging.rootCrate.build;

  tarSrc = ledgerPkgs.runCommandCC "tarSrc" {
    nativeBuildInputs = [
      ledger-platform.cargo-ledger
      ledger-platform.ledgerRustPlatform.rust.cargo
    ];
  } (ledger-platform.cargoLedgerPreHook + ''

    cp ${./rust-app/Cargo.toml} ./Cargo.toml
    # So cargo knows it's a binary
    mkdir src
    touch src/main.rs

    RUSTC_BOOTSTRAP=1 cargo-ledger --use-prebuilt ${rootCrate}/bin/pocket --hex-next-to-json

    mkdir -p $out/pocket
    cp app.json app.hex $out/pocket
    cp ${./tarball-default.nix} $out/pocket/default.nix
    cp ${./rust-app/pocket.gif} $out/pocket/pocket.gif
  '');

  tarball = pkgs.runCommandNoCC "app-tarball.tar.gz" { } ''
    tar -czvhf $out -C ${tarSrc} pocket
  '';

  loadApp = pkgs.writeScriptBin "load-app" ''
  #!/usr/bin/env bash
    cd ${tarSrc}/pocket
    ${ledger-platform.ledgerctl}/bin/ledgerctl install -f ${tarSrc}/pocket/app.json
  '';

  appShell = pkgs.mkShell {
    packages = [ loadApp ledger-platform.generic-cli ];
  };

  testPackage = (import ./ts-tests/override.nix { inherit pkgs; }).package;

  testScript = pkgs.writeShellScriptBin "mocha-wrapper" ''
    cd ${testPackage}/lib/node_modules/*/
    export NO_UPDATE_NOTIFIER=true
    exec ${pkgs.nodejs-14_x}/bin/npm --offline test -- "$@"
  '';

  runTests = { appExe ? rootCrate + "/bin/pocket" }: pkgs.runCommandNoCC "run-tests" {
    nativeBuildInputs = [
      pkgs.wget ledger-platform.speculos.speculos testScript
    ];
  } ''
    RUST_APP=${rootCrate}/bin/*
    echo RUST APP IS $RUST_APP
    # speculos -k 2.0 $RUST_APP --display headless &
    mkdir $out
    (
    speculos -k 2.0 ${appExe} --display headless &
    SPECULOS=$!

    until wget -O/dev/null -o/dev/null http://localhost:5000; do sleep 0.1; done;

    ${testScript}/bin/mocha-wrapper
    rv=$?
    kill -9 $SPECULOS
    exit $rv) | tee $out/short |& tee $out/full
    rv=$?
    cat $out/short
    exit $rv
  '';

  # test-with-loging = runTests {
  #   appExe = rootCrate-with-logging + "/bin/pocket";
  # };
  test = runTests {
    appExe = rootCrate + "/bin/pocket";
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
    vendorSha256 = "04rwxmmk2za27ylyxidd499bb2c0ssrishgnfnq7wm6f1b99vbs0";
    doCheck = false;
  };
}
