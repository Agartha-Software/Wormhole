# Getting Started with Wormhole

Follow these steps to set up a basic Wormhole network on your machine.

Wormhole comes with two binaries:

- `wormholed`: the daemon (service) that manages pods and storage.
- `wormhole`: the command-line interface (CLI) to interact with the daemon.

Since the project is still under active development, you need to build it from source.

## Installation

See the [Installation Guide](./install.md) for system-specific instructions.

If you need to build manually:

### Prerequisites

- **Rust** (latest stable).
- **libfuse3-dev** and **pkg-config** (on Debian/Ubuntu: `apt install libfuse3-dev pkg-config`).

### Building

Run the following command at the project root:

```sh
cargo build --release
```

The resulting binaries will be located at:

- `target/release/wormhole`
- `target/release/wormholed`

## Running and Usage

### 1. Start the service

Start the daemon in a dedicated terminal. It must remain running.

```sh
./wormholed
```

### 2. Create a network (First node)

In another terminal, create your first "Pod". If you do not provide a URL for another pod, this initializes a new network.

```sh
# Syntax: wormhole new <POD_NAME> -m <MOUNT_POINT> -p <PORT>
./wormhole new my_pod1 -m ./shared_folder1 -p 5555
```

### 3. Join an existing network

To connect another machine (or another folder on the same machine) to the network, use the `-u` (url) option to point to the first pod.

```sh
./wormhole new my_pod2 -m ./shared_folder2 -p 5556 -u 127.0.0.1:5555
```

> **Note**: If testing locally, make sure to use different ports and mount folders.

## CLI commands

Available commands via the `wormhole` tool. Use `--help` on any command for details.

| Command   | Description                                                              |
| ---------- | ------------------------------------------------------------------------ |
| new        | Create a new pod. Joins a network if `-u` is provided, otherwise creates one. |
| status     | Check if the `wormholed` service responds correctly.                     |
| inspect    | Show technical information about the current pod.                       |
| tree       | Show the file tree and where files are stored (hosts).                  |
| get-hosts  | Retrieve the list of machines that have a specific file.                |
| freeze     | Freeze the pod: prevent file modifications (read-only).                 |
| unfreeze   | Unfreeze the pod: allow modifications again.                            |
| remove     | Remove a pod from the network and stop it cleanly.                      |

### Example usage

```sh
./wormhole tree
./wormhole freeze
```

## Configuration

You can configure your network using TOML files.

> [!WARNING]
> Configuration is a work in progress. Some options may not yet be active.

### Example configuration file (config.toml)

Options currently supported by the code:

```toml
# Global configuration (shared on the network)
[general]
name = "MyWormholeNetwork"
# List of known entry points to join the network
entrypoints = ["127.0.0.1:5000", "192.168.1.15:5555"]

# Redundancy configuration
[redundancy]
# Number of copies of each file to keep on the network
number = 2
```

### Local configuration

The local configuration (LocalConfig) handles machine-specific settings, like hostname or public URL, but these settings are usually handled automatically when creating the pod via the `new` command.