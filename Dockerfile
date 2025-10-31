FROM rust:trixie AS builder

RUN apt update && apt install -y pkg-config libfuse3-dev
WORKDIR /build
COPY Cargo.* .
COPY src/ ./src/

ENV GIT_HASH=1

RUN cargo build --release


FROM debian

WORKDIR /wormhole

COPY --from=builder /build/target/release/wormholed /bin/wormholed
COPY --from=builder /build/target/release/wormhole /bin/wormhole

RUN apt update && apt install -y fuse3

CMD ["/bin/wormholed"]