# Wormhole CLI Usage Guide to setup a testing environement

This guide explains how to set up a new Wormhole network with multiple pods using the command line interface (CLI). The steps are designed to be simple and clear, requiring no consultation of external resources beyond this document.

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) installed.
- Optional: [Docker](https://docs.docker.com/get-docker/) for containerized deployment.

## Step 1: Create virtual folders

Create three virtual folders to simulate different pods on your machine:

```
mkdir virtual1 virtual2 virtual3
```

## Step 2: Generate configuration templates

For each virtual folder, generate a configuration template using the CLI:

```
wormhole -- template virtual1
wormhole -- template virtual2
wormhole -- template virtual3
```

These commands create configuration files in each folder.

## Step 3: Start Wormhole services

Open three different terminals and run the following command in each to start three Wormhole services. These services will listen on 127.0.0.1:8081, 127.0.0.1:8082, and 127.0.0.1:8083 respectively, as configured in their respective virtual folders.

```sh
wormholed
```

## Step 4: Create a new network

In a new terminal, create a network with the first pod using the following command:

```sh
wormhole 127.0.0.1:8081 new virtual1 -p 40001
```

This command initializes a network with a pod named "virtual1".

## Step 5: Join the network with other pods

Add the second and third pods to the network using the following commands:

For the second pod:
```sh
wormhole 127.0.0.1:8082 new virtual2 -p 40002 -u 127.0.0.1:40001
```

For the third pod:
```sh
wormhole 127.0.0.1:8083 new default virtual3 -p 40003 -- 127.0.0.1:40001 127.0.0.1:40002
```

These commands connect the pods to the network using the address of the first pod.

## Step 6: Verify network connectivity

To test that all pods are properly connected, add a blank file to one pod and check if it appears in the others.

For example, create a file in the first pod's folder:
```sh
touch virtual1/testfile.txt
```

Wait a few seconds for synchronization, then check the other folders:
```sh
ls virtual2
ls virtual3
```

You should see `testfile.txt` in both `virtual2` and `virtual3`. If the file appears in all folders, the network is functioning correctly.

## Note for advanced users

To create a third instance on another machine in the same local area network, follow similar steps, adjusting the IP addresses accordingly. For example:

1. On the other machine, create a virtual folder: `mkdir virtual3`
2. Generate the configuration template: `wormhole template virtual3`
3. Start the service: `wormholed`
4. Join the network: `wormhole new virtual3 -p <listening_port> -u <first_pod_address:first_pod_port>`, where `<listening_port>` is the port for this pod, `<first_pod_address>` is the address of the first pod, and `<first_pod_port>` is its port.

For instance, if the first pod is on 192.168.1.100:40001 and the third pod is on 192.168.1.101:40003, you would use:
```
wormhole new virtual3 -i 192.168.1.101:40003 -u 192.168.1.100:40001
```

**Note**: The original commands use loopback aliases (127.0.0.10, etc.), which work for pods on the same machine if configured appropriately. For a different machine, use its actual IP address.

## Conclusion

By following these steps, you have set up a functional Wormhole network with multiple pods and verified their connectivity. This process demonstrates a simple and clear onboarding for new users, without the need for external resources.
