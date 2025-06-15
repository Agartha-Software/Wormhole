FROM rust:1.78.0-buster AS builder

RUN apt update && apt install -y pkg-config libfuse3-dev
WORKDIR /usr/src/wormhole
COPY . .
RUN cargo build --bin wormhole-service && cargo build --bin wormhole-cli

RUN mkdir -p /usr/src/wormhole/virtual && \
    chmod -R 775 /usr/src/wormhole/virtual

FROM debian:bullseye-slim

RUN apt-get update && \
    apt-get install -y fuse3 systemd netcat-openbsd && \
    echo 'user_allow_other' >> /etc/fuse.conf

WORKDIR /usr/src/wormhole
COPY --from=builder /usr/src/wormhole/target/debug/wormhole-service .
COPY --from=builder /usr/src/wormhole/target/debug/wormhole-cli .
COPY wormhole-service.service /etc/systemd/system/

RUN systemctl enable wormhole-service.service

CMD ["/lib/systemd/systemd"]