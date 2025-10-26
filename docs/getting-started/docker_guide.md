# Docker
We provide a sample Docker image and Docker compose to show you how it can be done. This setup is not ideal yet, and we are looking for a better way to do this. Kubernetes will soon be supported (in fact it should be already, but we didn't test it extensively).

The docker compose create a network composed of 3 pods

> [!WARNING]
> The docker compose use the `post_start` directive, requiring [docker compose 2.30.0](https://docs.docker.com/compose/how-tos/lifecycle/) and later.
> If you don't want to update, you can remove this directive and enter the commands manually.

You can go inside each one using
```sh
docker exec -it <container-name> bash
```
You will find a folder `whfolder` that is connected between the 3 containers.
