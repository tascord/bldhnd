# bldhnd — Nix flake

This repository includes a small Nix flake that builds a CLI package and a server package, and exposes a NixOS module to run the server as a systemd service.

Quick examples

Build the CLI:

```bash
nix build .#packages.x86_64-linux.cli
./result/bin/bldhnd-cli
```

Build the server and run it locally:

```bash
nix build .#packages.x86_64-linux.server
./result/bin/server &
```

Enable as a NixOS module

You can enable the module from this flake in your NixOS configuration and then enable the service. A minimal example for `configuration.nix`:

```nix
{
  imports = [ (builtins.getFlake (toString ./.)).nixosModules.bldhnd ];

  services.bldhnd.enable = true;
}
```

Then run `nixos-rebuild switch` (or the equivalent with `--flake`) on your machine.
