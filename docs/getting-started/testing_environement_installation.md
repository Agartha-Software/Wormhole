# Wormhole CLI Usage Guide to setup a testing environment

This guide explains how to set up a new Wormhole network with multiple pods using the command line interface (CLI) on a single machine.

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) installed.
- **libfuse3-dev** installed (Linux only).
- The project must be built (`cargo build --release`).

## Step 1: Start the Service

Open a terminal and start the main Wormhole daemon. This service will manage all your pods.

```sh
# Keep this terminal open
./target/release/wormholed
```

## Step 2: Create virtual folders

Open a new terminal. Create three folders that will serve as mount points for your pods:

```sh
mkdir virtual1 virtual2 virtual3
```

## Step 3: Create the first Pod (The Network Origin)

Use the CLI to create the first pod named `pod1`. It will be mounted on the `virtual1` folder and listen on port 40001.

```sh
./target/release/wormhole new pod1 -m ./virtual1 -p 40001
```

> **Note**: Since no update URL (`-u`) is provided, this pod starts a new network.

## Step 4: Join the network with other pods

Create two additional pods (`pod2` and `pod3`). We use the `-u` flag to tell them to connect to `pod1` (which is at `127.0.0.1:40001`).

### For the second pod

```sh
./target/release/wormhole new pod2 -m ./virtual2 -p 40002 -u 127.0.0.1:40001
```

### For the third pod (connecting to pod2, creating a chain)

```sh
./target/release/wormhole new pod3 -m ./virtual3 -p 40003 -u 127.0.0.1:40002
```

## Step 5: Verify network connectivity

To test that all pods are properly connected, add a file to one pod and check if it appears in the others.

### Create a file in the first pod

```sh
echo "Hello World" > virtual1/testfile.txt
```

### Check the other folders (wait a second for sync)

```sh
cat virtual2/testfile.txt
cat virtual3/testfile.txt
```

You should see "Hello World" in all folders.

## Step 6: Inspect the Network

You can see the status of your pods and the file tree using the following commands:

```sh
# Show the file tree and where files are stored
./target/release/wormhole tree

# Inspect a specific pod
./target/release/wormhole inspect
```

## Cleaning up

To stop the test environment:

1. Remove the pods (this unmounts the folders):

```sh
./target/release/wormhole remove pod1
./target/release/wormhole remove pod2
./target/release/wormhole remove pod3
```

2. Stop the `wormholed` process (Ctrl+C).

