### Wormhole on Kubernetes (3 nodes)

This deploys one `wormholed` per node using a DaemonSet, with host networking so services communicate via node IPs. FUSE is enabled via `/dev/fuse` and `SYS_ADMIN` capability.

#### 1) Build and push the image
```bash

docker build -t ghcr.io/agartha-software/wormhole:latest .
docker push ghcr.io/agartha-software/wormhole:latest
```

#### 2) Apply manifests

Edit `k8s/wormhole.yaml` and set image: [ghcr.io/agartha-software/wormhole:latest](ghcr.io/agartha-software/wormhole:latest).

```bash
kubectl apply -f k8s/wormhole.yaml
```

This creates:

- Namespace `wormhole`
- Headless Service `wormhole`
- DaemonSet `wormhole` (one pod per node)

#### 3) Verify

```bash
kubectl -n wormhole get pods -o wide
kubectl -n wormhole get ds wormhole -o yaml | kubectl-neat
```

Each pod listens on `0.0.0.0:8081` and, with `hostNetwork: true`, is reachable on its node IP at port `8081`.

#### 4) Using the CLI

The image contains `wormhole-cli`. You can exec into a pod and run commands. For example:

```bash
POD=$(kubectl -n wormhole get pod -l app=wormhole -o jsonpath='{.items[0].metadata.name}')
kubectl -n wormhole exec -it "$POD" -- /bin/wormhole-cli 127.0.0.1:8081 status
```

To reach other nodes' daemons, use their node IP, e.g. `10.0.0.12:8081`.

#### 5) Persistent data

By default, the DaemonSet mounts `/var/lib/wormhole` from the node to `/wormhole` in the container. Adjust as needed.

#### 6) Requirements

- Nodes must support FUSE and expose `/dev/fuse`
- Container runtime must allow privileged pods with `SYS_ADMIN`

