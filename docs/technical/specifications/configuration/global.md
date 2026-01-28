# Global Network Configuration

> [!WARNING]
> /!\ Not all of theses configuration settings are implemented yet /!\

Main configuration for a Wormhole network.
This configuration defines the general behavior of the network and all its related information.

## Implemented Features

### General
>
> [!NOTE] [general]

> [!CAUTION] Mandatory
> **name**: string
> Short and simple name for the network.

**entrypoints**: list of strings
*default: []*
List of network URLs (addresses) to use to join the network.
> [!TIP]
> Essential for bootstrapping. You must provide at least one active peer URL here if you want to join an existing mesh.

**hosts**: list of strings
*default: []*
List of known hostnames of peers in the network. Helps in identifying and connecting to specific nodes.

---

### Redundancy
>
> [!NOTE] [redundancy]

> [!TIP]
> Redundancy is a very important parameter for securing data. This value defines the number of replications of a file across multiple nodes. Having at least one replica allows for:
> - Fault-tolerant data storage.
> In case of a node failure, no data is lost and the cluster will rebalance itself.
> Always-on system.
> - Even during rebalancing after a failure, data is still available and users maintain a seamless experience.

**number**: number
*default: 2*
Number of replicas for a file. Replicas are made for security and therefore stored on different nodes.
> [!WARNING]
> - Cannot exceed the number of active nodes.
> - Storage requirements increase linearly.

> [!TIP]
> The system will intelligently store replicas on nodes where the file is regularly requested to speed up the system :D

---

## Features Not Yet Implemented

> [!WARNING]
> /!\ Section Not implemented at this time /!\

### General (continued)
>
> [!NOTE] [general]

**access**: open | demand | whitelist | blacklist
*default: demand*
Defines how new pods should join the network.

---

> [!WARNING]
> /!\ Section Not implemented at this time /!\

### Network
>
> [!NOTE] [network]

**frequency**: seconds
*default: 0 (smart)*
Time during which outgoing write requests are stored locally before being sent all at once.
Prevents network flooding when creating many files rapidly.
> [!NOTE]
> A value of 0 lets the system manage itself, balancing on a base frequency of 1 second depending on current usage.

---

> [!WARNING]
> /!\ Strategy options not implemented at this time /!\

### Redundancy (continued)
>
> [!NOTE] [redundancy]

**strategy**: number
*default: 2*
Instantly replicating every change can cause a lot of unnecessary stress on the cluster. You can define a strategy based on your needs.

0. Instantly replicate all operations
If you can't afford to lose even one minute of data upon failure.
1. System managed
Will target inactivity periods for a file, preventing the propagation of too many minor writes when using a file.
Uses `min-replication-time` & `max-replication-time`.
1. Fixed
Replicates a file every `max-replication-time` (if the file has been modified since the last time).

---

**min-replication-time**: minutes
*default: 10*
> [!NOTE] Used by the redundancy strategy when system-managed.

Minimum time before re-propagating a backup when system-managed.

---

**max-replication-time**: minutes
*default: 120*
> [!NOTE] Used by the redundancy strategy when system-managed.Used by the redundancy strategy when fixed.

Maximum time before re-propagating a backup when system-managed.
