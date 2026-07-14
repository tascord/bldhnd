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
      # Build deps-only first (external crates only) with vendoring disabled
      # because we have local path deps that aren't in vendor
      cargoArtifacts = craneLib.buildDepsOnly (
        commonArgs
        // {
          vendorSrc = null;
        }
      );

      fz = craneLib.buildPackage (
        commonArgs
        // {
          inherit cargoArtifacts;
          pname = "fz";
          cargoExtraArgs = "-p fz";
          doCheck = false;
        }
      );

      cli = craneLib.buildPackage (
        commonArgs
        // {
          inherit cargoArtifacts;
          pname = "bldhnd";
          cargoExtraArgs = "-p bldhnd";
          doCheck = false;
        }
      );

      server = craneLib.buildPackage (
        commonArgs
        // {
          inherit cargoArtifacts;
          pname = "bh-server";
          cargoExtraArgs = "-p bh-server";
          doCheck = false;
        }
      );

      service-tui = pkgs.symlinkJoin {
        name = "bldhnd-service-tui";
        paths = [
          (craneLib.buildPackage (
            commonArgs
            // {
              inherit cargoArtifacts;
              pname = "bh-service";
              cargoExtraArgs = "-p bh-service";
              doCheck = false;
            }
          ))
          (craneLib.buildPackage (
            commonArgs
            // {
              inherit cargoArtifacts;
              pname = "bldhnd-tui";
              cargoExtraArgs = "-p bldhnd";
              doCheck = false;
            }
          ))
        ];
        meta = {
          description = "bldhnd combined service and TUI";
        };
      };
    in
    {
      packages.${system} = {
        inherit
          fz
          cli
          server
          service-tui
          ;
        default = cli;
      };

      checks.${system} = {
        inherit
          fz
          cli
          server
          service-tui
          ;
      };

      nixosModules.bldhnd = import ./nixos/bldhnd.nix;
    };
}
