{ pkgs, ... }: 

{ 

  packages = [ pkgs.git pkgs.rustup ];

  languages.rust = {
    enable = true;
    channel = "nightly";
  };

}
