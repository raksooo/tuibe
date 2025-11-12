{
  description = "Command line YouTube TUI";

  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";

  outputs =
    { self, nixpkgs }:
    let
      name = "tuibe";
      forAllSystems = nixpkgs.lib.genAttrs nixpkgs.lib.platforms.all;
    in
    {
      packages = forAllSystems (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
        in
        {
          default = (pkgs.callPackage ./${name}.nix { }).package;
        }
      );

      nixosModules.default =
        {
          lib,
          config,
          pkgs,
          ...
        }:
        {
          options.programs.${name} = {
            enable = lib.mkEnableOption "Enable ${name}";
          };

          config = lib.mkIf config.programs.${name}.enable {
            environment.systemPackages = [ self.packages.${pkgs.system}.default ];
          };
        };

      homeManagerModules.default =
        {
          config,
          lib,
          pkgs,
          ...
        }:
        {
          options.programs.${name} = {
            enable = lib.mkEnableOption "Enable ${name}";
          };

          config = lib.mkIf config.programs.${name}.enable {
            home.packages = [ self.packages.${pkgs.system}.default ];
          };
        };

      devShells = forAllSystems (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
        in
        {
          default = pkgs.mkShellNoCC {
            packages = with pkgs; [
              gcc
              cargo
              rustc
              pkg-config
              openssl
            ];
          };
        }
      );
    };
}
