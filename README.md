# Wormhole
Wormhole is a data decentralisation solution. It aims to create one single virtual stockage space between many computers.

You can think if it as the *Kubernetes* of storage space.

## Our idea
Inspired by great declarative software of modern times like docker, we are aming to provide users with a very flexible solution, allowing all kinds of usages while staying declarative, simple and shareable.

## The concept
We want Wormhole to be as transparent as possible for final users. The storage space take shape of a simple folder. No need to create or mount any partition, the virtual space is mounted in place, where you want in your file tree.

For users and other softwares, the files behaves like any normal files, while they are in fact shared and moved accross all nodes (differents computers) of the network.

## How to launch
(this is for demo purposes and will be changed)

The connection between the cli and the service is made by default on the address 127.0.0.1:8081 but can be modified by adding the new address in the command just after the binary (to be done for both)

```
cargo run --bin wormholed
^---------------------
Build and run
```

Create a new Wormhole network
```
cargo run --bin wormhole new pod_name -C virutal1/ -i 127.17.0.1:8081
^---------------------       ^--                ^-----       ^---------------
Build and run                 command            directory     host ip
```

Join an existing Wormhole network
```
cargo run --bin wormhole new pod_name2 -C virutal2/ -i 127.17.0.2:8081 -u 127.17.0.1:8081 -a 127.17.0.3:8081 127.17.0.4:8081 127.17.0.5:8081
^---------------------       ^--                ^-----       ^----------       ^-------------     ^-----------------------------------------------
Build and run                 command            directory     host ip         ip of node to join     additionnal host
```



## Guide to Using Docker Images and Test Commands (Not up to date)

#### **1. Start the Infrastructure**
```bash
docker-compose up
```
- **Purpose**: Launches the `wormhole1` and `wormhole2` services in the background.
- **Expected Events**:
  - Containers `w1` and `w2` start with their volumes (`shared_mnt1`, `shared_mnt2`).
  - Services listen on ports `8081` (w1) and `8082` (w2).

---

#### **2. Create a Network Template on w1**
```bash
docker exec -it w1 ./wormhole template
```
- **Purpose**: Initializes a default network configuration in `shared_mnt1/.global_config.toml`.
- **Expected Result**:
  ```bash
  creating network "default"
  Network configuration created at /usr/src/wormhole/virtual/.global_config.toml
  ```

---

#### **3. Create a New Pod on w1**
```bash
docker exec -it w1 ./wormhole new test
```
- **Purpose**: Creates a pod named `test` in `w1`'s network.
- **Expected Events**:
  - A `test` folder is created in `shared_mnt1`.
  - The `w1` service becomes the primary network node.

---

#### **4. Inspect the w2 Container**
```bash
docker inspect w1
```
- **Purpose**: Retrieve `w1`'s internal IP for inter-container communication.
- **Key Data**:
  ```json
  "GateWay": "172.19.0.3",
  ```

---

#### **5. Connect w2 to w1's Network**
```bash
docker exec -it w2 ./wormhole new test 172.20.0.3:8081
```
- **Purpose**: Join `w2` to the `test` network hosted by `w1`.
- **Expected Result**:
  ```bash
  Pod "test" joined network via 172.20.0.3:8081
  Syncing with peer... OK
  ```

---

### Complete Workflow
```bash
# 1. Start services
docker-compose up

# 2. Configure w1 as the primary node
docker exec -it w1 ./wormhole template
docker exec -it w1 ./wormhole new test1

# 3. Configure w2 and connect it
docker inspect w1 # → Get w1’s IP and port (e.g., GateWay:172.20.0.3)
docker exec -it w2 ./wormhole new test2 172.20.0.3:8081
```
