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
      };
    };
}
