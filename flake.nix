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
        armv7MuslPkgs = import nixpkgs {
          inherit system;
          crossSystem = {
            config = "armv7l-unknown-linux-musleabihf";
            gcc = { arch = "armv7-a"; fpu = "vfpv3-d16"; };
          };
        };
      in
      {
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            nodejs_22
            gum
            figlet
            rustup
            cargo
            probe-rs-tools
            prek
            pkgs.pkgsCross.aarch64-multiplatform-musl.stdenv.cc
            pkgsCross.aarch64-multiplatform-musl.stdenv.cc
            armv7MuslPkgs.stdenv.cc
            sqlite
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
            export CC_aarch64_unknown_linux_musl=aarch64-unknown-linux-musl-cc
            export AR_aarch64_unknown_linux_musl=aarch64-unknown-linux-musl-ar
            export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER=aarch64-unknown-linux-musl-cc
            export CC_armv7_unknown_linux_musleabihf="${armv7MuslPkgs.stdenv.cc}/bin/${armv7MuslPkgs.stdenv.cc.targetPrefix}cc"
            export AR_armv7_unknown_linux_musleabihf="${armv7MuslPkgs.stdenv.cc.bintools}/bin/${armv7MuslPkgs.stdenv.cc.targetPrefix}ar"
            export CARGO_TARGET_ARMV7_UNKNOWN_LINUX_MUSLEABIHF_LINKER="${armv7MuslPkgs.stdenv.cc}/bin/${armv7MuslPkgs.stdenv.cc.targetPrefix}cc"
          '';
        };

      });
}
