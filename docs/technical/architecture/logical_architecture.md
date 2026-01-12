# Logical Architecture

This document describes the conceptual functioning of Wormhole. It abstracts away from code (Rust, functions) to focus on concepts, data flows, and interactions between system components.

This is the ideal document to understand "How does it work?" without having to read the source code.

## 1. Fundamental Concepts

To understand Wormhole, you need to master three key definitions:

### The Node

This is the physical machine (or virtual/container) on which the Wormhole software is installed.

- **Role**: Provide resources (CPU, Disk, Network).
- **Representation**: One instance of the `wormholed` daemon.

### The Pod

This is a "gateway" to the Wormhole network. In practice, it's a folder mounted on your computer.

- **Role**: Bridge between the user (local files) and the network (remote data).
- **Property**: A Node can host multiple Pods (e.g., one for work, one for personal).

### The Mesh (Network)

This is the set of all Pods connected to each other.

- **Architecture**: Decentralized (Peer-to-Peer). There is no "master server".
- **State**: All connected Pods share a "common truth" about the state of files.

## 2. The Layer Model

Wormhole can be visualized as a stack of layers, ranging from the user to the network cable.

```
+-------------------------------------------------------+
|   User Layer (Applications, File Explorer...)         |  <-- "I want to read file.txt"
+-------------------------------------------------------+
|   System Interface Layer (FUSE / WinFSP)             |  <-- "The kernel requests bytes"
+-------------------------------------------------------+
|   Logic Layer (File Tree & Virtualization)           |  <-- "Where is this file? Who has it?"
+-------------------------------------------------------+
|   Physical Storage Layer (Disk Manager)              |  <-- "Write to actual hard drive"
+---------------------------+---------------------------+
                            |
+---------------------------+---------------------------+
|   Network Layer (P2P Protocol)                       |  <-- "Send data to Pod B"
+-------------------------------------------------------+
```

### Detail of Layers

- **User**: For them, Wormhole is a normal folder. They use `ls`, `cp`, Word, or Photoshop without knowing Wormhole exists.

- **System Interface**: This is the translator. It intercepts calls from the kernel (Linux/Windows) and transforms them into commands Wormhole understands.

- **Logic (The Brain)**: This is where the magic happens. This layer maintains a map of all files (the file tree). It knows that `photo.jpg` is not on your disk, but on your colleague's.

- **Storage**: This layer manages the real files that are present on the machine.

- **Network**: Manages connections between machines to exchange data.

## 3. The Virtualization Concept (Metadata vs Data)

The greatest distinction in Wormhole's logical architecture is the separation between Metadata and Data.

### A. Metadata (The File Tree)

This is the folder structure, file names, permissions, modification dates.

- **Distribution**: Metadata is synchronized across all connected Pods.
- **Effect**: If you create an empty folder, it appears instantly everywhere because it only weighs a few bytes.

### B. Data (The Content)

This is the actual content of files (pixels in an image, text in a document).

- **Distribution**: Data is only stored where necessary (according to redundancy configuration).
- **Lazy Loading**: If a 10 GB file is added to the network, other Pods see the file appear (Metadata), but do not download the 10 GB (Data) until the user tries to open it.

## 4. Life Scenarios

Here's how the architecture responds to common actions.

### Scenario A: Reading a Remote File (Streaming)

The user double-clicks on `video.mp4` which is not stored on their machine.

1. **Interception**: The kernel asks Wormhole: "Open `video.mp4`".

2. **Lookup**: The Logic layer checks the file tree.

3. **Result**: "I know this file, but I don't have the local data. The data is on Pod Server_A."

4. **Connection**: Wormhole contacts Server_A via the Network layer.

5. **Transfer**: Server_A sends the requested data blocks.

6. **Caching**: Your Pod receives the data, writes it to its disk (for next time), and simultaneously sends it to the video player.

### Scenario B: Writing and Replication

The user saves a document `report.pdf`.

1. **Local Write**: The file is physically written to the user's hard drive (in a hidden folder).

2. **Notification**: The Logic layer tells the entire network: "Hey, I updated `report.pdf`!".

3. **Redundancy Strategy**:
   - If the configuration requires 2 copies of the file.
   - The Pod calculates who should store the second copy (e.g., Pod_B).
   - The Pod sends the content to Pod_B in the background.

4. **Result**: If your computer burns down, Pod_B has a complete copy of the file.

## 5. Topology and Discovery

How do Pods find each other?

- **Entry Point (Entrypoint)**: To join a network, a Pod must know the IP address of at least one other Pod already connected (the sponsor).

- **Gossip (Word of Mouth)**: Once connected to a peer, that peer shares their contact list. Very quickly, the new Pod knows the entire network.

- **Full Mesh**: Ideally, each Pod attempts to maintain an open connection with other Pods for maximum responsiveness, though the system can function in degraded mode if certain connections are impossible (NAT, Firewall).

## 6. Failure Management (Resilience)

The architecture is designed to be "Crash-Proof".

- **Pod Failure**: If a Pod disappears, others mark it as "Offline". Files it was the only one to possess become temporarily inaccessible (visible but unreadable).

- **Conflicts**: If two people modify the same file at the same time (or while offline), Wormhole detects the conflict. The logical architecture prioritizes data safety (nothing is deleted, the conflicting version is renamed).
