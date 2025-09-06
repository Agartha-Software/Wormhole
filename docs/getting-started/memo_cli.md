# memo for the cli usage

Create 3 virtual folders (virtual1, virtual2, virtual3)
for each :
```
RUST_LOG=wormhole=debug cargo run --bin wormhole -- template -m virtual1
```

Then create 3 services in 3 differents terminals (they will take 127.0.0.1:8081/2/3)
```
RUST_LOG=wormhole=debug cargo run --bin wormholed
```

Then, in one terminal
```
RUST_LOG=wormhole=debug cargo run --bin wormhole -- 127.0.0.1:8081 new virtual1 -p 40001
RUST_LOG=wormhole=debug cargo run --bin wormhole -- 127.0.0.1:8082 new virtual2 -p 40002 -u 0.0.0.0:40001
RUST_LOG=wormhole=debug cargo run --bin wormhole -- 127.0.0.1:8083 new virtual3 -p 40003 -u 0.0.0.0:40002
```
