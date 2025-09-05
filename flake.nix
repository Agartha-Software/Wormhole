{
  inputs = { nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05"; };

  outputs = { self, nixpkgs, ... }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forEachSystem = nixpkgs.lib.genAttrs systems;
    in {
      devShells = forEachSystem (system:
        let pkgs = nixpkgs.legacyPackages.${system};
        in {
          default = pkgs.mkShell {
            packages = [
              pkgs.cargo
              pkgs.rustc
              pkgs.rustfmt
              pkgs.pkg-config
              pkgs.fuse3
              pkgs.docker
              pkgs.docker-compose
            ];
            RUST_SRC_PATH =
              "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
          };
        });

      packages = forEachSystem (system:
        let pkgs = nixpkgs.legacyPackages.${system};
        in { wormhole = import ./nix/package.nix { inherit pkgs; }; });

      nixosModules.wormhole = { config, lib, pkgs, ... }:
        let
          cfg = config.services.wormhole;
          package = self.packages.${pkgs.system}.wormhole;
        in {
          options.services.wormhole = {
            enable = lib.mkEnableOption "Run the Wormhole daemon";
          };

          config = lib.mkIf cfg.enable {
            systemd.services.wormhole = {
              description = "Wormhole Service Daemon";
              wantedBy = [ "multi-user.target" ];
              after = [ "network.target" ];
              serviceConfig = {
                ExecStart = "${package}/bin/wormholed 0.0.0.0:8081";
                Restart = "on-failure";
              };
              environment.SERVICE_ADDRESS = "0.0.0.0:8081";
            };
          };
        };
    };
}
