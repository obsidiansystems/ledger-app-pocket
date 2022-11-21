rec {
  alamgu = import ./dep/alamgu {};

  inherit (alamgu) lib pkgs crate2nix alamguLib;

  appName = "pocket";

  makeApp = { rootFeatures ? [ "default" ], release ? true, device }:
    let collection = alamgu.perDevice.${device};
    in import ./Cargo.nix {
      inherit rootFeatures release;
      pkgs = collection.ledgerPkgs;
      buildRustCrateForPkgs = alamguLib.combineWrappers [
        # The callPackage of `buildRustPackage` overridden with various
        # modified arguemnts.
        (pkgs: (collection.buildRustCrateForPkgsLedger pkgs).override {
          defaultCrateOverrides = pkgs.defaultCrateOverrides // {
            ${appName} = attrs: let
              sdk = lib.findFirst (p: lib.hasPrefix "rust_nanos_sdk" p.name) (builtins.throw "no sdk!") attrs.dependencies;
            in {
              preHook = collection.gccLibsPreHook;
              extraRustcOpts = attrs.extraRustcOpts or [] ++ [
                "-C" "linker=${pkgs.stdenv.cc.targetPrefix}clang"
                "-C" "link-arg=-T${sdk.lib}/lib/nanos_sdk.out/link.ld"
                "-C" "link-arg=-T${sdk.lib}/lib/nanos_sdk.out/${device}_layout.ld"
              ];
            };
          };
        })

        # Default Alamgu wrapper
        alamguLib.extraArgsForAllCrates

        # Another wrapper specific to this app, but applying to all packages
        (pkgs: args: args // lib.optionalAttrs (alamguLib.platformIsBolos pkgs.stdenv.hostPlatform) {
          dependencies = map (d: d // { stdlib = true; }) [
            collection.ledgerCore
            collection.ledgerCompilerBuiltins
          ] ++ args.dependencies;
        })
      ];
  };

  makeTarSrc = { appExe, device }: pkgs.runCommandCC "make-tar-src-${device}" {
    nativeBuildInputs = [
      alamgu.cargo-ledger
      alamgu.ledgerRustPlatform.rust.cargo
    ];
  } (alamgu.cargoLedgerPreHook + ''

    cp ${./rust-app/Cargo.toml} ./Cargo.toml
    # So cargo knows it's a binary
    mkdir src
    touch src/main.rs

    cargo-ledger --use-prebuilt ${appExe} --hex-next-to-json ledger ${device}

    dest=$out/${appName}
    mkdir -p $dest

    # Create a file to indicate what device this is for
    echo ${device} > $dest/device
    cp app_${device}.json $dest/app.json
    cp app.hex $dest
    cp ${./tarball-default.nix} $dest/default.nix
    cp ${./tarball-shell.nix} $dest/shell.nix
    cp ${./rust-app/pocket.gif} $dest/pocket.gif
    cp ${./rust-app/pocket-small.gif} $dest/pocket-small.gif
  '');

  testPackage = (import ./ts-tests/override.nix { inherit pkgs; }).package;

  testScript = pkgs.writeShellScriptBin "mocha-wrapper" ''
    cd ${testPackage}/lib/node_modules/*/
    export NO_UPDATE_NOTIFIER=true
    exec ${pkgs.nodejs-14_x}/bin/npm --offline test -- "$@"
  '';

  runTests = { appExe, device, variant ? "", speculosCmd }: pkgs.runCommandNoCC "run-tests-${device}${variant}" {
    nativeBuildInputs = [
      pkgs.wget alamgu.speculos.speculos testScript
    ];
  } ''
    mkdir $out
    (
    ${speculosCmd} ${appExe} --display headless &
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

  appForDevice = device: rec {
    app = makeApp { inherit device; };
    app-with-logging = makeApp {
      inherit device;
      release = false;
      rootFeatures = [ "default" "speculos" "extra_debug" ];
    };

    stack-check = pkgs.runCommandNoCC "stack-check-${device}" {
      nativeBuildInputs = [ alamgu.stack-sizes ];
    } ''
      stack-sizes ${appExe} ${rootCrate}/bin/*.o | tee $out
    '';

    rootCrate = app.rootCrate.build;
    rootCrate-with-logging = app-with-logging.rootCrate.build;

    appExe = rootCrate + "/bin/" + appName;

    tarSrc = makeTarSrc { inherit appExe device; };
    tarball = pkgs.runCommandNoCC "app-tarball-${device}.tar.gz" { } ''
      tar -czvhf $out -C ${tarSrc} ${appName}
    '';

    loadApp = pkgs.writeScriptBin "load-app" ''
      #!/usr/bin/env bash
      cd ${tarSrc}/${appName}
      ${alamgu.ledgerctl}/bin/ledgerctl install -f ${tarSrc}/${appName}/app.json
    '';

    speculosCmd = {
      nanos = "speculos -m nanos";
      nanosplus = "speculos  -m nanosp -k 1.0.3";
      nanox = "speculos -m nanox";
    }.${device} or (throw "Unknown target device: `${device}'");

    test = runTests { inherit appExe speculosCmd device; };
    # test-with-loging = runTests {
    #   inherit speculosCmd device;
    #   appExe = rootCrate-with-logging + "/bin/" + appName;
    #   variant = "-with-logging";
    # };

    appShell = pkgs.mkShell {
      packages = [
        loadApp alamgu.generic-cli pkgs.jq
        pocket-core pocket-cli-cmd-renamed
      ];
    };
  };

  nanos = appForDevice "nanos";
  nanosplus = appForDevice "nanosplus";
  nanox = appForDevice "nanox";

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
