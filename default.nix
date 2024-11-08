{
  pkgs ? import <nixpkgs> { },
}:

let
  compilation = with pkgs; [
    gcc
    cargo
    rustc
  ];
  nativeBuildInputs = with pkgs; [ pkg-config ];
  buildInputs = with pkgs; [ openssl ];
in
{
  package = pkgs.rustPlatform.buildRustPackage {
    pname = "tuibe";
    version = "0.1.0";

    src = pkgs.fetchFromGitHub {
      owner = "raksooo";
      repo = "tuibe";
      rev = "afba981bbcdc16e0e518ab9c1110502b05953f10";
      hash = "sha256-ldwRYnStSXu+Vfe0c19L0OFFr4DiS+p/rvhIWvszCzo=";
    };

    cargoHash = "sha256-w6xB4ulrionZvVoynLu+TK0otLPlF4dBeq9SCbJ5lDU=";

    inherit nativeBuildInputs buildInputs;
  };

  env = pkgs.mkShell {
    name = "env";
    buildInputs = compilation ++ nativeBuildInputs ++ buildInputs;
    LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [ pkgs.openssl ];
  };

}
