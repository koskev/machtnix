{ inputs, ... }:
let
  inherit (inputs.nix-actions.lib) steps;
  inherit (inputs.nix-actions.lib) platforms;
in
{
  imports = [ inputs.actions-nix.flakeModules.default ];
  flake.actions-nix = {
    pre-commit.enable = true;
    defaultValues = {
      jobs = {
        runs-on = "ubuntu-latest";
      };
    };
    workflows = {
      ".github/workflows/docker-publish.yaml" = inputs.nix-actions.lib.mkDocker { };
      ".github/workflows/mr.yaml" = inputs.nix-actions.lib.mkConform { };
      ".github/workflows/linting.yaml" = inputs.nix-actions.lib.mkClippy { };
      ".github/workflows/test.yaml" = {
        on = {
          push = { };
          pull_request = { };
        };
        env = {
          CARGO_TERM_COLOR = "always";
        };
        jobs = {
          nix-build = {
            strategy.matrix.platform = [
              platforms.linux
              platforms.linux_aarch64
              platforms.mac
            ];
            runs-on = "\${{ matrix.platform.runs-on }}";
            steps = [
              steps.checkout
              steps.installNix
              {
                name = "Build";
                run = "nix build .";
              }
            ];
          };
        };
      };
    };
  };
}
