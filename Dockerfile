FROM rust:1-slim-bookworm as builder
WORKDIR /usr/src/vm_transcript
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev && \
    rm -rf /var/lib/apt/lists/* && \
    apt-get clean

COPY Cargo.toml Cargo.lock ./
RUN mkdir src && \
    touch src/lib.rs && \
    cargo build --release && \
    rm -rf src

COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    openssl \
    ca-certificates && \
    rm -rf /var/lib/apt/lists/* && \
    apt-get clean

COPY --from=builder /usr/src/vm_transcript/target/release/vm_transcript /usr/local/bin/vm_transcript

CMD ["vm_transcript"]