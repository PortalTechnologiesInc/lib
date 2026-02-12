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
        rustToolchain = pkgs.rust-bin.stable."1.90.0".default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };

        rustPlatform = pkgs.makeRustPlatform {
          cargo = rustToolchain;
          rustc = rustToolchain;
        };

        # Static build: same Rust 1.90 as overlay, musl target (lnurl-models needs rustc 1.88+)
        rustToolchainStatic = pkgs.rust-bin.stable."1.90.0".default.override {
          targets = [ "x86_64-unknown-linux-musl" ];
        };
        staticRustPlatform = pkgs.makeRustPlatform {
          cargo = rustToolchainStatic;
          rustc = rustToolchainStatic;
        };

        rest' = platform: platform.buildRustPackage {
          pname = "portal-rest";
          version = (pkgs.lib.importTOML ./crates/portal-rest/Cargo.toml).package.version;
          src = pkgs.lib.sources.sourceFilesBySuffices ./. [ ".rs" "Cargo.toml" "Cargo.lock" "fiatUnits.json" "example.config.toml" ];

          cargoHash = "";
          cargoLock = {
            lockFile = ./Cargo.lock;
            outputHashes = {
              "cashu-0.11.0" = "sha256-/GLSl7/Io4vfEVD9a64dnC7DgiTfwfSOODmsMVkr8w8=";
              "nostr-0.43.0" = "sha256-1TLthpGnDLUmnBoq2CneWnfTMwRocitbD4+wnrlCA44=";
              "breez-sdk-common-0.1.0" = "sha256-7pK+yuJAvbbh7r+kNCAQT7Siu7xZ9gzPtLDoUoMgvgs=";
            };
          };
          buildAndTestSubdir = "crates/portal-rest";

          doCheck = false;

          nativeBuildInputs = with pkgs; [
            # Needed to build cashu
            protobuf
            pkg-config
          ];

          buildInputs = with pkgs; [
            openssl
          ];

          meta.mainProgram = "rest";
        };

        tsClient = pkgs.buildNpmPackage {
          name = "portal-ts-client";
          version = (builtins.fromJSON (builtins.readFile ./crates/portal-rest/clients/ts/package.json)).version;
          src = ./crates/portal-rest/clients/ts;
          npmDepsHash = "sha256-nM/XE4CRUAP7W4FQ9t68kBHrUIvNbBxZC+gwpZcCCfA=";
        };
        backend = pkgs.buildNpmPackage {
          name = "portal-backend";
          version = (builtins.fromJSON (builtins.readFile ./backend/package.json)).version;
          src = ./backend;
          npmDepsHash = "sha256-tUGd0fSl4H3lmSC2eMy8wtvgQa9/y8h+m4SI7zORRTk=";
          buildInputs = [ pkgs.sqlite ];
          preBuild = ''
            # Remove symlink to non-existent "../crates/portal-rest/clients/ts"
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
        packages = rec {
          inherit backend;

          rest = rest' rustPlatform;
          # Static binary for Docker: overlay Rust 1.90 + musl stdenv + static openssl
          rest-static = rest' staticRustPlatform;

          rest-docker = let
            minimal-closure = pkgs.runCommand "minimal-rust-app" {
              nativeBuildInputs = [ pkgs.removeReferencesTo ];
            } ''
              mkdir -p $out/bin
              cp ${rest-static}/bin/rest $out/bin/

              for binary in $out/bin/*; do
                remove-references-to -t ${rustToolchainStatic} "$binary"
              done
            '';
          in pkgs.dockerTools.buildLayeredImage {
            name = "getportal/sdk-daemon";
            tag = if system == "x86_64-linux" then "amd64" else "arm64";

            config = {
              Cmd = [ "${minimal-closure}/bin/rest" ];
              ExposedPorts = {
                "3000/tcp" = {};
              };
              # Only non-secret defaults. Required (AUTH_TOKEN, NOSTR__PRIVATE_KEY) and
              # optional env are passed when starting the container (docker run -e / --env-file).
              Env = [
                "PORTAL__INFO__LISTEN_PORT=3000"
                "RUST_LOG=portal=debug,rest=debug,info"
              ];
            };
          };

          docs = pkgs.stdenv.mkDerivation {
            name = "portal-docs";
            src = ./docs;
            buildInputs = [ pkgs.mdbook ];
            buildPhase = "mdbook build";
            installPhase = "cp -R book $out";
          };
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain
            protobuf
	    openssl
	    pkg-config
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
          portal-rest = ./crates/portal-rest/module.nix;
          portal-backend = ./backend/module.nix;
        };
    };
}
