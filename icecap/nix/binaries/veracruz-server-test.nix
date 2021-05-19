{ lib, hostPlatform, stdenv
, llvmPackages
, pkgconfig, protobuf, perl, python3
, openssl, sqlite
, mkShell, crateUtils
, buildPackages
, cargo, git, cacert
, nixToToml
}:

let
  libclang = (llvmPackages.libclang.nativeDrv or llvmPackages.libclang).lib;

  cargoConfig = crateUtils.clobber [
    crateUtils.baseCargoConfig
    (lib.optionalAttrs (hostPlatform.system == "aarch64-none") { profile.release.panic = "abort"; }) # HACK
  ];

  # debug = true;
  debug = false;

in

mkShell (crateUtils.baseEnv // {

  NIX_HACK_CARGO_CONFIG = nixToToml cargoConfig;

  nativeBuildInputs = [
    cargo git cacert
    pkgconfig protobuf perl python3
  ];

  depsBuildBuild = [
    buildPackages.stdenv.cc
    libclang
  ];

  buildInputs = [
    openssl sqlite
  ];

  shellHook = ''
    # From: https://github.com/NixOS/nixpkgs/blob/1fab95f5190d087e66a3502481e34e15d62090aa/pkgs/applications/networking/browsers/firefox/common.nix#L247-L253
    # Set C flags for Rust's bindgen program. Unlike ordinary C
    # compilation, bindgen does not invoke $CC directly. Instead it
    # uses LLVM's libclang. To make sure all necessary flags are
    # included we need to look in a few places.
    export BINDGEN_EXTRA_CLANG_ARGS="$(< ${stdenv.cc}/nix-support/libc-crt1-cflags) \
      $(< ${stdenv.cc}/nix-support/libc-cflags) \
      $(< ${stdenv.cc}/nix-support/cc-cflags) \
      $(< ${stdenv.cc}/nix-support/libcxx-cxxflags) \
      ${lib.optionalString stdenv.cc.isClang "-idirafter ${stdenv.cc.cc}/lib/clang/${lib.getVersion stdenv.cc.cc}/include"} \
      ${lib.optionalString stdenv.cc.isGNU "-isystem ${stdenv.cc.cc}/include/c++/${lib.getVersion stdenv.cc.cc} -isystem ${stdenv.cc.cc}/include/c++/${lib.getVersion stdenv.cc.cc}/${stdenv.hostPlatform.config}"} \
      ${lib.optionalString stdenv.cc.isGNU "-isystem ${stdenv.cc.cc}/lib/gcc/${stdenv.hostPlatform.config}/${lib.getVersion stdenv.cc.cc}/include"} \
    "

    build() {
      cargo test --no-run --target ${hostPlatform.config} --features icecap \
        ${lib.optionalString (!debug) "--release"} \
        -j $NIX_BUILD_CORES \
        --target-dir build/veracruz-server-test/target --manifest-path ../veracruz-server-test/Cargo.toml \
        $@ \
      && install
    }

    install() {
      d=build/veracruz-server-test/target/aarch64-unknown-linux-gnu/${if debug then "debug" else "release"}/deps
      f="$(find $d -executable -type f -printf "%T@ %p\n" \
        | sort -n \
        | tail -n 1 \
        | cut -d ' ' -f 2 \
      )"
      mkdir -p build/veracruz-server-test/out
      mv "$f" build/veracruz-server-test/out/veracruz-server-test
    }
  '';

  PKG_CONFIG_ALLOW_CROSS = 1;
  LIBCLANG_PATH = "${libclang}/lib";
  hardeningDisable = [ "all" ];
})
