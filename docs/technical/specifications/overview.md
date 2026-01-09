# Technical Specification
As explained in the project context, Wormhole is a **decentralized data storage** solution.  
This section of the document provides a brief explanation of what decentralization is and how this approach compares to others.  
The technical details of the features offered by the project and its technical stack will then be discussed.

## Decentralization (context - definition - utility)
Today, both small and large companies have significant data storage needs:
- **Internal data**
  - Company documents (internal cloud for employees)
  - Work-related data  
    > Assets for a video game studio  
    > Scientific datasets for a laboratory  
    > Training sets for artificial intelligence studios  
    > Big Data  
    > ... any data directly serving the company  
  - Sensitive data  
    > Accounts, quotes, and invoices (legal data)  
    > Customer-related data  
- **Data used by a software service offered by the company**  
  > Music for applications like Spotify/Deezer  
  > Videos for applications like YouTube/TikTok  
  > Various data stored for services like OneDrive/Google Drive  

These use cases are just examples but represent the needs of companies well-established in the digital era.  
**However, this need quickly encounters a physical limitation.**  
Indeed, it is impossible to concentrate infinite resources on a single server.  
Centralizing data on a single machine would also pose a data integrity issue in case of failure.  

**The need to increase the number of machines quickly arises to meet at least some of the following requirements:**  
- Need for massive storage capacity (more space)  
- Need for more power (faster data delivery)  
- Reliability / Crisis management  
  - Effortlessly withstand minor failures  
  - Follow its Business Continuity Plan (BCP) in case of a major incident  
- Facilitate access for all the company’s geographical sites  

