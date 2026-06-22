{ pkgs, ... }: 

{ 

  packages = [ pkgs.git pkgs.rustup pkgs.openssl ];

  languages.rust = {
    enable = true;
    channel = "nightly";
  };

}
