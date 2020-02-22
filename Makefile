all: build_gameserver build_frontend build_director build_mmf

build_base:
	docker build . -t gameserver-base -f Dockerfile.base

build_gameserver: build_base
	docker build . -t gameserver -f Dockerfile.gameserver

build_frontend: build_base
	docker build . -t frontend -f Dockerfile.frontend

build_director: build_base
	docker build . -t director -f Dockerfile.director

build_mmf: build_base
	docker build . -t mmf -f Dockerfile.mmf

minikube_cache_del:
	minikube cache delete gameserver:latest
	minikube cache delete frontend:latest
	minikube cache delete director:latest
	minikube cache delete mmf:latest

minikube_cache_add:
	minikube cache add gameserver:latest
	minikube cache add frontend:latest
	minikube cache add director:latest
	minikube cache add mmf:latest
