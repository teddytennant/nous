{
  description = "Nous — decentralized everything-app";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        rustToolchain = pkgs.rust-bin.stable.latest.default;
      in
      {
        packages = {
          nous = pkgs.rustPlatform.buildRustPackage {
            pname = "nous";
            version = "0.1.0";
            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;

            nativeBuildInputs = with pkgs; [
              protobuf
              pkg-config
            ];

            buildInputs = with pkgs; [
              openssl
            ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
              pkgs.darwin.apple_sdk.frameworks.Security
              pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
            ];

            # Only build the CLI and API server binaries
            cargoBuildFlags = [ "--bin" "nous" "--bin" "nous-api" ];
            cargoTestFlags = [ "--workspace" ];

            meta = with pkgs.lib; {
              description = "Decentralized everything-app — identity, messaging, governance, payments, AI, and more";
              homepage = "https://github.com/teddytennant/nous";
              license = licenses.mit;
              mainProgram = "nous";
            };
          };

          default = self.packages.${system}.nous;
        };

        apps = {
          nous = flake-utils.lib.mkApp {
            drv = self.packages.${system}.nous;
            name = "nous";
          };
          nous-api = flake-utils.lib.mkApp {
            drv = self.packages.${system}.nous;
            name = "nous-api";
          };
          default = self.apps.${system}.nous;
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain
            protobuf
            pkg-config
            openssl
            nodejs_22
          ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            pkgs.darwin.apple_sdk.frameworks.Security
            pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
          ];

          RUST_LOG = "info";
        };
      }
    );
}
