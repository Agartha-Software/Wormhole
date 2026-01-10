# Docker Guide

We provide a Docker image and a ready-to-use Docker Compose configuration to demonstrate Wormhole's capabilities. This configuration automatically creates a mesh network of 3 nodes.

> [!NOTE]
> The `docker-compose.yml` configures 3 services: `whone`, `whtwo`, and `whthree`. They are connected in a chain to form a single network.

## Prerequisites

- **Docker** and **Docker Compose** installed.
- **Docker Compose version 2.30.0+** is recommended because we use the `post_start` directive to automate pod creation.

## Quick Start

At the project root, run:

```sh
docker compose up --build
```

If your Docker Compose version is recent, the script will automatically:

1. Start the `wormholed` daemons.
2. Create the mount folders `whfolder`.
3. Run the `wormhole new` commands to connect the containers together using the `post_start` script.

> [!WARNING] If you are using an older version of Docker Compose, the `post_start` commands will be ignored.
> You will need to enter each container and run the commands manually (see the `docker-compose.yml` section for the exact commands).

## Usage and Testing

Once the containers are running, you can enter any node to test synchronization.

### 1. Access a node

Open a terminal and connect to the first container:
```sh
docker exec -it wormhole-whone-1 bash
```

(The container name may vary depending on the parent folder name; use `docker ps` to verify.)

### 2. Check status

Inside the container, check that the pod is online and connected:
```sh
wormhole status
wormhole tree
```

### 3. Test synchronization

The `whfolder` directory is now your shared space.

In **container 1**:
```sh
echo "Hello from node 1" > whfolder/hello.txt
```
Open another terminal and connect to **container 3**:

```sh
docker exec -it wormhole-whthree-1 bash
cat whfolder/hello.txt
```

You should see: `Hello from node 1`.

> [!CAUTION] Docker **Volumes** are not yet fully supported for data persistence on the host.
> Mounting a volume on `whfolder` may hide Wormhole's virtual filesystem.