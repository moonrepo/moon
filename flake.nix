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

        rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

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

          nativeBuildInputs = nativeDeps ++ (with pkgs; [
            installShellFiles
            writableTmpDirAsHomeHook
          ]);
          buildInputs = buildDeps;

          env = {
            RUSTFLAGS = "-C strip=symbols";
            OPENSSL_NO_VENDOR = 1;
          };

          postInstall = pkgs.lib.optionalString
            (pkgs.stdenv.hostPlatform.emulatorAvailable pkgs.buildPackages)
            (
              let emulator = pkgs.stdenv.hostPlatform.emulator pkgs.buildPackages;
              in ''
                installShellCompletion --cmd moon \
                  --bash <(${emulator} $out/bin/moon completions --shell bash) \
                  --fish <(${emulator} $out/bin/moon completions --shell fish) \
                  --zsh <(${emulator} $out/bin/moon completions --shell zsh)
              ''
            );

          doCheck = false;

          meta = with pkgs.lib; {
            description = "A monorepo build system and task runner for the web ecosystem";
            mainProgram = "moon";
            homepage = "https://github.com/moonrepo/moon";
            changelog = "https://github.com/moonrepo/moon/releases/tag/v2.3.2";
            license = licenses.mit;
            maintainers = [ ];
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

          env = {
            OPENSSL_NO_VENDOR = "1";
            RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
          };
        };
      }
    );
}
