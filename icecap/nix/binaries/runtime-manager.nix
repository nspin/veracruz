{ lib, hostPlatform
, mkShell
, protobuf, perl
, libs, liboutline, sysroot-rs
, which
, stdenv
, nixToToml
, cargo, git, cacert
, buildPackages
, crateUtils
, icecapCratesAttrs
}:

let


  cargoConfig = crateUtils.clobber [
    crateUtils.baseCargoConfig
    (lib.optionalAttrs (hostPlatform.system == "aarch64-none") { profile.release.panic = "abort"; }) # HACK
    {
      target.${hostPlatform.config} = crateUtils.clobber (map (crate:
      if crate.buildScript == null then {} else {
        ${"dummy-link-${crate.name}"} = crate.buildScript;
      }) (lib.attrValues icecapCratesAttrs));
    }
    {
      target.${hostPlatform.config}.rustflags = [ "--sysroot=${sysroot-rs}" ];
    }
  ];

in

mkShell (crateUtils.baseEnv // {
  NIX_HACK_CARGO_CONFIG = nixToToml cargoConfig;

  nativeBuildInputs = [
    cargo git cacert
    protobuf perl
  ];

  depsBuildBuild = [
    buildPackages.stdenv.cc
  ];

  buildInputs = with libs; [
    liboutline
  ];

  shellHook = ''
    build() {
      cargo build --target ${hostPlatform.config} --features icecap \
        -j $NIX_BUILD_CORES \
        --target-dir build/runtime-manager/target --manifest-path ../runtime-manager/Cargo.toml \
        --out-dir=build/runtime-manager/out -Z unstable-options \
        $@
    }
  '';

})
