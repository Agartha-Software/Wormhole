# Code Architecture

This document guides developers (new and experienced) through the Wormhole codebase. It explains how different components interact to create the distributed file system.

## 1. Overview (Big Picture)

Wormhole operates on a Local Client-Server architecture:

- **The Service (`wormholed`)**: The heart of the system. It runs in the background as a daemon, manages the P2P network, maintains virtual file systems (Pods), and handles configuration.

- **The Client (`wormhole`)**: The user interface (CLI). It doesn't do anything by itself except send commands to the Service via a local socket (IPC).

```
User  --->  [ wormhole (CLI) ]
                  |
            (IPC / Unix Socket or TCP)
                  |
                  v
           [ wormholed (Service) ]  <=====>  (Internet / P2P)  <=====> Other Nodes
                  |
       +----------+----------+
       |                     |
   [ Pod A ]             [ Pod B ]
       |                     |
  (Mount Point)        (Mount Point)
```

## 2. Source Code Structure (src/)

The organization of the source code follows a modular logic:

- **`bin/`**: Entry points for executables.
  - `cli.rs`: Client code.
  - `service.rs`: Daemon code.

- **`cli/`**: Command-line argument definitions (using clap).

- **`config/`**: Configuration management (TOML parsing, structure definitions for GlobalConfig, LocalConfig).

- **`ipc/`**: Inter-Process Communication. All code enabling the CLI to communicate with the Service.

- **`network/`**: Peer-to-Peer network management (TCP, Handshake, Protocol messages).

- **`pods/`**: Core logic. Contains all logic for managing a shared storage space.
  - `disk_managers/`: Abstraction for physical disk writes.
  - `filesystem/`: FUSE operation implementations (read, write, lookup, etc.).
  - `network/`: Synchronization logic specific to a pod.

- **`fuse/` & `winfsp/`**: Low-level wrappers for kernel interaction (Linux/Windows).

## 3. Major Components

### A. The Service (`wormholed`)

The entry point is `src/bin/service.rs`. Its role is to initialize the tokio runtime (for async), load the configuration, and launch two main loops:

- **IPC Server**: Listens to local commands (e.g., `wormhole new ...`).
- **Pod Manager**: Maintains all active Pod instances.

### B. IPC (Inter-Process Communication)

Code is located in `src/ipc/`. Wormhole uses a request/response approach serialized via bincode.

- **Commands**: Defined in `src/ipc/commands.rs`. These are orders (e.g., NewPod, GetHosts).
- **Responses**: Defined in `src/ipc/answers.rs`. These are returns (e.g., Success, Error, PodInfo).

### C. The Pod (`src/pods/`)

A Pod represents a Wormhole shared folder mounted on the system. It's the most complex object. It orchestrates four vital sub-components:

#### 1. The System Interface (`filesystem/`)

This is the "facade" of the Pod. It receives system calls from the kernel (via FUSE or WinFSP).

Example: A user runs `cat file.txt`.

The filesystem module receives the read call. It asks the Tree where the data is located.

#### 2. The File Tree (`pods/itree.rs`)

This is the "brain" of the Pod.

- It keeps the file structure in memory (the inode/name tree).
- It knows if a file is present locally or if it's remote.
- It maintains metadata (permissions, size).

#### 3. The Disk Manager (`disk_managers/`)

This is the "hand" of the Pod.

- It abstracts physical storage.
- It writes real files in a hidden folder (`.wormhole/` at the mount point root).
- There is a `UnixDiskManager` and a `WindowsDiskManager` version.

#### 4. The Network Manager (`pods/network/`)

This is the "voice" of the Pod.

- If the Tree says "this file is not here, the neighbor has it", the Network Manager contacts the peer to download the data.

## 4. Data Flow: Reading a File (Example)

To understand how everything fits together, let's follow the path of a read request for a file that is not present locally (Lazy Loading).

1. **System Call**: An application (e.g., `cat`) requests to read bytes 0-100 of file X.

2. **FUSE/WinFSP**: Intercepts the call and transmits it to Wormhole (`src/pods/filesystem/read.rs`).

3. **Pod (Logic)**:
   - The Pod checks the Tree: "Do I have the content of file X?"
   - Tree response: "No, but Peer_B has it."

4. **Retrieval (Network)**:
   - The Pod pauses the read request (await).
   - It asks the NetworkManager to retrieve file X from Peer_B.
   - Peer_B sends the data via TCP (`src/network/`).

5. **Storage (DiskManager)**:
   - Once received, the DiskManager writes the data to the local cache.

6. **Response**:
   - The read request resumes.
   - The DiskManager reads the freshly arrived local data.
   - Wormhole returns the bytes to the application.

## 5. Technical Specifics

- **Async**: The entire project relies on tokio. The file system, though traditionally synchronous, is managed here asynchronously to avoid blocking the service during network requests.

- **Serialization**: `serde` is used everywhere to transform Rust structures into binary packets (for network/IPC) or text (for TOML config).

- **Error Handling**: We use a custom `WhError` type (`src/error.rs`) that propagates errors across layers (from disk to network to user).

## 6. Code Conventions

- **Logging**: Use `log::info!`, `log::warn!`, `log::debug!`, or `log::trace!` abundantly. This is the only way to debug the daemon.

- **Modules**: Each folder has a `mod.rs`. It should properly expose public types and hide internal implementation.

- **Tests**:
  - **Unit Tests**: In the same file as the code, in a `tests` module.
  - **Functional Tests**: In the `tests/functionnal/` folder. These tests launch real pods in separate threads.