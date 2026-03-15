{ inputs, self, ... }:
{
  perSystem =
    {
      pkgs,
      ...
    }:
    let
      craneLib = inputs.crane.mkLib pkgs;
    in
    {
      packages.default = craneLib.buildPackage {
        name = "machtnix";
        src = self;
        nativeBuildInputs = with pkgs; [
          pkg-config
        ];
        buildInputs = with pkgs; [
          nix
          stdenv.cc.libc.dev
        ];
        RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
        LIBCLANG_PATH = with pkgs; "${llvmPackages.libclang.lib}/lib";

        BINDGEN_EXTRA_CLANG_ARGS = "-isystem ${pkgs.stdenv.cc.libc.dev}/include";
        C_INCLUDE_PATH = "${pkgs.stdenv.cc.libc.dev}/include";
      };
    };
}
