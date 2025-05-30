# memo cli for the noobz

Create 3 virtual folders (virtual, virtual2, virtual3)
for each :
```
RUST_LOG=wormhole=debug cargo run --bin wormhole-cli -- template -C virtual
```

Then create 3 services in 3 differents terminals (they will take 127.0.0.1:8081/2/3)
```
RUST_LOG=wormhole=debug cargo run --bin wormhole-service
```

Then, in one terminal
```
RUST_LOG=wormhole=debug cargo run --bin wormhole-cli -- 127.0.0.1:8081 new default -C virtual -i 127.0.0.10:8080
RUST_LOG=wormhole=debug cargo run --bin wormhole-cli -- 127.0.0.1:8082 new default -C virtual2 -i 127.0.0.11:8080 -u 127.0.0.10:8080
RUST_LOG=wormhole=debug cargo run --bin wormhole-cli -- 127.0.0.1:8083 new default -C virtual3 -i 127.0.0.12:8080 -u 127.0.0.10:8080
```
