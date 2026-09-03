{
  description = "Kalcite compiler and CLI";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
    in {
      packages = forAllSystems (system:
        let pkgs = import nixpkgs { inherit system; };
        in { kalcite = pkgs.rustPlatform.buildRustPackage {
          pname = "kalcite"; version = "0.14.0"; src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          cargoBuildFlags = [ "-p" "kalcite-cli" ];
        }; default = self.packages.${system}.kalcite; });
    };
}
