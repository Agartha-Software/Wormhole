# Wormhole

Wormhole is a data decentralisation solution. It aims to create one single virtual storage space between many computers.

You can think if it as the *[Kubernetes](https://github.com/kubernetes/kubernetes)* of storage space.

---

## Overview

Wormhole is an open-source project designed to provide a decentralized, scalable, and user-friendly data storage solution. By creating a virtual file system that spans multiple machines, Wormhole enables seamless data sharing and redundancy without the need for complex infrastructure management. Whether you're a small startup, a large enterprise, or an individual managing personal devices, Wormhole simplifies data storage and access with a native, intuitive interface. The storage space is integrated seamlessly in the usual files of your system.

This `README` provides an introduction to Wormhole, setup instructions, and links to detailed documentation. For a comprehensive understanding of the project's goals and technical details, refer to the [Technical Specification](docs/technical/technical_spec.md).

## Our Idea

Inspired by great declarative softwares of modern times like Docker, we are aiming to provide users with a very flexible solution, allowing all kinds of usages while staying declarative, simple and shareable.

## Concept

We want Wormhole to be as seamless as possible for final users. The storage space takes the shape of a simple folder. No need to create or mount any partition, nor use a gui to access it, the virtual space is mounted in place, where you want in your file tree.

For users and other softwares, the files behave like any normal files, while they are in fact shared and moved accross all nodes (differents computers) of the network.

## Features

- **Decentralized Storage**: Combine multiple machines into a single virtual storage space.
- **Native Integration**: Files appear as local files, requiring no changes to existing applications.
- **Scalability**: Suitable from small local networks to large enterprise infrastructures.
- **Redundancy**: Configurable data replication to ensure integrity and availability.
- **Flexibility**: Supports dynamic addition/removal of nodes without service interruption.
- **Configuration**: Declarative, file-based configuration using TOML for ease of use and sharing.

For detailed use cases and technical details, see the [Technical Specification](docs/technical/technical_spec.md).

## Documentation

The Wormhole documentation is organized into the following sections:

- **Getting Started**:
  - [Getting Started](docs/getting-started/getting_started.md): Step-by-step instructions for setting up Wormhole.
  - [Docker Guide](docs/getting-started/docker_guide.md): Instructions for running Wormhole in Docker containers.
- **User Guide**:
  - ~[Configuration Guide](docs/user-guide/configuration.md):~ How to configure Wormhole using TOML files. (need update)
  - [Glossary](docs/user-guide/glossary.md): Definitions of key terms and concepts.
- **Technical Documentation**:
  - [Technical Specification](docs/technical/technical_spec.md): Detailed explanation of Wormhole’s architecture and features.
  - [Technical Specification (French)](docs/technical/technical_spec_fr.md): French version of the technical specification.
  - [Configuration Details](docs/technical/configuration/): In-depth configuration options (main, pod, and file-level).
- **Beta Testing**:
  - [Beta Test Plan](docs/beta-testing/beta_test_plan.md): Scenarios and criteria for testing the beta version.
- **UML Diagrams**:
  - Located in [docs/uml/](docs/uml/): Visual representations of Wormhole’s architecture.

---

## Contributing

Wormhole is an open-source project, and we welcome contributions from the community! To get involved:

1. Read the [Technical Specification](docs/technical/technical_spec.md) to understand the project’s goals and architecture.
2. Check the [Beta Test Plan](docs/beta-testing/beta_test_plan.md) to see testing scenarios and provide feedback.
3. Report issues or suggest improvements via the [GitHub Issues](https://github.com/Agartha-Software/Wormhole/issues) page.
4. Submit pull requests with code contributions, following the guidelines in [Code Architecture](docs/technical/architecture/code_architecture.md).

For terminology, refer to the [Glossary](docs/user-guide/glossary.md) to understand key concepts like nodes, pods, and networks.

---

## Known Issues and Limitations

The current beta version has some known limitations, detailed in the [Beta Test Plan](docs/beta-testing/beta_test_plan.md). Key issues include:

- **Windows Support**: Incomplete, with some features not fully implemented.
- **Documentation**: Some sections are incomplete and being expanded.
- **Configuration**: The configuration files settings are not all implemented.
- **Stability**: Some bugs persistes due to the early state of the project, check the issues.

We are actively working on these issues and encourage community feedback to improve Wormhole.

---

## License

Wormhole is licensed under the [The GNU Affero General Public License](LICENSE.txt). See the license file for details.

---

## Acknowledgments

Wormhole is developed by Axel Denis, Julian Scott, Ludovic de Chavagnac, and Arthur Aillet. We thank all contributors and testers for their support in making Wormhole.
