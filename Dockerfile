FROM rust:1-bookworm as builder
WORKDIR /usr/src/vm_transcript
COPY . .

RUN cargo install --path .

FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update && apt install -y openssl ca-certificates
COPY --from=builder /usr/local/cargo/bin/vm_transcript /usr/local/bin/vm_transcript
CMD ["vm_transcript"]