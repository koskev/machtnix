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
          pkgs.rustPlatform.bindgenHook
        ];
        buildInputs = with pkgs; [
          nix
        ];
        RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
        LIBCLANG_PATH = with pkgs; "${llvmPackages.libclang.lib}/lib";
      };
    };
}
