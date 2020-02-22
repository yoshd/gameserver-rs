# gameserver-rs

This is a real-time game server for multiplayer running on Agones.

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
