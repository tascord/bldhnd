{
  description = "Nix flake for bldhnd — builds CLI and server, and provides a NixOS module";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    crane = {
      url = "github:ipetkov/crane?ref=v0.23.4";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      crane,
    }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
      craneLib = crane.mkLib pkgs;
      src = ./.;
      commonArgs = {
        pname = "bldhnd-workspace";
        version = "0.1.0";
        inherit src;
        buildInputs = [ pkgs.pkg-config pkgs.openssl ];
      };
      fz = craneLib.buildPackage (
        commonArgs
        // {
          pname = "fz";
          cargoExtraArgs = "-p fz";
          doCheck = false;
        }
      );

      cli = craneLib.buildPackage (
        commonArgs
        // {
          pname = "bldhnd";
          cargoExtraArgs = "-p bldhnd";
          doCheck = false;
        }
      );

      bh-server = craneLib.buildPackage (
        commonArgs
        // {
          pname = "bh-server";
          cargoExtraArgs = "-p bh-server";
          doCheck = false;
        }
      );

      bh-service = craneLib.buildPackage (
        commonArgs
        // {
          pname = "bh-service";
          cargoExtraArgs = "-p bh-service";
          doCheck = false;
        }
      );

      bldhnd = craneLib.buildPackage (
        commonArgs
        // {
          pname = "bldhnd-tui";
          cargoExtraArgs = "-p bldhnd";
          doCheck = false;
        }
      );
    in
    {
      packages.${system} = {
        inherit
          fz
          cli
          bh-server
          bh-service
          bldhnd
          ;
        default = cli;
      };

      checks.${system} = {
        inherit
          fz
          cli
          bh-server
          bh-service
          bldhnd
          ;
      };

      nixosModules.bldhnd = import ./nixos/bldhnd.nix;
    };
}