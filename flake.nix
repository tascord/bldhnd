{
  description = "Nix flake for bldhnd — builds CLI and server, and provides a NixOS module";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }: let
    system = "x86_64-linux";
    pkgs = import nixpkgs { inherit system; };
  in
  {
    packages.${system} = {
      cli = pkgs.rustPlatform.buildRustPackage {
        pname = "bldhnd";
        version = "0.1.0";
        src = ./app;
        cargoLock = { lockFile = ./Cargo.lock; };
        # Ensure the workspace Cargo.lock is present in the crate source so
        # cargoSetup hooks can validate without requiring a manual copy.
        patchPhase = ''
          if [ -f ${./Cargo.lock} ]; then
            cp ${./Cargo.lock} "$sourceRoot/Cargo.lock" || true
          fi
        '';
      };

      server = pkgs.rustPlatform.buildRustPackage {
        pname = "bh-server";
        version = "0.1.0";
        src = ./server;
        cargoLock = { lockFile = ./Cargo.lock; };
        # Copy workspace Cargo.lock into the server crate so builds succeed
        # when building the crate in isolation.
        patchPhase = ''
          if [ -f ${./Cargo.lock} ]; then
            cp ${./Cargo.lock} "$sourceRoot/Cargo.lock" || true
          fi
        '';
      };
    };

    nixosModules.bldhnd = import ./nixos/bldhnd.nix;
  };
}