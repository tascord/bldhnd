# bldhnd — Nix flake

This repository includes a small Nix flake that uses Crane to build a CLI, server, service, and TUI, and exposes a NixOS module to run them separately.

## Packages

| Output | Description |
|--------|-------------|
| `cli` | Command-line interface |
| `bh-server` | Soulseek server |
| `bh-service` | Service layer (talks to server) |
| `bldhnd` | Terminal UI |
| `fz` | Fuzzy finder |

## Build

```bash
nix build .#bh-server   # Server
nix build .#bh-service   # Service
nix build .#bldhnd      # TUI
nix build .#cli         # CLI
```

## Running locally

```bash
# Server
nix build .#bh-server
./result/bin/bh-server &

# Service (connects to server)
nix build .#bh-service
./result/bin/bh-service &

# TUI (connects to service)
nix build .#bldhnd
./result/bin/bldhnd
```

## NixOS

Enable the server, service, and TUI separately:

```nix
{
  imports = [ (builtins.getFlake (toString ./.)).nixosModules.bldhnd ];

  services.bldhnd-server.enable = true;
  services.bldhnd-service.enable = true;
  programs.bldhnd.enable = true;
}
```

Each component has an optional `package` attribute if you need to override the default.

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

On non-NixOS systems with Nix installed, enable flakes and install to your user profile:

```bash
nix profile install github:tascord/bldhnd#packages.x86_64-linux.cli
bldhnd
```

Or build locally:

```bash
nix build .#packages.x86_64-linux.cli
./result/bin/bldhnd
```