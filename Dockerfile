# Use the official Rust image with a newer version
FROM rust:1.74

# Set the working directory
WORKDIR /app

# Copy the project files
COPY . .

# Install dependencies and build the application
RUN apt-get update && apt-get install -y pkg-config libssl-dev && \
    cargo build --release

# Use a smaller final image for deployment
FROM debian:buster-slim

WORKDIR /app

COPY --from=0 /app/target/release/solana_whale_bot .

# Add necessary libraries for runtime
RUN apt-get update && apt-get install -y libssl1.1 && apt-get clean

# Set the startup command
CMD ["./solana_whale_bot"]