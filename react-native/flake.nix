{
  description = "Rust development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
  };

  # This would be great but it creates an annoying interactive prompt, even if we locally trust the substituter already
  # nixConfig = {
  #   extra-substituters = [
  #     "https://binarycache.getportal.cc"
  #   ];
  #   extra-trusted-public-keys = [
  #     "binarycache.getportal.cc:I4++LAqnkRNdfYo5IFR+Khe8uJheqVnR/xQ8Gf29OvI="
  #   ];
  # };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, crane, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;

          config.android_sdk.accept_license = true;
          config.allowUnfree = true;
        };
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
          targets = [ "aarch64-linux-android" "x86_64-linux-android" ];
        };
        rustPlatform = pkgs.makeRustPlatform {
          rustc = rustToolchain;
          cargo = rustToolchain;
        };
        rustToolchainApple = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
          targets = [ "aarch64-apple-ios" "aarch64-apple-ios-sim" ];
        };
        android = {
          buildToolsVersion = "35.0.0";
          cmakeVersion = "3.22.1";
        };
        androidComposition = pkgs.androidenv.composeAndroidPackages {
          buildToolsVersions = [ android.buildToolsVersion ];
          platformVersions = [ "35" ];
          includeNDK = true;
          ndkVersion = "27.1.12297006";
          cmakeVersions = [ android.cmakeVersion ];
        };

        xcodeenv = import "${nixpkgs}/pkgs/development/mobile/xcodeenv" { inherit (pkgs) callPackage; };
        xcodewrapper = (xcodeenv.composeXcodeWrapper {
          versions = [ ];
          xcodeBaseDir = "/Applications/Xcode.app";
        });

        craneLib = (crane.mkLib pkgs);
        craneLibAndroid = craneLib.overrideToolchain (
          p: p.rust-bin.stable.latest.default.override {
            targets = [ "aarch64-linux-android" "x86_64-linux-android" ];
          }
        );
        craneLibIos = craneLib.overrideToolchain (
          p: p.rust-bin.stable.latest.default.override {
            targets = [ "aarch64-apple-ios" "aarch64-apple-ios-sim" ];
          }
        );

        unfilteredRoot = ../.; # The original, unfiltered source
        commonArgs = {
          src = pkgs.lib.fileset.toSource {
            root = unfilteredRoot;
            fileset = pkgs.lib.fileset.unions [
              # Default files from crane (Rust and cargo files)
              (craneLib.fileset.commonCargoSources unfilteredRoot)
              ../assets
            ];
          };
          strictDeps = true;
          doCheck = false;
          cargoExtraArgs = "-p business_app";

          cargoCheckCommand = "true";
        };
        mkAndroidCommonArgs = target: rec {
          cargoBuildCommand = "cargo ndk --target ${target} --platform 23 -- build --profile release";

          ANDROID_SDK_ROOT = "${androidComposition.androidsdk}/libexec/android-sdk";
          ANDROID_NDK_ROOT = "${ANDROID_SDK_ROOT}/ndk-bundle";

          nativeBuildInputs = with pkgs; [
            cargo-ndk
            protobuf
          ];
        };
        mkIosCommonArgs = target: {
          cargoBuildCommand = "cargo build --profile release --target ${target}";

          preBuild = ''
            unset DEVELOPER_DIR
            export PATH=${xcodewrapper}/bin:$PATH
          '';

          nativeBuildInputs = with pkgs; [
            protobuf
          ];
        };

        mkAndroidArtifacts = target: (craneLibAndroid.buildDepsOnly ((mkAndroidCommonArgs target) // commonArgs // {
          pname = "libapp-android-deps-${target}";
        }));
        mkAndroidPackage = target: craneLibAndroid.buildPackage ((mkAndroidCommonArgs target) // commonArgs // {
          pname = "libapp-android-${target}";
          cargoArtifacts = mkAndroidArtifacts target;
        });

        mkIosArtifacts = target: (craneLibIos.buildDepsOnly ((mkIosCommonArgs target) // commonArgs // {
          pname = "libapp-ios-deps-${target}";
        }));
        mkIosPackage = target: craneLibIos.buildPackage ((mkIosCommonArgs target) // commonArgs // {
          pname = "libapp-ios-${target}";
          cargoArtifacts = mkIosArtifacts target;
        });

        cargoMetadata = {
          cargoArtifacts,
          manifestPath ? null,
          ...
        }@origArgs: let
          args = builtins.removeAttrs origArgs [
            "manifestPath"
          ];
        in
        craneLibAndroid.mkCargoDerivation (args // {
          inherit cargoArtifacts;

          pnameSuffix = "-metadata";

          buildPhase = ''
            mkdir -p $out
            cargo metadata --manifest-path ${manifestPath} > $out/metadata.json
          '';
          buildPhaseCargoCommand = "";
          nativeBuildInputs = (args.nativeBuildInputs or [ ]);
        });
        mkCargoMetadata = target: (cargoMetadata ((mkAndroidCommonArgs target) // commonArgs // {
          cargoArtifacts = mkAndroidArtifacts target;
          manifestPath = "./business_app/Cargo.toml";
        }));

        yarn-berry = pkgs.yarn-berry_3;
        reactNativeDeps = yarn-berry.fetchYarnBerryDeps {
          yarnLock = ./yarn.lock;
          missingHashes = ./missing-hashes.json;
          hash = "sha256-QJqtSj+PLlFkS+PJPAKezJeSBI/7a+Vet+3XpBAAJxk=";
        };
        ubrnSrc = pkgs.stdenv.mkDerivation {
          name = "ubrn-src";
          src = pkgs.lib.fileset.toSource {
            root = ./.;
            fileset = pkgs.lib.fileset.unions [
              ./package.json
              ./yarn.lock
            ];
          };
          nativeBuildInputs = [
            yarn-berry
            yarn-berry.yarnBerryConfigHook
          ];

          installPhase = ''
            UBRN_PATH=$(yarn ubrn --path)
            UBRN_PATH=$(echo $UBRN_PATH | sed 's/\/bin\/cli.cjs//')

            mkdir -p $out/
            cp -R $UBRN_PATH/* $out/
          '';

          missingHashes = ./missing-hashes.json;
          offlineCache = reactNativeDeps;
        };
        ubrn = rustPlatform.buildRustPackage {
          name = "ubrn";
          src = ubrnSrc;
          buildFlags = "--manifest-path crates/ubrn_cli/Cargo.toml";
          cargoPatches = [
            ./ubrn-cargo-lock.patch
          ];
          cargoHash = "sha256-mwjxYDszdPL23jcrfSF/qmoTyShOovKSgHNXQqbAsLs=";
          doCheck = false;
          meta.mainProgram = "uniffi-bindgen-react-native";
        };

        libAndroidAarch64 = mkAndroidPackage "arm64-v8a";
        libAndroidX86_64 = mkAndroidPackage "x86_64";

        libIosAarch64 = mkIosPackage "aarch64-apple-ios";
        libIosAarch64Sim = mkIosPackage "aarch64-apple-ios-sim";

        fakeCargoMetadata = pkgs.writeShellScriptBin "cargo" ''
          # We need to patch the manifest path otherwise uniffi-bindgen-react-native will fail to find the package
          cat ${mkCargoMetadata "arm64-v8a"}/metadata.json | sed 's|/build/source|${../business_app}|g'
        '';

        reactNativeLib = { withIos ? false }: pkgs.stdenv.mkDerivation (finalAttrs: {
          name = "react-native-lib${if withIos then "" else "-android-only"}";
        
          src = pkgs.lib.sources.cleanSource ./.;

          nativeBuildInputs = with pkgs; [
            nodejs
            ubrn
            fakeCargoMetadata
            yarn-berry.yarnBerryConfigHook
          ];

          buildPhase = ''
            # The ubrn config poits to ../app, so symlink the source there
            ln -s ${../business_app} ../business_app

            # Generate the bindings for both platforms
            uniffi-bindgen-react-native generate all --config ./ubrn.config.yaml ${libAndroidAarch64}/lib/libbusiness_app.a

            # Copy the artifacts to the android directory
            mkdir -p android/src/main/jniLibs/{arm64-v8a,x86_64}
            cp ${libAndroidAarch64}/lib/libbusiness_app.a android/src/main/jniLibs/arm64-v8a/libportal.a
            cp ${libAndroidX86_64}/lib/libbusiness_app.a android/src/main/jniLibs/x86_64/libportal.a

            ${pkgs.lib.optionalString withIos ''
              # Since those packages are built on a native macos worker, we take them from its package output set
              mkdir -p PortalAppLibFramework.xcframework/{ios-arm64,ios-arm64-simulator}
              cp ${./static/Info.plist} PortalAppLibFramework.xcframework/Info.plist
              cp ${self.outputs.packages.aarch64-darwin.ios-lib-aarch64}/lib/libapp.a PortalAppLibFramework.xcframework/ios-arm64/libapp.a
              cp ${self.outputs.packages.aarch64-darwin.ios-lib-aarch64-sim}/lib/libapp.a PortalAppLibFramework.xcframework/ios-arm64-simulator/libapp.a
            ''}

            # Fix tsc compilation issues (https://github.com/jhugman/uniffi-bindgen-react-native/issues/244)
            node ./patch-uniffi-bindgen.js

            # Pack the package for npm
            npm pack
          '';

          installPhase = ''
            mkdir -p $out
            mv *.tgz $out/
          '';
        
          missingHashes = ./missing-hashes.json;
          offlineCache = reactNativeDeps;
        });
      in
      {
        devShells.ios = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchainApple
            nodejs
            yarn
          ];

          shellHook = ''
            # This is set somewhere by nix
            unset DEVELOPER_DIR
            # We want to use stuff from the xcode wrapper over nixpkgs
            export PATH=${xcodewrapper}/bin:$PATH
          '';
        };
        devShells.default = pkgs.mkShell rec {
          buildInputs = with pkgs; [
            rustToolchain
            nodejs
            cmake
            ninja
            clang-tools
            cargo-ndk
            jdk
            yarn
            protobuf
          ];

          ANDROID_SDK_ROOT = "${androidComposition.androidsdk}/libexec/android-sdk";
          ANDROID_NDK_ROOT = "${ANDROID_SDK_ROOT}/ndk-bundle";

          # Ensures that we don't have to use a FHS env by using the nix store's aapt2.
          GRADLE_OPTS = "-Dorg.gradle.project.android.aapt2FromMavenOverride=${ANDROID_SDK_ROOT}/build-tools/${android.buildToolsVersion}/aapt2";
        };

        packages.react-native-lib = reactNativeLib { withIos = true; };
        packages.react-native-lib-android-only = reactNativeLib { withIos = false; };

        packages.android-lib-aarch64 = libAndroidAarch64;
        packages.android-lib-x86_64 = libAndroidX86_64;

        packages.ios-lib-aarch64 = libIosAarch64;
        packages.ios-lib-aarch64-sim = libIosAarch64Sim;

        packages.ubrn = ubrn;
      }
    );
}
