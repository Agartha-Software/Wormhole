# Getting Started

Follow these steps to set up a basic Wormhole network on your machine.

Wormhole uses two binaries:
 - "wormholed" the node managing the different pods
 - "womrhole" the command line interface, acting as an interface with the node

 Wormhole being still in heavy developpement, the project still require to build the project from source.

## Install
See the [install guide](./install.md). This is the simplest way to directly install Wormhole

## Build for source
If the [install guide](./install.md) does not cover your system.
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
wormholed
^--------
deamon   
```

Create a new Wormhole network
The new pod being created with any other connection it will automaticaly create a new network
```
./wormhole  new      my_pod    -m dir1/   -p 5555
^---------  ^--      ^-----    ^-------   ^-----------------
CLI helper  Command  Pod Name  optional   Pod port
                               mount path

```

Join an existing Wormhole network
```
./wormhole new my_pod2 -m dir2/ -p 5556 -u 127.0.0.1:5555
                                        ^-----------------
                                        Existing pod url
```

Check help menus to see more:
```sh
./wormhole --help # general help
./wormhole new --help # help for command "new"
```

### For easy testing, go check the [Docker Guide](docs/getting-started/docker_guide.md).

---

## CLI Commands Overview

To continue going forward, you can check the cli help menu:

```
Usage: wormhole [OPTIONS] <COMMAND>

Commands:
  new        Create a new pod and if possible join a network, otherwise create a new one
  inspect    Inspect the basic informations of a given pod
  get-hosts  Get the hosts of a given file
  tree       Display the file tree at a given pod or path and show the hosts for each files
  remove     Remove a pod from its network and stop it
  status     Checks if the service is working
  help       Print this message or the help of the given subcommand(s)

Options:
  -s, --socket <SOCKET>  Specify a specific service socket in case of multiple services running [default: wormhole.sock]
  -h, --help             Print help
  -V, --version          Print version
```

## More info
Both the client and daemon programs are fully documented, you can pass --help to any command or subcommand for more info:
```
wormhole --help
wormhole new --help

wormholed --help
```

## Configuration

You network can by configured futher by the configuration file.

You can configure the [local network configuration](../../docs/technical/configuration/local_conf.md) which is pod specific and not replicated.
Or you can configure the [global network configuration](../../docs/technical/configuration/global_conf.md) which is for the whole network and replicated.

> [!WARNING]
> /!\ Not all of theses configuration settings are implemented yet /!\
