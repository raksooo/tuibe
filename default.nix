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

    src = ./.;
    cargoHash = "sha256-w6xB4ulrionZvVoynLu+TK0otLPlF4dBeq9SCbJ5lDU=";

    inherit nativeBuildInputs buildInputs;
  };

  env = pkgs.mkShell {
    name = "env";
    buildInputs = compilation ++ nativeBuildInputs ++ buildInputs;
    LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [ pkgs.openssl ];
  };

}
