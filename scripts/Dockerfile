# NOTE: this Dockerfile is primarily used to compile moon's linux binaries.
# It supports compiling x86_64 and aarch64 binaries for both glibc and musl.
#
# Usage: docker buildx build --output type=local,dest=./output --platform linux/amd64,linux/arm64 -f scripts/Dockerfile .

# glibc

FROM ubuntu:22.04 AS base-gnu
RUN apt-get update
RUN apt install --no-install-recommends -y ca-certificates curl file build-essential autoconf automake autotools-dev libtool xutils-dev libssl-dev pkg-config

WORKDIR /app
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --default-toolchain stable -y
ENV PATH=/root/.cargo/bin:$PATH

COPY .cargo .cargo
COPY rust-toolchain.toml .
RUN rustup update && rustup toolchain install
# RUN cargo install cargo-chef --version ^0.1

# FROM base-gnu AS plan-gnu
# COPY Cargo* .
# COPY crates crates
# COPY legacy legacy
# RUN cargo chef prepare --recipe-path recipe.json

FROM base-gnu AS build-gnu
ENV RUSTFLAGS "-C strip=symbols"
# COPY --from=plan-gnu /app/recipe.json recipe.json
# RUN cargo chef cook --release --recipe-path recipe.json
COPY Cargo* .
COPY crates crates
COPY legacy legacy
RUN cargo build --bin moon --release
RUN target/release/moon --version
RUN set -ex; \
  export target="$(rustc --version --verbose | grep 'host:' | cut -d' ' -f2)"; \
  mkdir -p "/out/target/$target/release" && cp target/release/moon "/out/target/$target/release/moon"

FROM scratch AS bin-gnu
COPY --from=build-gnu /out/target/ /target/

# musl (amd64)

FROM messense/rust-musl-cross:x86_64-musl AS base-musl-amd64
RUN apt-get update
RUN apt install --no-install-recommends -y libssl-dev pkg-config musl-tools wget

# RUN echo "Building OpenSSL" && \
#   ln -s /usr/include/x86_64-linux-gnu/asm /usr/include/x86_64-linux-musl/asm && \
#   ln -s /usr/include/asm-generic /usr/include/x86_64-linux-musl/asm-generic && \
#   ln -s /usr/include/linux /usr/include/x86_64-linux-musl/linux && \
#   mkdir /musl && \
#   wget https://github.com/openssl/openssl/releases/download/OpenSSL_1_1_1t/openssl-1.1.1t.tar.gz && \
#   tar zxvf openssl-1.1.1t.tar.gz && \
#   cd openssl-1.1.1t/ && \
#   CC="musl-gcc -fPIE -pie" ./Configure no-shared no-async --prefix=/musl --openssldir=/musl/ssl linux-x86_64 && \
#   make depend && \
#   make -j$(nproc) && \
#   make install
# ENV PKG_CONFIG_ALLOW_CROSS 1
# ENV OPENSSL_STATIC true
# ENV OPENSSL_DIR /musl

RUN cargo install cargo-chef --version ^0.1
WORKDIR /app
COPY .cargo .cargo
COPY rust-toolchain.toml .
RUN rustup update && rustup toolchain install && rustup target add x86_64-unknown-linux-musl

# FROM base-musl-amd64 AS plan-musl-amd64
# COPY Cargo* .
# COPY crates crates
# COPY legacy legacy
# RUN cargo chef prepare --recipe-path recipe.json

FROM base-musl-amd64 AS build-musl-amd64
ENV RUSTFLAGS "-C strip=symbols"
# COPY --from=plan-musl-amd64 /app/recipe.json recipe.json
# RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json
COPY Cargo* .
COPY crates crates
COPY legacy legacy
RUN cargo build --bin moon --release --target x86_64-unknown-linux-musl
RUN target/x86_64-unknown-linux-musl/release/moon --version
RUN mkdir -p /out && cp --parents target/*/release/moon /out/

# musl (arm64)

FROM messense/rust-musl-cross:aarch64-musl AS base-musl-arm64
RUN apt-get update
RUN apt install --no-install-recommends -y libssl-dev pkg-config musl-tools wget

# RUN wget https://musl.cc/aarch64-linux-musl-cross.tgz && \
#   tar zxvf aarch64-linux-musl-cross.tgz && \
#   mv aarch64-linux-musl-cross /usr/local && \
#   ln -s /usr/local/aarch64-linux-musl-cross/bin/* /usr/local/bin/

# RUN echo "Building OpenSSL" && \
#   ln -s /usr/include/aarch64-linux-gnu/asm /usr/include/aarch64-linux-musl/asm && \
#   ln -s /usr/include/asm-generic /usr/include/aarch64-linux-musl/asm-generic && \
#   ln -s /usr/include/linux /usr/include/aarch64-linux-musl/linux && \
#   mkdir /musl && \
#   wget https://github.com/openssl/openssl/releases/download/OpenSSL_1_1_1t/openssl-1.1.1t.tar.gz && \
#   tar zxvf openssl-1.1.1t.tar.gz && \
#   cd openssl-1.1.1t/ && \
#   CC="aarch64-linux-musl-gcc" ./Configure no-shared no-async --prefix=/musl --openssldir=/musl/ssl linux-aarch64 && \
#   make depend && \
#   make -j$(nproc) && \
#   make install
# ENV PKG_CONFIG_ALLOW_CROSS 1
# ENV OPENSSL_STATIC true
# ENV OPENSSL_DIR /musl

# RUN cargo install cargo-chef --version ^0.1
WORKDIR /app
COPY .cargo .cargo
COPY rust-toolchain.toml .
RUN rustup update && rustup toolchain install && rustup target add aarch64-unknown-linux-musl

# FROM base-musl-arm64 AS plan-musl-arm64
# COPY Cargo* .
# COPY crates crates
# COPY legacy legacy
# RUN cargo chef prepare --recipe-path recipe.json

FROM base-musl-arm64 AS build-musl-arm64
ENV RUSTFLAGS "-C strip=symbols"
# COPY --from=plan-musl-arm64 /app/recipe.json recipe.json
# RUN cargo chef cook --release --target aarch64-unknown-linux-musl --recipe-path recipe.json
COPY Cargo* .
COPY crates crates
COPY legacy legacy
RUN cargo build --bin moon --release --target aarch64-unknown-linux-musl
RUN target/aarch64-unknown-linux-musl/release/moon --version
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
