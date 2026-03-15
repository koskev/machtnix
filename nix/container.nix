_: {
  perSystem =
    {
      inputs',
      self',
      ...
    }:
    let
      nix2containerPkgs = inputs'.nix2container.packages;
    in
    {
      packages = {
        dockerImageFull = nix2containerPkgs.nix2container.buildImage {
          name = "machtnix";
          tag = "latest";

          config = {
            Cmd = [ "${self'.packages.default}/bin/machtnix" ];
            Env = [
              "PATH=${self'.packages.default}/bin"
            ];
          };
        };
      };
    };
}
