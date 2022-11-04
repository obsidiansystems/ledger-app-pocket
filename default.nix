rec {
  alamgu = import ./dep/alamgu {};

  inherit (alamgu)
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
              preHook = alamgu.gccLibsPreHook;
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
          alamgu.ledgerCore
          alamgu.ledgerCompilerBuiltins
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
      alamgu.cargo-ledger
      alamgu.ledgerRustPlatform.rust.cargo
    ];
  } (alamgu.cargoLedgerPreHook + ''

    cp ${./rust-app/Cargo.toml} ./Cargo.toml
    # So cargo knows it's a binary
    mkdir src
    touch src/main.rs

    RUSTC_BOOTSTRAP=1 cargo-ledger --use-prebuilt ${rootCrate}/bin/pocket --hex-next-to-json

    mkdir -p $out/pocket
    cp app.json app.hex $out/pocket
    cp ${rootCrate}/bin/pocket $out/pocket/app.elf
    cp ${./tarball-default.nix} $out/pocket/default.nix
    cp ${./tarball-shell.nix} $out/pocket/shell.nix
    cp ${./rust-app/pocket.gif} $out/pocket/pocket.gif
  '');

  tarball = pkgs.runCommandNoCC "app-tarball.tar.gz" { } ''
    tar -czvhf $out -C ${tarSrc} pocket
  '';

  loadApp = pkgs.writeScriptBin "load-app" ''
  #!/usr/bin/env bash
    cd ${tarSrc}/pocket
    ${alamgu.ledgerctl}/bin/ledgerctl install -f ${tarSrc}/pocket/app.json
  '';

  appShell = pkgs.mkShell {
    packages = [ loadApp alamgu.generic-cli pkgs.jq pocket-core pocket-cli-cmd-renamed ];
  };

  testPackage = (import ./ts-tests/override.nix { inherit pkgs; }).package;

  testScript = pkgs.writeShellScriptBin "mocha-wrapper" ''
    cd ${testPackage}/lib/node_modules/*/
    export NO_UPDATE_NOTIFIER=true
    exec ${pkgs.nodejs-14_x}/bin/npm --offline test -- "$@"
  '';

  runTests = { appExe ? rootCrate + "/bin/pocket" }: pkgs.runCommandNoCC "run-tests" {
    nativeBuildInputs = [
      pkgs.wget alamgu.speculos.speculos testScript
    ];
  } ''
    mkdir $out
    (
    speculos -k 2.0 ${appExe} --display headless &
    SPECULOS=$!

    until wget -O/dev/null -o/dev/null http://localhost:5000; do sleep 0.1; done;
    sleep 1;

    ${testScript}/bin/mocha-wrapper
    rv=$?
    echo "Finished tests"
    kill -9 $SPECULOS
    exit $rv) | tee $out/short |& tee $out/full &
    TESTS=$!
    (sleep 2m; kill $TESTS) &
    TESTKILLER=$!
    wait $TESTS
    rv=$?
    kill $TESTKILLER
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
    patches = [ ./pocket-core.patch ];
    vendorSha256 = "04rwxmmk2za27ylyxidd499bb2c0ssrishgnfnq7wm6f1b99vbs0";
    doCheck = false;
  };
  pocket-cli-cmd-renamed = pkgs.linkFarm "pocket-cmd" [ {
    name = "bin/pocket";
    path = "${pocket-core}/bin/pocket_core";
  } ];
}
