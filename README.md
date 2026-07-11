# bldhnd — Nix flake

This repository includes a small Nix flake that uses Crane to build a CLI package and a server package, and exposes a NixOS module to run the server as a systemd service.

## NixOS

Build the CLI:

```bash
nix build .#packages.x86_64-linux.cli
./result/bin/bldhnd
```

Build the server and run it locally:

```bash
nix build .#packages.x86_64-linux.server
./result/bin/bh-server &
```

Build the service+TUI combo:

```bash
nix build .#packages.x86_64-linux.service-tui
./result/bin/bh-service &
./result/bin/bldhnd &
```

Enable as a NixOS module

You can enable the module from this flake in your NixOS configuration and then enable the service. A minimal example for `configuration.nix`:

```nix
{
  imports = [ (builtins.getFlake (toString ./.)).nixosModules.bldhnd ];

  services.bldhnd.enable = true;
  services.bldhnd.mode = "combo";  # for service+TUI
}
```

Then run `nixos-rebuild switch` (or the equivalent with `--flake`) on your machine.

## Other Linux / macOS / Windows

### Via Nix (flakes)

```bash
nix run github:tascord/bldhnd -- --help
nix build github:tascord/bldhnd#packages.x86_64-linux.cli
```

### Via Cargo

Requires Rust 1.85+ and pkg-config.

```bash
cargo build --release -p bldhnd    # TUI
cargo build --release -p bh-server # Server
cargo build --release -p bh-service # Service
```

### Binary releases

See the GitHub Releases page for pre-built binaries.

## Nix (non-NixOS)

On non-NixOS systems with Nix installed, enable flakes:

```bash
nix build .#packages.x86_64-linux.cli
./result/bin/bldhnd
```
