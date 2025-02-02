# Builder stage
FROM rustlang/rust:nightly AS builder

# Set the working directory
WORKDIR /usr/src/app

# Copy the project files
COPY . .

# Configure Rust flags to ignore warnings
ENV RUSTFLAGS="-A warnings"

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Build the project
RUN rustup default nightly && \
    cargo build --release

# Runtime stage
FROM debian:buster-slim

WORKDIR /app

# Copy the binary from builder stage
COPY --from=builder /usr/src/app/target/release/solana_whale_trader /app/solana_whale_trader

# Add necessary libraries for runtime
RUN apt-get update && \
    apt-get install -y libssl1.1 ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# Set the startup command
CMD ["/app/solana_whale_trader"]