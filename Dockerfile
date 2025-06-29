# Build stage
FROM rust:1.83-alpine AS builder

# Install build dependencies
RUN apk add --no-cache musl-dev gcc

# Set working directory
WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src

# Build the application (use native architecture)
RUN cargo build --release

# Runtime stage
FROM alpine:3.19

# Install CA certificates for HTTPS requests
RUN apk add --no-cache ca-certificates

# Create a non-root user
RUN addgroup -g 1000 webly && \
    adduser -D -s /bin/sh -u 1000 -G webly webly

# Copy the binary from builder stage
COPY --from=builder /app/target/release/webly /usr/local/bin/webly

# Set the user
USER webly

# Set entrypoint
ENTRYPOINT ["webly"]
CMD ["--help"]
