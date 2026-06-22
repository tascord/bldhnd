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
    src = craneLib.cleanCargoSource ./.;
    commonArgs = {
      pname = "bldhnd-workspace";
      version = "0.1.0";
      inherit src;
      strictDeps = true;
    };
    cargoArtifacts = craneLib.buildDepsOnly commonArgs;

    cli = craneLib.buildPackage (commonArgs // {
      inherit cargoArtifacts;
      pname = "bldhnd";
      cargoExtraArgs = "-p bldhnd";
      doCheck = false;
    });

    server = craneLib.buildPackage (commonArgs // {
      inherit cargoArtifacts;
      pname = "bh-server";
      cargoExtraArgs = "-p bh-server";
      doCheck = false;
    });
  in
  {
    packages.${system} = {
      inherit cli server;
      default = cli;
    };

    checks.${system} = {
      inherit cli server;
    };

    nixosModules.bldhnd = args: import ./nixos/bldhnd.nix ({ inherit self; } // args);
  };
}