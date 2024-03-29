FROM node:latest

WORKDIR /app

# Install moon binary
RUN npm install -g @moonrepo/cli

# Copy moon files
COPY ./.moon/docker/workspace .

# Install toolchain and dependencies
RUN moon setup

# Copy project and required files
COPY ./.moon/docker/sources .

RUN moon run runtime:build

RUN moon docker prune
