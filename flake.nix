{
  description = "Rust development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        fs = pkgs.lib.fileset;
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };

        rustPlatform = pkgs.makeRustPlatform {
          cargo = rustToolchain;
          rustc = rustToolchain;
        };

        rest = rustPlatform.buildRustPackage {
          pname = "portal-rest";
          version = (pkgs.lib.importTOML ./rest/Cargo.toml).package.version;
          src = pkgs.lib.sources.sourceFilesBySuffices ./. [ ".rs" "Cargo.toml" "Cargo.lock" ];

          cargoHash = "";
          cargoLock.lockFile = ./Cargo.lock;
          buildAndTestSubdir = "rest";

          doCheck = false;

          meta.mainProgram = "rest";
        };

        tsClient = pkgs.buildNpmPackage {
          name = "portal-ts-client";
          version = (builtins.fromJSON (builtins.readFile ./rest/clients/ts/package.json)).version;
          src = ./rest/clients/ts;
          npmDepsHash = "sha256-FMDlTvFqtjCk7lVmDnBuuWlNmQVri9cbcD3vK24Y+1k=";
        };
        backend = pkgs.buildNpmPackage {
          name = "portal-backend";
          version = (builtins.fromJSON (builtins.readFile ./backend/package.json)).version;
          src = ./backend;
          npmDepsHash = "sha256-Q97WXfJRF6RMLGubHwaYkoHfgjbCvH2d18TO3n1PI/o=";
          buildInputs = [ pkgs.sqlite ];
          preBuild = ''
            # Remove symlink to non-existent "../rest/clients/ts"
            rm -rf ./node_modules/portal-sdk
            # Copy the dependency
            cp -R ${tsClient}/lib/node_modules/portal-sdk ./node_modules/
          '';
          postInstall = ''
            # Remove danlging symlink to non-existent path
            rm -rf $out/lib/node_modules/portal-backend/node_modules/portal-sdk
            # Copy again the dependency ??
            cp -R ${tsClient}/lib/node_modules/portal-sdk $out/lib/node_modules/portal-backend/node_modules/portal-sdk

            cp -Rv ./public $out/
          '';
          meta.mainProgram = "portal-backend";
        };
      in
      {
        packages = {
          inherit rest backend;
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain
          ];
        };
        devShells.nodejs = pkgs.mkShell rec {
          buildInputs = with pkgs; [
            nodejs
            python3
            sqlite
            yarn
          ];
        };

        checks = {
          vm-test = pkgs.nixosTest {
            name = "portal-backend-vm-test";

            nodes.machine = { config, pkgs, lib, ... }: {
              imports = [ self.nixosModules.default ];

              services.portal-backend = {
                enable = true;
                authToken = "vm-test-token";
              };
              services.portal-rest = {
                nostrKey = "nsec1rzl9z80dnn78zcv7p9t74sqss6xdvvg0dj0ef3wcmuy2lx3sh25qcmykwf";
                rustLog = "portal=trace,rest=trace,info";
              };
            };

            testScript = ''
              machine.start()
              machine.wait_for_unit("portal-rest.service")
              machine.wait_for_unit("portal-backend.service")

              # Wait a bit more for the service to fully start
              machine.sleep(5)

              # Test the health check endpoint
              machine.succeed("curl -f http://localhost:8000")

              print("✅ Portal backend is running!")
            '';
          };
        };
      }
    ) // {
        overlays.default = final: prev: {
          portal-backend = self.packages.${prev.stdenv.hostPlatform.system}.backend;
          portal-rest = self.packages.${prev.stdenv.hostPlatform.system}.rest;
        };

        nixosModules = {
          default = { ... }: {
            imports = [ self.nixosModules.portal-rest self.nixosModules.portal-backend ];
            nixpkgs.overlays = [ self.overlays.default ];
          };
          portal-rest = ./rest/module.nix;
          portal-backend = ./backend/module.nix;
        };
    };
}