> [!TIP] Business Continuity Plan  
> The **BCP** is a common practice for companies dependent on IT services.  
> Typically established by the IT department and relevant business units, it takes the form of a clear procedure for responding to the most likely serious incidents.  
> Wormhole does not create this plan for the company but provides the necessary parameters to comply with predefined procedures.  
> More information: [Wikipedia - Business Continuity Plan](https://en.wikipedia.org/wiki/Business_continuity)  

Increasing the number of machines for the same service is called decentralization, as opposed to centralization, which is limited to a single entity.  
Faced with this unavoidable need, companies have few solutions:  
- **Use an external cloud provider**  
  > This is the simplest solution.  
  > However, it is costly, and the company loses sovereignty over its data.  
  > This makes it impossible in some cases (sensitive data, data used by a software service, or specific needs).  
  > *Note that cloud services themselves use decentralization to secure data.*  
- **Semi-centralization (manual)**  
  > A solution that involves maintaining a primary centralized entity (server/server room) as much as possible, with a secondary offline entity for regular backups.  
  > In case of failure, the secondary entity is brought online as a replacement. It is also used to restore data to the primary entity once it is operational again.  
  > This strategy is more common in datacenter-scale infrastructures and is less accessible to medium-sized companies.  
  > It may also lead to service interruptions.  
- **Decentralization (manual)**  
  > **The ultimate solution**, addressing all the needs discussed.  
  > **However, there is no universal way to implement this solution. This is where Wormhole steps in,** offering a simple, open, and universal tool.  

> [!TIP] Wormhole aims to be the Kubernetes of disk space.  

## Our Solution: Wormhole
**Wormhole provides a simple and declarative solution for creating both simple and advanced decentralized infrastructures.**  
Wormhole creates a decentralized file system across all machines added to the network.  
Once mounted, this file system, natively integrated, is indistinguishable from other files on the machine.  
> [!NOTE] For a user, there is no difference between a local file folder and a Wormhole folder.  
> The same applies to software and applications; the files behave like normal local files, requiring no adaptation.  

### For Companies:
Tailored to large-scale needs, Wormhole enables the rapid creation of a powerful infrastructure:  
- **Massive**, free from the constraints of server centralization, with no growth limits.  
- **High-performing**, leveraging all available power optimally, avoiding unnecessary consumption.  
- **Secure** against data loss (even in case of failure).  
- **Uninterrupted**, even during failures or network modifications.  
- **Flexible**, allowing easy infrastructure changes without service interruption.  
- **Native**, requiring no adaptation of existing applications and services.  
- **Scalable**, suitable for all scales, from a startup’s local network to large international infrastructures.  

> [!IMPORTANT] The simple, clear, and declarative configuration minimizes human error.  
> Once launched, the experience is smooth and reliable for all services.  
> The network can be modified, with machines added or removed, without interrupting the service.  
> The company can easily define its data retention security policies and its [Business Continuity Plans](https://en.wikipedia.org/wiki/Business_continuity) to withstand minor and major incidents.  

> [!TIP] Scalable  
> Wormhole’s adaptive nature makes it suitable for a variety of use cases.  
> **Lightweight**, it does not require a powerful minimum configuration.  
> **Optimized**, it takes full advantage of the most capable servers.  

#### Example Use Cases (User Stories):  

> ➕**Cybersecurity Startup / SME**  
> Small team, no dedicated IT department to manage infrastructure.  
> Avoids external cloud providers to maintain data sovereignty.  
> Hosts data on a few (e.g., 3) small NAS servers.  
> - Wants to simplify data organization (currently scattered across different NAS servers).  
> - Wants to ensure data integrity in case of failure.  
> - Lacks time or a team to dedicate to data management (organization, backup, access, etc.).  
> - Desires a solution that can grow with the company.  
>  
> **Wormhole Solution:**  
> - Machines in a network are “merged.” For the end user, there is only one root (/) regardless of the number of individual machines. The user is free to create folders and organize as desired.  
> - The integrity configuration is comprehensive, allowing anticipation and reaction to unexpected events. Examples include:  
>   - The redundancy option stores the requested number of file copies across multiple machines. The more copies, the lower the risk of loss.  
>   - Crisis management options ([BCP](https://en.wikipedia.org/wiki/Business_continuity)) allow planning the approach if too many machines fail to maintain normal operation.  
> - Creating a network is quick, even for a beginner, and requires no maintenance once set up.  
> - Modifying a network does not require its deletion; it automatically rebalances when a machine is added or removed.  
>   This makes it easily adaptable to a growing infrastructure.  
<br>
___

> ➕**Laboratory**  
> Specialized team with powerful servers and machines, but IT is not their core business.  
> Conducts simulations and analyses, generating significant data flows.  
> Avoids external cloud providers due to performance requirements.  
> Owns powerful but specialized machines (GPU simulation computers, CPU analysis computers, massive storage servers).  
> - Has high performance needs.  
> - Wants multiple distinct machines to analyze the same dataset.  
> - Data is generated, analyzed, and deleted daily; loss in case of failure is not a concern.  
> - Has highly variable needs (ranging from a few GB to tens of TB) and wants to allocate resources daily.  
>  
> **Wormhole Solution:**  
> - Intelligently stores data where it is most in demand. Provides a caching system to speed up operations.  
> - Every machine in the network has access to the same dataset.  
> - The configuration allows complete optimization of the network for speed rather than long-term integrity.  
> - The speed and simplicity of setting up a network allow for creating, using, and deleting a network for a single use case.  
>   Additionally, keeping the configuration file allows recreating the network with a single command.  
<br>
___

> ➕**Web Service**  
> A rapidly growing company! This new social network allows sharing 3D scans instead of photos!  
> The network is unconventional but already has 10,000 regular users! Storing all these posts is resource-intensive!  
> - Has a growing need for storage space.  
> - Has contrasting performance needs. Resources should prioritize trending posts over older, rarely viewed ones.  
> - Requires uninterrupted service, even in case of failure.  
> - Has integrity requirements around the legal minimum (about 3 copies).  
>  
> **Wormhole Solution:**  
> - Utilizes all available resources and allows easy addition of new ones.  
> - Cache and affinity configuration distinguishes between fast (SSD) and massive (HDD) servers to optimize their potential.  
> - With the network installed across such a large number of servers, redundancy and automatic balancing make service interruptions or data loss virtually impossible.  

<br>
Once the system is set up, everything operates automatically, ensuring simple and seamless use.  
The file-based configuration is reusable and shareable. Its clarity makes it easy to understand and maintain, even years after implementation.  
The network’s flexibility makes it reliable, adaptable, and modifiable without complex measures.  

### For Individuals  
Wormhole’s **Flexible** nature makes it practical for individual use as well.  
Tired of searching for documents, photos, and projects across your NAS, desktop, and laptop?  
Set up a Wormhole network in minutes, and your devices become one. Your data is available on all as if it were local!  
> [!IMPORTANT] Once installed, you quickly forget Wormhole is even there.  
> Yet, the nightmare of searching for, syncing, or backing up data across devices is a thing of the past.  
> Wormhole does it all for you 😎  
> Laptop stolen? **You haven’t lost your data.**  
> NAS fails? **You haven’t lost your data.**  
> Desktop burns out?! **You haven’t lost your data!**  
> Got a new device? **One command, and it’s all handled.**  

___

## Specification  

### Native Interface  

For intuitive interaction with the network, data access is provided through a virtual folder mounted by Wormhole. This allows the same interaction methods as with any other file system. These virtual folders are enabled by native technologies such as FUSE (Linux) or WinFSP (Windows).  

### Universal Integration  

One of Wormhole’s priorities is to make the network accessible to as many devices as possible, ensuring the virtual disk is compatible with a wide range of workflows.  
Our primary objectives for the EIP are integration with the following platforms:  
- Linux  
- Windows  
- Mac  

> FUSE’s support for Android makes it an interesting secondary platform to implement.  

To simplify access for platforms not natively supported, a Docker image will be developed.  
This image will be provided with a Kubernetes configuration to facilitate integration into the existing world of distributed computing.  

### Configuration  

Our project aims to combine rapid setup with extensible configuration.  
To meet these goals, we opt for file-based configuration. This method has proven effective for services like Docker and Kubernetes, enabling sharing, reuse, and versioning.  
We plan to use the TOML format, which combines clarity and modernity and is well-integrated into the Rust ecosystem.  

The configuration is designed to be as comprehensive as possible to modulate all aspects of the network. It operates on multiple levels:  
- Network level for general behavior.  
- Pod level with local information and pod-specific affinities.  
- File level to specify exceptions in their behavior.  

Below is a list of example configuration fields available to the user.  
This list is neither exhaustive nor definitive. Our goal is to allow configuration of everything possible, which is why most specific configuration fields will be defined during the project.  

> **General Configuration:**  
> - Unique network name  
> - Number of redundancies per file  
> - Node addition strategy (accepting new nodes)  
> - Maximum storage size offered  
> - Administration (who can modify the general configuration)  
> - Failure strategy  
> - If it does not affect operation or integrity  
> - If it affects integrity (lack of redundancies, but no files lost)  
> - If it affects operation (missing files)  

> **Pod Configuration:**  
> - Storage space limit  
> - Local cache (propensity to keep local copies for faster access)  
> - Affinities (prioritize or avoid a pod for a task)  
> - Storage of redundancies  
> - Storage of new files  
> - Storage of most-requested files  
> - Storage of least-requested files  
> - Local failure strategy (reaction if disconnected from the network)  

> **File Configuration:**  
> - Keep (force this pod to retain a local version)  
> - Do not cache  
> - Read-only  
> - Number of redundancies  

Many configuration options are available to the user. To simplify their definition, we have chosen to follow the same approach as Docker and Kubernetes with file-based configurations, specifically in the TOML format for its modernity and integration with the Rust ecosystem.  

The configuration operates on multiple levels: at the network level for general settings, at the machine level for local information and pod-specific affinities, and at the file level to specify exceptions in their behavior.  

### Data Distribution  

With Wormhole, when reading a file that is not locally present on the machine, the data is downloaded from the host machine on the fly.  
This offers several possibilities:  
- Act remotely on the file during the entire process (streaming).  
- Create a local copy of the file during use before exporting updates to the network.  
- Remote action is slower (latency) and uses bandwidth but has the benefit of not consuming disk space.  
- Using a local copy consumes disk space but provides enhanced performance.  
- The extensible configuration allows the user to customize this behavior (and other similar behaviors).  

> It is also important to note that, automatically, Wormhole stores files on the nodes requesting them most frequently, optimizing the entire system.  

With Wormhole, when reading a file not present on the machine, the data is downloaded from the host machine. This presents the option to either stream the file content directly or save it before transmitting the content. One option consumes more network bandwidth, while the other uses more disk space. This balance can be chosen by the user, ranging from full streaming, full saving, or a middle ground based on read frequency and/or file size.  

### Management Strategies (fault tolerance, redundancy and integrity, performance…)  

Data management is a complex issue, especially for large infrastructures like those Wormhole can operate. It’s no surprise that companies dedicate entire teams to this topic.  

As requirements can vary greatly depending on the use case, Wormhole allows configuring strategies to address different issues.  

#### Data Conflicts:  

Simultaneous modification of the same file by multiple nodes can cause conflicts. There is no perfect, universal conflict resolution method.  
The user can choose from a list of strategies, including (but not limited to):  
- Overwrite (keep the last written version)  
- Keep two copies  

Multiple copies of a file can lead to conflicts during simultaneous modifications, so conflict resolution is configurable. Either the most recent version of the file is kept, or a copy with the older modifications is retained alongside the original file, allowing the user to resolve conflicts manually.  

##### Data Integrity and Uninterrupted Service (general case):  

Ensuring data integrity during failures is generally critical. Distributing file copies across different network machines guarantees their integrity in case of failure.  
Not only that, but this replication allows the network to continue operating without interruption or file loss, even temporarily.  

This process, called redundancy, has the drawback of consuming significant disk space.  
Depending on their use case, the user can enable or disable this process and choose the number of replicas per file.  
Generating a large number of copies can be resource-intensive for the cluster. The user can therefore adjust the frequency of copy updates.  

##### Integrity and Business Continuity (crisis case):  

Decentralization and the use of redundancy greatly reduce the likelihood of major incidents.  
However, Wormhole allows defining strategies to adopt in case of widespread failure.  

##### Situations are divided into three categories:  

- **Favorable situation:**  
No file loss, the cluster has enough space to rebalance and recreate missing redundancies.  
Covered in the data integrity and uninterrupted service (general case) section.  

- **Mixed situation:**  
No file loss, but the cluster lacks space to rebalance and recreate necessary redundancies.  

- **Critical situation:**  
Missing files on the network, normal operation disrupted.  

For each situation, the user can configure an appropriate response.  

**Examples of responses (non-exhaustive):**  
- Slow down/limit traffic  
- Freeze the network (read-only) until the issue is resolved or administrator action is taken  
- Reduce the number of redundancies to increase free space and continue service as much as possible  
- Stop everything  

A key element in data backup is redundancy. Distributing copies of backed-up data across the network ensures their safety in case of an issue with one of the disks.  
In the configuration, the user can enable this and define the number of file replications, either globally or per folder/file.  

Multiple copies of a file can lead to conflicts during simultaneous modifications, so conflict resolution is configurable. Either the most recent version of the file is kept, or a copy with the older modifications is retained alongside the original file, allowing the user to resolve conflicts manually.  

### Optimization and Load Balancing  

The decentralized mesh structure pools capacities and offers great prospects for performance optimization.  
The system can “intelligently” manage its infrastructure, for example:  
- Place files and their redundancies on the nodes using them most frequently.  
- Parallel transfers (downloading different parts of the same file from two or more nodes, doubling the transfer speed. The same applies to uploads).  
- Distribute heavy operations. For example, if the number of redundancies is high, each node transfers to only two others, which do the same, and so on, preventing a single node from handling all transfers.  

The user can also adjust their needs to reduce network strain.  

**Example:**  
Reduce the frequency of file replication to avoid propagating resource-intensive operations across the cluster for each edit.  

The mesh distribution pools network capacities, opening up many optimization possibilities. For example, to optimize data transfers:  
- Place file replicas on nodes with the best network speed.  
- If a file being downloaded is present on multiple machines, each machine can send a portion of the file, significantly increasing upload speed.  
- With a replication number greater than 2, the user’s pod uploads once to a “server” pod, and the “server” pods handle the remaining replications among themselves. This quickly frees up the user’s network load.  

### Handling Absent Pods  

Network connectivity is an uncertain factor, so it’s critical to handle pod disconnections.  

At the cluster level:  
- Rebalance the replication load among the remaining pods.  
- Disable reading of absent files.  

At the disconnected pod level:  
- Inform the user.  
- Simple reaction (e.g., freeze).  

- **Seamless node addition/removal** (when it does not compromise data integrity)  
> Wormhole aims to fully exploit the flexibility offered by decentralization.  

- **Passive pods (portals/clients)**  

- **Flexibility and additional features**  
- The cluster can be modified without interruption, facilitating evolution and enabling:  
- Adding new nodes  
- Removing nodes  
- Modifying the configuration  

The cluster automatically rebalances based on the new context without disrupting services that may depend on the data.  

It is also possible to create so-called “Client” pods. These can access cluster files without becoming part of the network mesh.  
They can connect or disconnect on the fly without disrupting the system, making them suitable for large-scale deployment.  
(For example, employee laptops in a company.)