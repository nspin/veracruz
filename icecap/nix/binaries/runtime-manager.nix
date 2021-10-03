{ lib, stdenv, hostPlatform, buildPackages, mkShell
, rustc, cargo, git, cacert
, crateUtils, nixToToml
, protobuf, perl, python3
, liboutline, sysroot-rs
, icecapCrates, fakeLibc
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

  LIBCLANG_PATH = "${lib.getLib buildPackages.llvmPackages.libclang}/lib";

  depsBuildBuild = [
    buildPackages.stdenv.cc
  ];

  nativeBuildInputs = [
    rustc cargo git cacert
    protobuf perl python3
  ];

  buildInputs = [
    liboutline
    fakeLibc
  ];

  NIX_LDFLAGS = [
    "-lfake_libc"
    "-licecap_pure"
    "-licecap_utils"
  ];

  shellHook = ''
    export BINDGEN_EXTRA_CLANG_ARGS="$NIX_CFLAGS_COMPILE"

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
