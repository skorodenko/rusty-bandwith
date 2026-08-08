# Stage 1: Build binary with all C/Rust dependencies
FROM rust:1.97-alpine AS builder

WORKDIR /usr/src/app

# Install build dependencies (available in main/community repos)
RUN apk add --no-cache \
    build-base \
    pkgconf \
    cmake \
    clang \
    git \
    musl-dev \
    openssl-dev \
    libjxl-dev \
    libwebp-dev

COPY . .
# Disable static CRT linking so musl uses .so libraries
RUN RUSTFLAGS="-C target-feature=-crt-static" cargo build --release

# Stage 2: Minimal runtime image
FROM alpine:latest

RUN apk add --no-cache \
    ca-certificates \
    openssl \
    libjxl \
    libwebp \
    libgcc

COPY --from=builder /usr/src/app/target/release/rusty-bandwidth /usr/local/bin/rusty-bandwidth

EXPOSE 8080

ENTRYPOINT ["rusty-bandwidth"]
CMD ["--host", "0.0.0.0", "--port", "8080"]
