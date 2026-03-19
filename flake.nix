{
  description = "sauna";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
      in
      {
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            nodejs_20
            gum
            figlet
            rustup
            cargo
            probe-rs-tools
            prek
            pkgs.pkgsCross.aarch64-multiplatform-musl.stdenv.cc
          ];

          shellHook = ''
            printf "\033[34m"
            cat << 'EOF'
                    ┌─────┐
                    │ FUEL│
                    │ $$$ │
                    ├─────┤
                    │     │▒
                    │     ┝━┓
                    │     │ ┃
                    └──┬──┘ ┃
                    ════╧════╝
            EOF
            printf "\033[0m"
            gum style --align center --border normal \
            --border-foreground 4 --foreground 3 \
            "$(figlet FUEL LOGGER | head -n-1)"
            export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER=rust-lld
          '';
        };

      });
}
