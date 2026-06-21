{
  description = "Nix flake for bldhnd — builds CLI and server, and provides a NixOS module";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-23.11";
  };

  outputs = { self, nixpkgs }: let
    system = "x86_64-linux";
    pkgs = import nixpkgs { inherit system; };
  in
  {
    packages.${system} = {
      cli = pkgs.rustPlatform.buildRustPackage {
        pname = "bldhnd-cli";
        version = "0.1.0";
        src = ./app;
      };

      server = pkgs.rustPlatform.buildRustPackage {
        pname = "bldhnd-server";
        version = "0.1.0";
        src = ./server;
      };
    };

    nixosModules.bldhnd = import ./nixos/bldhnd.nix;
  }
