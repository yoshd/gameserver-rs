![Rust](https://github.com/yoshd/gameserver-rs/workflows/Rust/badge.svg)

# gameserver-rs

This is a sample implementation of a game matchmaking system and real-time game server, written in Rust.

## Matchmaking

The matching system uses [Open Match](https://github.com/googleforgames/open-match) and implements the following components:

- [Game Frontend](https://open-match.dev/site/docs/guides/matchmaker/frontend/) implementation
  - `src/frontend`
- [Director](https://open-match.dev/site/docs/guides/matchmaker/director/) implementation
  - `src/director`
- [Match Function](https://open-match.dev/site/docs/guides/matchmaker/matchfunction/)
  - `src/mmf`
  - It matches players with a fixed number of players in the order they came in and assigns a game server running on the Agones

## Real-time game server

The implementation in `src/gameserver` is a real-time game server for multiplayer running on [Agones](https://github.com/googleforgames/agones).

## How to run on minikube

```
# Install Agones on your Kubernetes cluster.
# https://agones.dev/site/docs/installation/
# Install OpenMatch on your Kubernetes cluster.
# https://open-match.dev/site/docs/installation/

$ make
$ make minikube_cache_del
$ make minikube_cache_add
$ cd k8s
# Start gameserver
$ kubectl apply -k gameserver
$ kubectl get gs
$ Start matchmaker
$ kubectl apply -k matchmaker

# todo
# run example
$ cargo build --bin match-and-join
$ MM_SERVER_ADDR=$(minikube ip):$(kubectl get svc frontend -o jsonpath='{.spec.ports[0].nodePort}') ./target/debug/match-and-join
```
