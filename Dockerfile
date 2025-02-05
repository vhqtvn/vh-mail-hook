FROM rust:1.84.1 AS builder
WORKDIR /app

# Copy the workspace manifest and lock file
COPY Cargo.toml Cargo.lock ./

# Copy workspace source code directories
COPY src src
COPY crates crates

# Build the vh-mail-hook in release mode
RUN cargo build --release -p vh-mail-hook

FROM debian:buster-slim

# Install minimal dependencies needed
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the compiled binary from the builder stage
COPY --from=builder /app/target/release/vh-mail-hook .

# Expose ports:
# - 2525: Plain SMTP
# - 465: TLS SMTP
# - 8080: Web interface
EXPOSE 2525
EXPOSE 465
EXPOSE 8080

CMD ["./vh-mail-hook"] 