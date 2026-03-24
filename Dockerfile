# ===============================
# STAGE 1: Build Environment
# ===============================
FROM rust:1.80-slim-bookworm AS builder

# Install system dependencies required for compilation
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    build-essential \
    cmake \
    libsqlite3-dev

# Create a dummy project to cache dependencies
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Build the project (Release mode)
RUN cargo build -p sk-cli --release

# ===============================
# STAGE 2: Distroless Runtime
# ===============================
# We use a Debian-based slim image instead of pure distroless so we have sqlite and basic tools available if needed
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    libssl3 \
    libsqlite3-0 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create unprivileged user
RUN useradd -m -s /bin/bash sovereign

# Set working directory & copy binary
WORKDIR /app
COPY --from=builder /app/target/release/sovereign /usr/local/bin/sovereign

# Create persistent mounts
RUN mkdir -p /home/sovereign/.sovereign-kernel && chown -R sovereign:sovereign /home/sovereign

USER sovereign
# Set Environment variables
ENV RUST_LOG=info
ENV XDG_CONFIG_HOME=/home/sovereign
ENV XDG_DATA_HOME=/home/sovereign

# Expose API and Web UI ports
EXPOSE 4200
EXPOSE 8080

ENTRYPOINT ["sovereign"]
CMD ["start"]
