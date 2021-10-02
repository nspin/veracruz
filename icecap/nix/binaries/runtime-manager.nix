{ lib, stdenv, hostPlatform, buildPackages, mkShell
, rustc, cargo, git, cacert
, crateUtils, nixToToml
, protobuf, perl
, liboutline, sysroot-rs
, icecapCratesAttrs
}:

let

  name = "runtime-manager";

  manifestPath = toString ../../.. + "/${name}/Cargo.toml";

  debug = false;

  cargoConfig = nixToToml (crateUtils.clobber [
    crateUtils.baseCargoConfig
    { profile.release.panic = "abort"; }
    {
      target.${hostPlatform.config} = crateUtils.clobber (map (crate:
      if crate.buildScript == null then {} else {
        ${"dummy-link-${crate.name}"} = crate.buildScript;
      }) (lib.attrValues icecapCratesAttrs));
    }
    {
      target.${hostPlatform.config}.rustflags = [ "--sysroot=${sysroot-rs}" ];
    }
  ]);

in

mkShell (crateUtils.baseEnv // {

  depsBuildBuild = [
    buildPackages.stdenv.cc
  ];

  nativeBuildInputs = [
    rustc cargo git cacert
    protobuf perl
  ];

  buildInputs = [
    liboutline
  ];

  shellHook = ''
    build_dir=build/${name}

    build() {
      setup && \
      (cd $build_dir && cargo build \
         -Z unstable-options \
        --manifest-path ${manifestPath} \
        --target ${hostPlatform.config} --features icecap \
        ${lib.optionalString (!debug) "--release"} \
        --target-dir ./target \
        --out-dir ./out \
        -j $NIX_BUILD_CORES \
        $@
      )
    }

    setup() {
      mkdir -p $build_dir/.cargo
      ln -sf ${cargoConfig} $build_dir/.cargo/config
    }
  '';

})
