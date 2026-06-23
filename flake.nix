{
  description = "Nix flake for bldhnd — builds CLI and server, and provides a NixOS module";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    crane = {
      url = "github:ipetkov/crane?ref=v0.23.4";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, crane }: let
    system = "x86_64-linux";
    pkgs = import nixpkgs { inherit system; };
    craneLib = crane.mkLib pkgs;
    src = ./.;
    commonArgs = {
      pname = "bldhnd-workspace";
      version = "0.1.0";
      inherit src;
    };
    # Build deps-only first (external crates only)
    cargoArtifacts = craneLib.buildDepsOnly commonArgs;

    # fz is built without pre-built artifacts since it's a local path workspace member
    fz = craneLib.buildPackage (commonArgs // {
      pname = "fz";
      cargoExtraArgs = "-p fz";
      doCheck = false;
    });

    cli = craneLib.buildPackage (commonArgs // {
      inherit cargoArtifacts;
      pname = "bldhnd";
      cargoExtraArgs = "-p bldhnd";
      doCheck = false;
    });

    server = craneLib.buildPackage (commonArgs // {
      pname = "bh-server";
      cargoExtraArgs = "-p bh-server -p fz";
      doCheck = false;
    });
  in
  {
    packages.${system} = {
      inherit fz cli server;
      default = cli;
    };

    checks.${system} = {
      inherit fz cli server;
    };

    nixosModules.bldhnd = import ./nixos/bldhnd.nix;
  };
}