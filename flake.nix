{
  description = "A monorepo build system and task runner for the web ecosystem";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachSystem [ "x86_64-linux" "aarch64-linux" ] (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };

        # Honors the pinned channel (1.96.0) and profile from rust-toolchain.toml.
        rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

        # reqwest enables native-tls-vendored for musl targets; on glibc Linux we
        # disable vendoring and let pkg-config find the system openssl instead.
        nativeDeps = with pkgs; [ pkg-config protobuf ];
        buildDeps = with pkgs; [ openssl ];
      in
      {
        packages.default = (pkgs.makeRustPlatform {
          cargo = rustToolchain;
          rustc = rustToolchain;
        }).buildRustPackage {
          pname = "moon";
          version = "2.3.2";
          src = ./.;

          cargoLock.lockFile = ./Cargo.lock;

          # Build only the CLI package; the wasm/ dir is a separate workspace and excluded.
          cargoBuildFlags = [ "--package" "moon_cli" ];

          nativeBuildInputs = nativeDeps;
          buildInputs = buildDeps;

          OPENSSL_NO_VENDOR = "1";

          meta = with pkgs.lib; {
            description = "A monorepo build system and task runner for the web ecosystem";
            homepage = "https://moonrepo.dev/moon";
            changelog = "https://github.com/moonrepo/moon/blob/master/CHANGELOG.md";
            license = licenses.mit;
            maintainers = [ ];
            mainProgram = "moon";
            platforms = platforms.linux;
          };
        };

        devShells.default = pkgs.mkShell {
          nativeBuildInputs = nativeDeps ++ [
            rustToolchain
            pkgs.just
            pkgs.cargo-nextest
          ];
          buildInputs = buildDeps;

          OPENSSL_NO_VENDOR = "1";
          # Needed by rust-analyzer and cargo doc.
          RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
        };
      }
    );
}
