## Wormhole on Kubernetes - 3 Pods Connected

This guide shows how to deploy 3 Wormhole pods on Kubernetes and connect them together using the CLI inside each pod.

### Prerequisites
- kubectl configured to your cluster
- A Kubernetes cluster (kind, k3s, minikube, etc.)
- The container image [ghcr.io/agartha-software/wormhole:latest](ghcr.io/agartha-software/wormhole:latest) accessible from the cluster
  - If private, a GitHub token with `read:packages` and a Kubernetes imagePullSecret

### Why create a dedicated cluster ([kind](https://kind.sigs.k8s.io/)
Using a local, dedicated cluster (with kind) gives:
- Isolation: avoids interfering with other workloads/contexts on your machine
- Reproducibility: same kube version/behavior across dev machines
- Networking parity: simulates pod-to-pod networking and DNS the way a real cluster does
- Easy cleanup: delete the cluster and you are back to a clean slate

### 0) Create a local cluster with kind (recommended for dev)
```bash
# Create a local Kubernetes cluster named "wormhole"
kind create cluster --name wormhole

# Check the context points to the new cluster
kubectl cluster-info --context kind-wormhole

# Optional: if you plan to use a locally built image instead of GHCR, load it into kind
# kind load docker-image ghcr.io/agartha-software/wormhole:latest --name wormhole
```

### 1) Create the imagePullSecret (only if GHCR image is private)
```bash
kubectl -n wormhole create secret docker-registry ghcr-creds \
  --docker-server=ghcr.io \
  --docker-username=<GITHUB_USERNAME> \
  --docker-password=<GITHUB_PAT_with_read:packages> \
  --docker-email=<you@example.com>
```

The manifest `k8s/wormhole.yaml` already references:
- image: `ghcr.io/agartha-software/wormhole:latest`
- `imagePullSecrets: [ { name: ghcr-creds } ]`
- a headless Service `wormhole` for pod DNS
- privileged container with `/dev/fuse` (required)

### 2) Deploy
```bash
kubectl apply -f k8s/wormhole.yaml

# If you changed immutable fields of the StatefulSet in a previous run, delete and re-apply:
# kubectl -n wormhole delete statefulset wormhole
# kubectl apply -f k8s/wormhole.yaml

kubectl -n wormhole get pods -w
```

You should see:
```
wormhole-0   1/1   Running
wormhole-1   1/1   Running
wormhole-2   1/1   Running
```

### 3) Create a pod (filesystem) on wormhole-0
Exec into `wormhole-0`, create a mount directory, and create the first pod (listening on port 5000):
```bash
kubectl -n wormhole exec -it wormhole-0 -- /bin/bash
mkdir -p /wormhole/whfolder
wormhole new pod1 -p 5000 -m /wormhole/whfolder
```

Expected output ends with:
```
Pod 'pod1' created with success with port '5000'.
```

### 4) Connect wormhole-1 to wormhole-0
Important: The CLI requires an IP address for `-u`, not a DNS name.

Get the IP of `wormhole-0` (from within the cluster DNS):
```bash
# Exec into any wormhole pod (example with wormhole-1)
kubectl -n wormhole exec -it wormhole-1 -- /bin/bash

# Inside the container
PEER_IP=$(getent hosts wormhole-0.wormhole | awk '{ print $1 }')
echo $PEER_IP
```

Exec into `wormhole-1`, create a mount directory, then join using the IP of `wormhole-0` on port 5000:
```bash
kubectl -n wormhole exec -it wormhole-1 -- /bin/bash
mkdir -p /wormhole/whfolder
wormhole new pod2 -p 5000 -m /wormhole/whfolder -u $PEER_IP:5000
```

You should see `Pod 'pod2' created with success ...`.

### 5) Connect wormhole-2 to wormhole-0 (same pattern)
```bash
kubectl -n wormhole exec -it wormhole-2 -- /bin/bash
mkdir -p /wormhole/whfolder
wormhole new pod3 -p 5000 -m /wormhole/whfolder -u $PEER_IP:5000
```

### 6) Notes and verification
- The service daemon runs as: `wormholed -i 0.0.0.0:8081`.
- Headless Service `wormhole` provides DNS like `wormhole-0.wormhole`, but the CLI currently requires an IP literal for `-u`.
- Verify files in the mountpoint path inside each container (e.g. `/wormhole/whfolder`).

### Troubleshooting
- Image pull errors:
  - Ensure `ghcr-creds` is created and referenced in the manifest
  - Ensure the PAT has `read:packages`
- CrashLoopBackOff right after start:
  - The daemon requires the `-i` flag for the listen address (already set in the manifest)
  - The container must be privileged and mount `/dev/fuse` from the host
- Join fails with `InvalidUrlIp`:
  - Use `-u <IP:PORT>` with the IP of `wormhole-0` from `kubectl get pod -o wide`, not a hostname

### Cleanup
```bash
kubectl delete -f k8s/wormhole.yaml
```


