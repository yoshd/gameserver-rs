# gameserver-rs

This is a real-time game server for multiplayer running on Agones.

## How to run on minikube

```
# Install Agones on your Kubernetes cluster.
# https://agones.dev/site/docs/installation/

$ docker build . --tag=game-server
$ minikube cache delete game-server:latest && minikube cache add game-server:latest
$ cd k8s
# Start server
$ kubectl apply -f game-server/fleet.yml
$ kubectl get gs
# Allocate server
$ kubectl apply -f game-server/game_server_allocation.yml
$ kubectl get gs
```
