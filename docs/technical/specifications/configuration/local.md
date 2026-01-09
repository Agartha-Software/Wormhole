# Local Pod Configuration

> [!WARNING]
> /!\ Not all of these configuration options are implemented yet /!\

This configuration is specific to the pod.<br>
The cluster will comply with it if possible, but may override it if necessary (see [emergency strategies](../strategies/emergency.md)).<br>
This configuration is mainly used to adjust the cluster strategy at the local level.

> [!WARNING]
> Heterogeneous cluster configuration is considered an advanced setup. This can lead to bottlenecks, decreased performance, or insecure data management. We recommend letting the system manage itself if you do not have specific knowledge or needs.

## Implemented Features

### General
> [!NOTE] [general]

**hostname**: string<br>
*default: machine's hostname*<br>
Hostname that this pod will use to identify itself on the network. Must be unique within the network.

---

**url**: string (Optional)<br>
*default: value of `hostname`*<br>
URL (usually `hostname:port` or `ip:port`) that this pod communicates to others so they can reach it.

---

## Features Not Yet Implemented

> [!WARNING]
> /!\ Section Not implemented at this time /!\

### Storage
> [!NOTE] [storage]

**max-disk-space**: MB<br>
*default: 95% of the node's available disk space at the pod's mount point.*<br>
This size **cannot** be exceeded by Wormhole's actions.<br>
The cluster will take this information into account when choosing a pod to store data.
> [!TIP]
> - Can be exceeded if the cluster compensates for a failure (e.g., storing redundancies). [More info](../strategies/emergency.md)
> - Can be temporarily exceeded if a local user retrieves a lot of data into the mount point.
> - Can be temporarily exceeded during large data movements.
> - Will not be exceeded if the user pushes too much data for the cluster. The user will be alerted of reaching maximum capacity.
> - Will not be exceeded if the administrator creates a policy that generates too much data (e.g., increasing redundancy). The administrator will be alerted of the unachievable rule.
> [!NOTE]
> Can be temporarily exceeded when the user loads a new local file into the system, during the time it takes to be offloaded to the cluster.
> [!IMPORTANT]
> If a requested file is too large to be retrieved when requested, the pod will have to offload local data to the cluster, resulting in increased response time. If the cluster [runs out of space] for this data transfer, the user will not be able to access this file.

---

> [!WARNING]
> /!\ Section Not implemented at this time /!\

### Strategy
> [!NOTE] [strategy]

**redundancy-priority**: number<br>
*default: 0*<br>
When choosing a pod to store redundancy, pods higher priority will be used first.

---

**cache**: number<br>
*default: 2*<br>

0. unload all
1. system managed light preset
2. system managed heavy preset
3. download all

> [!NOTE]
> This parameter is more useful for the node using the file system than for the cluster.<br>
> Nevertheless, having more cache can give the system more freedom when retrieving data and help achieve better cluster performance.