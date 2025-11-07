FROM rust:trixie AS builder

RUN apt update && apt install -y pkg-config libfuse3-dev
WORKDIR /build
COPY Cargo.* .
COPY src/ ./src/

ENV GIT_HASH=1

RUN cargo build --release

FROM debian AS prod
WORKDIR /wormhole
COPY --from=builder /build/target/release/wormholed /bin/wormholed
COPY --from=builder /build/target/release/wormhole /bin/wormhole
RUN apt update && apt install -y fuse3
CMD ["/bin/wormholed"]

FROM ubuntu:24.04 AS test
WORKDIR /test
COPY --from=builder /build/target/release/wormholed /bin/wormholed
COPY --from=builder /build/target/release/wormhole /bin/wormhole

# Installation des dépendances complètes
RUN apt-get update && \
    DEBIAN_FRONTEND=noninteractive apt-get install -y \
    git \
    build-essential \
    autoconf \
    automake \
    libtool \
    pkg-config \
    libuuid1 \
    uuid-dev \
    libattr1-dev \
    libacl1-dev \
    libaio-dev \
    libgdbm-dev \
    xfslibs-dev \
    liburing-dev \
    libblkid-dev \
    fuse3 \
    libfuse3-dev \
    attr \
    acl \
    bc \
    dump \
    e2fsprogs \
    quota \
    && apt-get clean

# Cloner et compiler xfstests
RUN cd /opt && \
    git clone --depth 1 https://git.kernel.org/pub/scm/fs/xfs/xfstests-dev.git && \
    cd xfstests-dev && \
    make && \
    make install

# Créer les points de montage
RUN mkdir -p /mnt/test /mnt/scratch

# Définir le répertoire de travail
WORKDIR /opt/xfstests-dev

CMD ["/bin/bash"]