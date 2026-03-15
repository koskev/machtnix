_: {
  perSystem =
    {
      pkgs,
      ...
    }:
    {
      devShells = {
        clippy = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            cargo
            clippy
            gnumake
          ];
          buildInputs = with pkgs; [
            rustc
          ];
          RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
          LIBCLANG_PATH = with pkgs; "${llvmPackages.libclang.lib}/lib";
        };

        # For `nix develop`:
        default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            cargo
            gdb
            cargo-tarpaulin
            clippy
            rustfmt
            cargo2junit

            rust-analyzer
            bacon
            tracy
            reuse

            conform
            prek
            gnumake
            git-cliff
            pkg-config

          ];
          buildInputs = with pkgs; [
            rustc
            nix
            boost
            stdenv.cc.libc.dev
          ];
          RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
          LIBCLANG_PATH = with pkgs; "${llvmPackages.libclang.lib}/lib";

          BINDGEN_EXTRA_CLANG_ARGS = "-isystem ${pkgs.stdenv.cc.libc.dev}/include";
          C_INCLUDE_PATH = "${pkgs.stdenv.cc.libc.dev}/include";
        };
      };
    };
}
