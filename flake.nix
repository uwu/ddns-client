{
  description = "Dynamic DNS client";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
    in
    {
      packages = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
        in
        {
          default = pkgs.rustPlatform.buildRustPackage {
            pname = "ddns-client";
            version = "0.2.0";
            src = self;
            cargoLock.lockFile = ./Cargo.lock;
            meta = {
              homepage = "https://github.com/uwu/ddns-client";
              license = pkgs.lib.licenses.mit;
              mainProgram = "ddns-client";
            };
          };
        }
      );

      checks = forAllSystems (system: {
        package = self.packages.${system}.default;
      });

      nixosModules.default =
        {
          config,
          lib,
          pkgs,
          ...
        }:
        let
          cfg = config.services.ddns-client;
        in
        {
          options.services.ddns-client = {
            enable = lib.mkEnableOption "ddns-client";
            package = lib.mkPackageOption pkgs "ddns-client" { };
            configFile = lib.mkOption {
              type = lib.types.path;
              description = "Path to the JSON config file.";
            };
          };

          config = lib.mkIf cfg.enable {
            systemd.services.ddns-client = {
              description = "Dynamic DNS client";
              wantedBy = [ "multi-user.target" ];
              after = [ "network-online.target" ];
              wants = [ "network-online.target" ];
              serviceConfig = {
                DynamicUser = true;
                LoadCredential = "config.json:${cfg.configFile}";
                ExecStart = "${lib.getExe cfg.package} %d/config.json";
                Restart = "always";
              };
            };
          };
        };

      formatter = forAllSystems (system: nixpkgs.legacyPackages.${system}.nixfmt-tree);
    };
}
