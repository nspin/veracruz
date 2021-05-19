{ lib, runCommand
, nukeReferences
, icecapPlat
, icecapSrcAbsSplit, crateUtils
, nixosLite, linuxKernel, uBoot
, mkIceDL, mkDynDLSpec, stripElfSplit
, pkgs_dev, pkgs_linux
, mkInstance
, globalCrates
}:

let

in
mkInstance (self: with self; {

  payload = uBoot.${icecapPlat}.mkDefaultPayload {
    dtb = composition.host-dtb;
    linuxImage = linuxKernel.host.${icecapPlat}.kernel;
    # initramfs = nx2Stage.config.build.initramfs;
    initramfs = nx1Stage.config.build.initramfs;
    bootargs = [
      "earlycon=icecap_vmm"
      "console=hvc0"
      "loglevel=7"
      "spec=${spec}"
      "test_collateral=${test-collateral}"
      # "next_init=${nx2Stage.config.build.nextInit}"
    ];
  };

  icecapPlatArgs.rpi4.extraBootPartitionCommands = ''
    ln -s ${spec} $out/spec.bin
    ln -s ${test-collateral} $out/test-collateral
  '';

  nx1Stage = pkgs_linux.nixosLite.mk1Stage {
    modules = [
      (import ./host/1-stage/config.nix {
        inherit icecapPlat;
        instance = self;
      })
    ];
  };

  nx2Stage = pkgs_linux.nixosLite.mk2Stage {
    modules = [
      (import ./host/2-stage/config.nix {
        inherit icecapPlat;
        instance = self;
      })
    ];
  };

  spec = mkDynDLSpec {
    cdl = "${ddl}/icecap.cdl";
    root = "${ddl}/links";
  };

  ddl = mkIceDL {
    src = ./realm/ddl;
    config = {
      components = {
        veracruz.image = stripElfSplit runtimeManagerEnclaveElf;
      };
    };
  };

  icecapCratesAttrs = crateUtils.flatDepsWithRoots (with globalCrates; [
    icecap-core
    icecap-start-generic
    icecap-std-external
    generated-module-hack
  ]);

  icecapCrates = crateUtils.collectLocal (lib.attrValues icecapCratesAttrs);

  runtime-manager = callPackage ./binaries/runtime-manager.nix {};
  veracruz-server-test = pkgs_linux.icecap.callPackage ./binaries/veracruz-server-test.nix {};

  runtimeManagerEnclaveElf = ../build/runtime-manager/out/runtime_manager_enclave.elf;
  veracruzServerTestElf = ../build/veracruz-server-test/out/veracruz-server-test;

  veracruzServerTestElfInContext = runCommand "veracruz-server-test" {} ''
    mkdir -p $out/bin
    cp ${veracruzServerTestElf} $out/bin/veracruz-server-test
  '';

  test-collateral = runCommand "test-collateral" {
    nativeBuildInputs = [ nukeReferences ];
  } ''
    cp -r --no-preserve=mode,ownership ${test-collateral-raw} $out
    find $out -type d -empty -delete
    nuke-refs $out
  '';

  test-collateral-raw = lib.cleanSourceWith {
    src = lib.cleanSource ../veracruz/test-collateral;
    filter = name: type: type == "directory" || lib.any (pattern: builtins.match pattern name != null) [
      ".*\\.json"
      ".*\\.pem"
      ".*\\.wasm"
      ".*\\.dat"
    ];
  };

  t = pkgs_linux.writeScript "x.sh" ''
    #!${pkgs_linux.runtimeShell}
    cd /x
    cp -f ${spec} /spec.bin
    ln -sf ${test-collateral} /test-collateral
    cp -f ${veracruzServerTestElf} v
    RUST_LOG=debug \
    DATABASE_URL=proxy-attestation-server.db \
    VERACRUZ_SERVER_ENDPOINT=/dev/rb_realm \
      v --test-threads=1 "$@"
      # RUST_BACKTRACE=full \
  '';

})
