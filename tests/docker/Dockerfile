FROM node:latest
WORKDIR /app
ENV CI=true

# Install moon binary
# RUN npm install -g @moonrepo/cli
COPY ./target/debug/moon /usr/bin

# Copy workspace skeleton
COPY ./.moon/docker/workspace .

# Manually copy these since they aren't moon projects
COPY ./packages/cli ./packages/cli
COPY ./packages/core-linux-arm64-gnu ./packages/core-linux-arm64-gnu
COPY ./packages/core-linux-arm64-musl ./packages/core-linux-arm64-musl
COPY ./packages/core-linux-x64-gnu ./packages/core-linux-x64-gnu
COPY ./packages/core-linux-x64-musl ./packages/core-linux-x64-musl
COPY ./packages/core-macos-arm64 ./packages/core-macos-arm64
COPY ./packages/core-macos-x64 ./packages/core-macos-x64
COPY ./packages/core-windows-x64-msvc ./packages/core-windows-x64-msvc

# Install toolchain and dependencies
RUN moon docker setup --log debug

# Copy source files
COPY ./.moon/docker/sources .

# Run something
RUN moon run website:build --log debug

# Prune workspace
RUN moon docker prune --log debug
