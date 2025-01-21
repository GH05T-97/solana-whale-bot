# Use the latest Rust nightly image
FROM rustlang/rust:nightly

# Set the working directory
WORKDIR /app

# Copy the project files
COPY . .

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    curl

# Use nightly rust and build
RUN rustup default nightly && \
    cargo build --release

# Use a smaller final image for deployment
FROM debian:buster-slim

WORKDIR /app

COPY --from=0 /app/target/release/solana_whale_bot .

# Add necessary libraries for runtime
RUN apt-get update && apt-get install -y libssl1.1 && apt-get clean

# Set the startup command
CMD ["./solana_whale_bot"]