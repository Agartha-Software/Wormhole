# Getting Started

Follow these steps to set up a basic Wormhole network on your machine.

Wormhole uses two binaries:
 - "wormholed" the node managing the different pods
 - "womrhole" the command line interface, acting as an interface with the node

 Wormhole being still in heavy developpement, the project still require to build the project from source.

## Install
See the [install guide](docs/getting-started/install.md). This is the simplest way to directly install Wormhole

## Build for source
If the [install guide](docs/getting-started/install.md) does not cover your system.
### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) installed.
- Clone the source code.
- Optional: [Docker](https://docs.docker.com/get-docker/) for containerized deployment.

### How to build

Run cargo build command:
```
cargo build --release
```

Move the binaries where needed, they can be found under `target/release/wormhole` and `target/release/wormholed`

## How to run

Launch a new service, the node is started automatically
```
./wormholed 127.0.0.1:8081
^---------- ^-------------
Node     Optional address with a default at 127.0.0.1:8081
```

Create a new Wormhole network
The new pod being created with any other connection it will automaticaly create a new network
```
./wormhole 127.0.0.1:8081            new      pod_name  --path dir1/ -i 127.0.0.10:8081
^--------- ^-------------            ^--      ^-----    ^-------        ^-----------------
 CLI       Optional node address     Command  Pod Name  Directory       Pod Address
```

Join an existing Wormhole network
```
./wormhole 127.0.0.1:8081 new pod_name2  --path dir2/ -i 127.0.0.11:8081 -u 127.0.0.10:8081
                                                                            ^-----------------
                                                                            Existing pod address
```

For a step-by-step guide to setting up a simple multi-pod network, see the [CLI Usage Guide](docs/getting-started/memo_cli.md).
For a more complex Docker-based deployment, refer to the [Docker Guide](docs/getting-started/docker_guide.md).

---

## CLI Commands Overview

To continue going forward, you can check the available cli commands:

```
  start        Start the service
  stop         Stop the service
  template     Create a new network (template)
  new          Create a new pod and join a network if he have peers in arguments or create a new network
  get-hosts    Get hosts for a specific file
  tree         Tree the folder structure from the given path and show hosts for each file
  remove       Remove a pod from its network
  apply        Apply a new configuration to a pod
  restore      Restore many or a specifique file configuration
  help         Print this message or the help of the given subcommand(s)
```

## Configuration

You network can by configured futher by the configuration file.

You can configure the [local network configuration](../../docs/technical/configuration/local_conf.md) which is pod specific and not replicated.
Or you can configure the [global network configuration](../../docs/technical/configuration/global_conf.md) which is for the whole network and replicated.

> [!WARNING]
> /!\ Not all of theses configuration settings are implemented yet /!\
