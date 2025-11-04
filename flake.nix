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
        rustToolchain = pkgs.rust-bin.stable."1.88.0".default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };

        rustPlatform = pkgs.makeRustPlatform {
          cargo = rustToolchain;
          rustc = rustToolchain;
        };

        rest' = platform: platform.buildRustPackage {
          pname = "portal-rest";
          version = (pkgs.lib.importTOML ./rest/Cargo.toml).package.version;
          src = pkgs.lib.sources.sourceFilesBySuffices ./. [ ".rs" "Cargo.toml" "Cargo.lock" ];

          cargoHash = "";
          cargoLock = {
            lockFile = ./Cargo.lock;
            outputHashes = {
              "cashu-0.11.0" = "sha256-kwwfQX5vDclfa86xPbbkbu+bh1VQXlX+imunUUoTYV4=";
              "nostr-0.43.0" = "sha256-1TLthpGnDLUmnBoq2CneWnfTMwRocitbD4+wnrlCA44=";
              "breez-sdk-common-0.1.0" = "sha256-b8R4V8L7lM0AOy9NxhiIt+RsIBHJdQPpfw9SN1/P//E=";
            };
          };
          buildAndTestSubdir = "rest";

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
          version = (builtins.fromJSON (builtins.readFile ./rest/clients/ts/package.json)).version;
          src = ./rest/clients/ts;
          npmDepsHash = "sha256-nM/XE4CRUAP7W4FQ9t68kBHrUIvNbBxZC+gwpZcCCfA=";
        };
        backend = pkgs.buildNpmPackage {
          name = "portal-backend";
          version = (builtins.fromJSON (builtins.readFile ./backend/package.json)).version;
          src = ./backend;
          npmDepsHash = "sha256-37YrynQdzAHc0gP+wWEQLjzDkrknqcS/Hxa49H3NsjU=";
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
        packages = rec {
          inherit backend;

          rest = rest' rustPlatform;
          rest-static = rest' pkgs.pkgsStatic.rustPlatform;

          rest-docker = let
            minimal-closure = pkgs.runCommand "minimal-rust-app" {
              nativeBuildInputs = [ pkgs.removeReferencesTo ];
            } ''
              mkdir -p $out/bin
              cp ${rest-static}/bin/rest $out/bin/

              for binary in $out/bin/*; do
                remove-references-to -t ${pkgs.pkgsStatic.rustPlatform.rust.rustc} "$binary"
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
              Env = [
                "AUTH_TOKEN=remeber-to-change-this"
                "NOSTR_RELAYS=wss://relay.getportal.cc,wss://relay.damus.io,wss://relay.nostr.net"
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
          portal-rest = ./rest/module.nix;
          portal-backend = ./backend/module.nix;
        };
    };
}
