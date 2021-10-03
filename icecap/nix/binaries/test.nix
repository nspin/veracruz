{ lib, stdenv, hostPlatform, buildPackages, mkShell
, rustc, cargo, git, cacert
, crateUtils, nixToToml
, pkgconfig, protobuf, perl, python3
, openssl, sqlite
}:

{ name }:

let

  manifestPath = toString ../../.. + "/${name}/Cargo.toml";

  debug = false;

  cargoConfig = nixToToml (crateUtils.clobber [
    crateUtils.baseCargoConfig
  ]);

in

mkShell (crateUtils.baseEnv // rec {

  inherit name;

  PKG_CONFIG_ALLOW_CROSS = 1;
  LIBCLANG_PATH = "${lib.getLib buildPackages.llvmPackages.libclang}/lib";

  hardeningDisable = [ "all" ]; # HACK

  depsBuildBuild = [
    buildPackages.stdenv.cc
  ];

  nativeBuildInputs = [
    rustc cargo git cacert
    pkgconfig protobuf perl python3
  ];

  buildInputs = [
    openssl sqlite
  ];

  shellHook = ''
    # From: https://github.com/NixOS/nixpkgs/blob/1fab95f5190d087e66a3502481e34e15d62090aa/pkgs/applications/networking/browsers/firefox/common.nix#L247-L253
    # Set C flags for bindgen. Bindgen does not invoke $CC directly. Instead it
    # uses LLVM's libclang. To make sure all necessary flags are included we
    # need to look in a few places.
    export BINDGEN_EXTRA_CLANG_ARGS=" \
      $(< ${stdenv.cc}/nix-support/libc-crt1-cflags) \
      $(< ${stdenv.cc}/nix-support/libc-cflags) \
      $(< ${stdenv.cc}/nix-support/cc-cflags) \
      $(< ${stdenv.cc}/nix-support/libcxx-cxxflags) \
      ${lib.optionalString stdenv.cc.isClang "-idirafter ${stdenv.cc.cc}/lib/clang/${lib.getVersion stdenv.cc.cc}/include"} \
      ${lib.optionalString stdenv.cc.isGNU "-isystem ${stdenv.cc.cc}/include/c++/${lib.getVersion stdenv.cc.cc} -isystem ${stdenv.cc.cc}/include/c++/${lib.getVersion stdenv.cc.cc}/${stdenv.hostPlatform.config}"} \
      ${lib.optionalString stdenv.cc.isGNU "-isystem ${stdenv.cc.cc}/lib/gcc/${stdenv.hostPlatform.config}/${lib.getVersion stdenv.cc.cc}/include"} \
    "

    build_dir=build/${name}

    build() {
      setup && \
      (cd $build_dir && cargo test --no-run \
        --manifest-path ${manifestPath} \
        --target ${hostPlatform.config} --features icecap \
        ${lib.optionalString (!debug) "--release"} \
        -j $NIX_BUILD_CORES \
        --target-dir ./target \
        "$@"
      ) && \
      distinguish
    }

    setup() {
      mkdir -p $build_dir/.cargo
      ln -sf ${cargoConfig} $build_dir/.cargo/config
    }

    distinguish() {
      d=$build_dir/target/${hostPlatform.config}/${if debug then "debug" else "release"}/deps
      f="$(find $d -executable -type f -printf "%T@ %p\n" \
        | sort -n \
        | tail -n 1 \
        | cut -d ' ' -f 2 \
      )"
      mkdir -p $build_dir/out
      mv "$f" $build_dir/out/${name}
    }
  '';

})
