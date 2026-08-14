{
  description = "A monorepo build system and task runner for the web ecosystem";

  nixConfig = {
    extra-substituters = [ "https://moonrepo.cachix.org" ];
    extra-trusted-public-keys = [
      "moonrepo.cachix.org-1:n4zm4mkV1Eoqck4mQvAhJM28EQwFLU7kW4dEbtAXbD8="
    ];
  };

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      crane,
      rust-overlay,
    }:
    flake-utils.lib.eachSystem
      [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
      ]
      (
        system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ rust-overlay.overlays.default ];
          };

          rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

          devRustToolchain = rustToolchain.override {
            extensions = [ "rust-src" ];
          };

          craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

          moonVersion = (builtins.fromTOML (builtins.readFile ./crates/cli/Cargo.toml)).package.version;

          nativeDeps = with pkgs; [
            pkg-config
            protobuf
          ];
          buildDeps = with pkgs; [ openssl ] ++ lib.optionals stdenv.hostPlatform.isDarwin [ libiconv ];

          cargoFiles = pkgs.lib.fileset.unions [
            ./Cargo.toml
            ./Cargo.lock
            ./.cargo/config.toml
            (craneLib.fileset.commonCargoSources ./crates)
          ];

          cargoSource = pkgs.lib.fileset.toSource {
            root = ./.;
            fileset = cargoFiles;
          };

          source = pkgs.lib.fileset.toSource {
            root = ./.;
            fileset = pkgs.lib.fileset.unions [
              cargoFiles
              ./crates/daemon-proto/proto
              ./crates/daemon-proto/vendor
              ./crates/config-loader/res
              ./crates/docker/templates/Dockerfile.tera
              ./crates/app/src/commands/graph/html.tera
              ./crates/query/src/mql.pest
            ];
          };

          commonArgs = {
            pname = "moon";
            version = moonVersion;
            src = source;

            cargoExtraArgs = "--locked --package moon_cli --bins";
            strictDeps = true;
            doCheck = false;

            nativeBuildInputs = nativeDeps;
            buildInputs = buildDeps;

            env = {
              RUSTFLAGS = "-C strip=symbols";
              OPENSSL_NO_VENDOR = "1";
            };
          };

          cargoArtifacts = craneLib.buildDepsOnly (
            commonArgs
            // {
              src = cargoSource;
              buildPhaseCargoCommand = "cargo build --release --locked --package moon_cli --bins";
            }
          );

          moon = craneLib.buildPackage (
            commonArgs
            // {
              inherit cargoArtifacts;

              nativeBuildInputs =
                nativeDeps
                ++ (with pkgs; [
                  installShellFiles
                  writableTmpDirAsHomeHook
                ]);

              postInstall =
                pkgs.lib.optionalString (pkgs.stdenv.hostPlatform.emulatorAvailable pkgs.buildPackages)
                  (
                    let
                      emulator = pkgs.stdenv.hostPlatform.emulator pkgs.buildPackages;
                    in
                    ''
                      installShellCompletion --cmd moon \
                        --bash <(${emulator} $out/bin/moon completions --shell bash) \
                        --fish <(${emulator} $out/bin/moon completions --shell fish) \
                        --zsh <(${emulator} $out/bin/moon completions --shell zsh)
                    ''
                  );

              meta = with pkgs.lib; {
                description = "A monorepo build system and task runner for the web ecosystem";
                mainProgram = "moon";
                homepage = "https://github.com/moonrepo/moon";
                changelog = "https://github.com/moonrepo/moon/releases/tag/v${moonVersion}";
                license = licenses.mit;
                maintainers = [ ];
                platforms = platforms.linux ++ [ "aarch64-darwin" ];
              };
            }
          );
        in
        {
          packages = {
            inherit moon;
            moon-deps = cargoArtifacts;
            default = moon;
          };

          apps.default = {
            type = "app";
            program = "${moon}/bin/moon";
          };

          devShells.default = pkgs.mkShell {
            nativeBuildInputs = nativeDeps ++ [
              devRustToolchain
              pkgs.just
              pkgs.cargo-nextest
            ];
            buildInputs = buildDeps;

            env = {
              OPENSSL_NO_VENDOR = "1";
              RUST_SRC_PATH = "${devRustToolchain}/lib/rustlib/src/rust/library";
            };
          };
        }
      );
}
