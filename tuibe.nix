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
    version = "0.2.0";

    src = ./.;
    cargoLock.lockFile = ./Cargo.lock;

    inherit nativeBuildInputs buildInputs;
  };

  env = pkgs.mkShell {
    name = "env";
    buildInputs = compilation ++ nativeBuildInputs ++ buildInputs;
    LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [ pkgs.openssl ];
  };
}
