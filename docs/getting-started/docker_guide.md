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

#### **4. Inspect the Containers**
```bash
docker inspect w1
docker inspect w2
```
- **Purpose**: Retrieve `w1` and `w2`s internal addresses for inter-container communication.
- **Key Data**:
  ```json
  "IPAddress": "172.19.0.2",
  ```
  ```json
  "IPAddress": "172.19.0.3",
  ```

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
docker exec -it w1 ./wormhole 172.19.0.2:8081 new test -p 7781
```
- **Purpose**: Creates a pod named `test` in `w1`'s network.
- **Expected Events**:
  - A `test` folder is created in `shared_mnt1`.
  - The `w1` service hosts a pod listening on port 7781

---



#### **5. Connect w2 to w1's Network**
```bash
docker exec -it w2 ./wormhole 172.19.0.3:8082 new test -p 7782 -- 172.20.0.2:7781
```
- **Purpose**: Join `w2` to the `test` network hosted by `w1`.
- **Expected Events**:
  - A `test` folder is created in `shared_mnt1`.
  - The `w1` service hosts a pod listening on port 7781

---

### Complete Workflow
```bash
# 1. Start services
docker-compose up
docker inspect w1 # → Get w1’s IP and port (e.g., GateWay:172.19.0.2)
docker inspect w2 # → Get w1’s IP and port (e.g., GateWay:172.19.0.3)

# 2. Configure w1 as the primary node
docker exec -it w1 ./wormhole 172.19.0.2:8081 template
docker exec -it w1 ./wormhole 172.19.0.2:8081 new test -p 7781

# 3. Configure w2 and connect it
docker exec -it w2 ./wormhole 172.19.0.3:8082 new test -p 7782 -- 172.20.0.2:7781
```
