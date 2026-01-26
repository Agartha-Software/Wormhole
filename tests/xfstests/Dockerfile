FROM ubuntu:24.04 AS test
WORKDIR /test
COPY --from=wormhole:latest /build/target/release/wormholed /bin/wormholed
COPY --from=wormhole:latest /build/target/release/wormhole /bin/wormhole

COPY tests/xfstests/run_xfstests_docker.sh /tests/run_xfstests_docker.sh
COPY tests/xfstests/generate_test_list.sh /tests/generate_test_list.sh
COPY tests/xfstests/mount.fuse.wormhole /sbin/mount.fuse.wormhole

RUN useradd -m fsgqa && \
    useradd -m 123456-fsgqa && \
    usermod -aG fsgqa 123456-fsgqa

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
    sudo \
    procps \
    psmisc \
    iputils-ping \
    vim \
    xfsprogs \
    dbench \
    && apt-get clean

RUN cd /opt && \
    git clone --depth 1 https://git.kernel.org/pub/scm/fs/xfs/xfstests-dev.git && \
    cd xfstests-dev && \
    make && \
    make install

COPY tests/xfstests/xfstests_exclude.txt /opt/xfstests-dev/exclude_tests

RUN mkdir -p /mnt/test /mnt/scratch

RUN chmod +x /tests/run_xfstests_docker.sh /tests/generate_test_list.sh /sbin/mount.fuse.wormhole

WORKDIR /opt/xfstests-dev
