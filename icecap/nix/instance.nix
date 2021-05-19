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
  host2Stage = false;

in
mkInstance (self: with self; {

  payload = uBoot.${icecapPlat}.mkDefaultPayload {
    dtb = composition.host-dtb;
    linuxImage = linuxKernel.host.${icecapPlat}.kernel;
    initramfs = (if host2Stage then nx2Stage else nx1Stage).config.build.initramfs;
    bootargs = [
      "earlycon=icecap_vmm"
      "console=hvc0"
      "loglevel=7"
    ] ++ (if host2Stage then [
      "spec=${spec}"
      "test_collateral=${testCollateral}"
    ] else [
      "next_init=${nx2Stage.config.build.nextInit}"
    ]);
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
        runtime_manager.image = stripElfSplit runtimeManagerEnclaveElf;
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

  # TODO
  # db = callPackage ./test-database.nix {};
  db = ../../veracruz-server-test/proxy-attestation-server.db;

  testCollateral = runCommand "test-collateral" {
    nativeBuildInputs = [ nukeReferences ];
  } ''
    cp -r --no-preserve=mode,ownership ${testCollateralRaw} $out
    find $out -type d -empty -delete
    nuke-refs $out
  '';

  testCollateralRaw = lib.cleanSourceWith {
    src = lib.cleanSource ../veracruz/test-collateral;
    filter = name: type: type == "directory" || lib.any (pattern: builtins.match pattern name != null) [
      ".*\\.json"
      ".*\\.pem"
      ".*\\.wasm"
      ".*\\.dat"
    ];
  };

  test2Stage = pkgs_linux.writeScript "test.sh" ''
    #!${pkgs_linux.runtimeShell}
    cd /x
    ln -sf ${testCollateral} /test-collateral
    RUST_LOG=debug \
    DATABASE_URL=proxy-attestation-server.db \
    VERACRUZ_RESOURCE_SERVER_ENDPOINT=file:/dev/rb_resource_server \
    VERACRUZ_REALM_ID=0 \
    VERACRUZ_REALM_SPEC=${spec} \
    VERACRUZ_REALM_ENDPOINT=/dev/rb_realm \
      ${veracruzServerTestElf} --test-threads=1 "$@"
  '';

})
