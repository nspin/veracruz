{ lib, stdenv, hostPlatform, buildPackages, mkShell
, rustc, cargo, git, cacert
, crateUtils, nixToToml
, protobuf, perl
, libs, liboutline, sysroot-rs
, icecapCrates
}:

let

  name = "runtime-manager";

  manifestPath = toString ../../.. + "/${name}/Cargo.toml";

  debug = false;

  cargoConfig = nixToToml (crateUtils.clobber [
    crateUtils.baseCargoConfig
    {
      target.${hostPlatform.config}.rustflags = [ "--sysroot=${sysroot-rs}" ];
    }
    {
      target.${hostPlatform.config} = crateUtils.clobber (lib.forEach icecapCrates (crate:
        lib.optionalAttrs (crate.buildScript != null) {
          ${"dummy-link-${crate.name}"} = crate.buildScript;
        }
      ));
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
    libs.icecap-pure
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
        "$@"
      )
    }

    setup() {
      mkdir -p $build_dir/.cargo
      ln -sf ${cargoConfig} $build_dir/.cargo/config
    }
  '';

})
