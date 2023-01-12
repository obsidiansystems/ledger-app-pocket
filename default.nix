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
            nanos_sdk = attrs: {
              passthru = (attrs.passthru or {}) // {
                link_wrap = pkgs.buildPackages.stdenvNoCC.mkDerivation {
                  name = "alamgu-linker-wrapper";
                  dontUnpack = true;
                  dontBuild = true;
                  installPhase = ''
                    mkdir -p "$out/bin"
                    cp "${attrs.src}/scripts/link_wrap.sh" "$out/bin"
                    chmod +x "$out/bin/link_wrap.sh"
                  '';
                };
              };
            };
            ${appName} = attrs: let
              sdk = lib.findFirst (p: lib.hasPrefix "rust_nanos_sdk" p.name) (builtins.throw "no sdk!") attrs.dependencies;
            in {
              preHook = collection.gccLibsPreHook;
              extraRustcOpts = attrs.extraRustcOpts or [] ++ [
                "-C" "linker=${sdk.link_wrap}/bin/link_wrap.sh"
                "-C" "link-arg=-T${sdk.lib}/lib/nanos_sdk.out/link.ld"
                "-C" "link-arg=-T${sdk.lib}/lib/nanos_sdk.out/${device}_layout.ld"
              ];
              passthru = (attrs.passthru or {}) // { inherit sdk; };
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

  apiPort = 5005;

  runTests = { appExe, device, variant ? "", speculosCmd }:
  pkgs.runCommandNoCC "run-tests-${device}${variant}" {
    nativeBuildInputs = [
      pkgs.wget alamgu.speculos.speculos testScript
    ];
  } ''
    mkdir $out
    (
    ${toString speculosCmd} ${appExe} --display headless &
    SPECULOS=$!

    until wget -O/dev/null -o/dev/null http://localhost:${toString apiPort}; do sleep 0.1; done;

    ${testScript}/bin/mocha-wrapper
    rv=$?
    kill -9 $SPECULOS
    exit $rv) | tee $out/short |& tee $out/full &
    TESTS=$!
    (sleep 3m; kill $TESTS) &
    TESTKILLER=$!
    wait $TESTS
    rv=$?
    kill $TESTKILLER
    cat $out/short
    exit $rv
  '';

  makeStackCheck = { rootCrate, device, memLimit, variant ? "" }:
  pkgs.runCommandNoCC "stack-check-${device}${variant}" {
    nativeBuildInputs = [ alamgu.stack-sizes ];
  } ''
    stack-sizes --mem-limit=${toString memLimit} ${rootCrate}/bin/${appName} ${rootCrate}/bin/*.o | tee $out
  '';

  appForDevice = device: rec {
    app = makeApp { inherit device; };
    app-with-logging = makeApp {
      inherit device;
      release = false;
      rootFeatures = [ "default" "speculos" "extra_debug" ];
    };

    memLimit = {
      nanos = 4500;
      nanosplus = 400000;
      nanox = 400000;
    }.${device} or (throw "Unknown target device: `${device}'");

    stack-check = makeStackCheck { inherit memLimit rootCrate device; };
    stack-check-with-logging = makeStackCheck {
      inherit memLimit device;
      rootCrate = rootCrate-with-logging;
      variant = "-with-logging";
    };

    rootCrate = app.rootCrate.build;
    rootCrate-with-logging = app-with-logging.rootCrate.build;

    appExe = rootCrate + "/bin/" + appName;

    rustShell = alamgu.perDevice.${device}.rustShell.overrideAttrs (old: {
      nativeBuildInputs = old.nativeBuildInputs ++ [ rootCrate.sdk.link_wrap ];
    });

    tarSrc = makeTarSrc { inherit appExe device; };
    tarball = pkgs.runCommandNoCC "app-tarball-${device}.tar.gz" { } ''
      tar -czvhf $out -C ${tarSrc} ${appName}
    '';

    loadApp = pkgs.writeScriptBin "load-app" ''
      #!/usr/bin/env bash
      cd ${tarSrc}/${appName}
      ${alamgu.ledgerctl}/bin/ledgerctl install -f ${tarSrc}/${appName}/app.json
    '';

    tarballShell = import (tarSrc + "/${appName}/shell.nix");

    speculosDeviceFlags = {
      nanos = [ "-m" "nanos" ];
      nanosplus = [ "-m" "nanosp" "-k" "1.0.3" ];
      nanox = [ "-m" "nanox" ];
    }.${device} or (throw "Unknown target device: `${device}'");

    speculosCmd = [
      "speculos"
      "--api-port" (toString apiPort)
    ] ++ speculosDeviceFlags;

    test = runTests { inherit appExe speculosCmd device; };
    test-with-logging = runTests {
      inherit speculosCmd device;
      appExe = rootCrate-with-logging + "/bin/" + appName;
      variant = "-with-logging";
    };

    appShell = pkgs.mkShell {
      packages = [
        alamgu.ledgerctl loadApp alamgu.generic-cli pkgs.jq
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
  pocket-cli-cmd-renamed = pkgs.linkFarm "pocket-cmd" [
    {
      name = "bin/pocket";
      path = "${pocket-core}/bin/pocket_core";
    }
  ];
}
