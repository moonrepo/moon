# NOTE: this Dockerfile is primarily used to compile moon's linux binaries.
# It supports compiling x86_64 and aarch64 binaries for both glibc and musl.
#
# Usage: docker buildx build --output type=local,dest=./output --platform linux/amd64,linux/arm64 -f scripts/Dockerfile .

# glibc

FROM rust:1.67.0 AS base-gnu
RUN cargo install cargo-chef --version ^0.1
WORKDIR /app
COPY .cargo .cargo
COPY rust-toolchain.toml .
RUN rustup update

FROM base-gnu AS plan-gnu
COPY Cargo* .
COPY crates crates
RUN cargo chef prepare --recipe-path recipe.json

FROM base-gnu AS build-gnu
ENV RUSTFLAGS "-C strip=symbols"
COPY --from=plan-gnu /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY Cargo* .
COPY crates crates
RUN cargo build --bin moon --release
RUN set -ex; \
  export target="$(rustc --version --verbose | grep 'host:' | cut -d' ' -f2)"; \
  mkdir -p "/out/target/$target/release" && cp target/release/moon "/out/target/$target/release/moon"

FROM scratch AS bin-gnu
COPY --from=build-gnu /out/target/ /target/

# musl (amd64)

FROM messense/rust-musl-cross:x86_64-musl AS base-musl-amd64
RUN cargo install cargo-chef --version ^0.1
WORKDIR /app
COPY rust-toolchain.toml .
RUN rustup update && rustup target add x86_64-unknown-linux-musl

FROM base-musl-amd64 AS plan-musl-amd64
COPY Cargo* .
COPY crates crates
RUN cargo chef prepare --recipe-path recipe.json

FROM base-musl-amd64 AS build-musl-amd64
ENV RUSTFLAGS "-C strip=symbols"
COPY --from=plan-musl-amd64 /app/recipe.json recipe.json
RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json
COPY Cargo* .
COPY crates crates
RUN cargo build --bin moon --release --target x86_64-unknown-linux-musl
RUN mkdir -p /out && cp --parents target/*/release/moon /out/

# musl (arm64)

FROM messense/rust-musl-cross:aarch64-musl AS base-musl-arm64
RUN cargo install cargo-chef --version ^0.1
WORKDIR /app
COPY rust-toolchain.toml .
RUN rustup update && rustup target add aarch64-unknown-linux-musl

FROM base-musl-arm64 AS plan-musl-arm64
COPY Cargo* .
COPY crates crates
RUN cargo chef prepare --recipe-path recipe.json

FROM base-musl-arm64 AS build-musl-arm64
ENV RUSTFLAGS "-C strip=symbols"
COPY --from=plan-musl-arm64 /app/recipe.json recipe.json
RUN cargo chef cook --release --target aarch64-unknown-linux-musl --recipe-path recipe.json
COPY Cargo* .
COPY crates crates
RUN cargo build --bin moon --release --target aarch64-unknown-linux-musl
RUN mkdir -p /out && cp --parents target/*/release/moon /out/

# musl (dynamic arch based on target)

FROM build-musl-${TARGETARCH} AS build-musl

FROM scratch AS bin-musl
COPY --from=build-musl /out/target/ /target/

# combined glibc and musl

FROM scratch AS bin
COPY --from=bin-gnu /target/ /target/
COPY --from=bin-musl /target/ /target/

FROM bin
