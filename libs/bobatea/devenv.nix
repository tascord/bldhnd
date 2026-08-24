{
  pkgs,
  lib,
  config,
  inputs,
  ...
}:

{
  env.GREET = "boba";
  packages = [
    pkgs.git
  ];

  languages.rust = {
    enable = true;
    channel = "nightly";
  };

  scripts.rustupdate.exec = ''
    rustup toolchain install nightly
    rustup default stable
  '';

  enterShell = ''
    rustupdate
    git --version
  '';
  
}
