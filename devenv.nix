{ pkgs, ... }:

{

  packages = [
    pkgs.git
    pkgs.rustup
    pkgs.openssl
    pkgs.gcc
  ];

  languages.rust = {
    enable = true;
    channel = "nightly";
  };

  processes.bh-service.exec = "cargo run -p bh-service";

}
