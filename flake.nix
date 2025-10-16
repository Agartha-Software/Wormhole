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
            packages =
              [ pkgs.cargo
                pkgs.rustc
                pkgs.rustfmt
                pkgs.pkg-config
                pkgs.fuse3
                # Kubernetes CLI & tooling
                pkgs.kubectl
                pkgs.kubernetes-helm
                pkgs.kustomize
                pkgs.k9s
                pkgs.k3d
                pkgs.kind
              ];
            RUST_SRC_PATH =
              "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
            shellHook = ''
              export KUBECONFIG="$PWD/.kube/config"
              mkdir -p "$(dirname "$KUBECONFIG")"
            '';
          };
        });

      packages = forEachSystem (system:
        let pkgs = nixpkgs.legacyPackages.${system};
        in { wormhole = import ./nix/package.nix { inherit pkgs self; }; });

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

      # Classic Kubernetes module (k3s single-node) independent from the app
      nixosModules.kubernetes = { config, lib, pkgs, ... }:
        let
          cfg = config.services.kubernetesSimple;
        in {
          options.services.kubernetesSimple = {
            enable = lib.mkEnableOption "Enable a local single-node Kubernetes (k3s) cluster";
          };

          config = lib.mkIf cfg.enable {
            environment.systemPackages = [ pkgs.kubectl pkgs.kubernetes-helm pkgs.k9s pkgs.kustomize ];
            services.k3s = {
              enable = true;
              role = "server";
              # Classic defaults; keep Traefik and defaults provided by k3s
              extraServerArgs = [ ];
            };
            virtualisation.containerd.enable = lib.mkDefault true;
            networking.firewall.enable = lib.mkDefault true;
            networking.firewall.allowedTCPPorts = lib.mkDefault [ 6443 ];
          };
        };

      # Optional Kubernetes module (k3s) you can enable in your NixOS config
      # Example usage in your system configuration:
      # {
      #   imports = [ inputs.wormhole.nixosModules.wormhole-k8s ];
      #   services.wormhole.kubernetes.enable = true;
      #   services.wormhole.kubernetes.disableTraefik = true; # optional
      #   services.wormhole.kubernetes.clusterCIDR = "10.42.0.0/16"; # optional
      #   services.wormhole.kubernetes.serviceCIDR = "10.43.0.0/16"; # optional
      # }
      nixosModules.wormhole-k8s = { config, lib, pkgs, ... }:
        let
          cfg = config.services.wormhole.kubernetes;
        in {
          options.services.wormhole.kubernetes = {
            enable = lib.mkEnableOption "Enable a local single-node Kubernetes (k3s) cluster";
            disableTraefik = lib.mkOption {
              type = lib.types.bool;
              default = true;
              description = "Disable built-in Traefik in k3s";
            };
            clusterCIDR = lib.mkOption {
              type = lib.types.str;
              default = "10.42.0.0/16";
              description = "Pod network CIDR for k3s";
            };
            serviceCIDR = lib.mkOption {
              type = lib.types.str;
              default = "10.43.0.0/16";
              description = "Service network CIDR for k3s";
            };
            extraServerArgs = lib.mkOption {
              type = lib.types.listOf lib.types.str;
              default = [ ];
              description = "Extra arguments passed to k3s server";
            };
          };

          config = lib.mkIf cfg.enable {
            # Ensure useful client tools are available on the system
            environment.systemPackages = [ pkgs.kubectl pkgs.kubernetes-helm pkgs.k9s pkgs.kustomize ];

            # k3s single-node server
            services.k3s = {
              enable = true;
              role = "server";
              extraServerArgs =
                [ "--cluster-cidr=${cfg.clusterCIDR}"
                  "--service-cidr=${cfg.serviceCIDR}"
                ]
                ++ lib.optional cfg.disableTraefik "--disable=traefik"
                ++ cfg.extraServerArgs;
            };

            # Container runtime & cgroups (generally safe defaults for k3s)
            virtualisation.containerd.enable = lib.mkDefault true;
            systemd.services.k3s.after = [ "network-online.target" ];
            networking.firewall.enable = lib.mkDefault true;
            # Open common Kubernetes ports (API server and nodeport range minimal example)
            networking.firewall.allowedTCPPorts = lib.mkDefault [ 6443 ];
          };
        };

      # Project-scoped helper apps to manage a local Kubernetes (kind) cluster
      apps = forEachSystem (system:
        let pkgs = nixpkgs.legacyPackages.${system};
        in {
          k8s-kind-start = {
            type = "app";
            program = "${pkgs.writeShellScriptBin "k8s-kind-start" ''
              set -euo pipefail
              KUBECONFIG="${KUBECONFIG:-$PWD/.kube/config}"
              mkdir -p "$(dirname "$KUBECONFIG")"

              # Create cluster if missing, then export kubeconfig to project-local path
              if ! "${pkgs.kind}/bin/kind" get clusters | grep -qx "wormhole"; then
                "${pkgs.kind}/bin/kind" create cluster --name wormhole --wait 120s
              fi
              "${pkgs.kind}/bin/kind" export kubeconfig --name wormhole --kubeconfig "$KUBECONFIG"
              echo "KUBECONFIG set to: $KUBECONFIG"
              "${pkgs.kubectl}/bin/kubectl" cluster-info || true
            ''}/bin/k8s-kind-start";
          };

          k8s-kind-stop = {
            type = "app";
            program = "${pkgs.writeShellScriptBin "k8s-kind-stop" ''
              set -euo pipefail
              "${pkgs.kind}/bin/kind" delete cluster --name wormhole || true
              echo "Deleted kind cluster 'wormhole'"
            ''}/bin/k8s-kind-stop";
          };
        });
    };
}
