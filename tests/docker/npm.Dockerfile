FROM node:latest

WORKDIR /app

# Install moon binary
RUN npm install -g @moonrepo/cli

# Copy moon files
COPY ./.moon/project.yml ./.moon/workspace.yml ./.moon/
COPY ./.moon/docker/workspace .

# Install toolchain and dependencies
RUN moon setup

# Copy project and required files
COPY ./packages/types ./packages/types
COPY ./packages/runtime ./packages/runtime

RUN moon run runtime:build
