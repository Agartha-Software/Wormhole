# Docker
We provide a sample Docker image and Docker compose to show you how it can be done. This setup is not ideal yet, and we are looking for a better way to do this. Kubernetes will soon be supported (in fact it should be already, but we didn't test it extensively).

The docker compose image create two instances with `wormholed` already started.
In each of them, you can run
```sh
docker exec -it <container-name> bash
```
Then you will be able to use the `wormhole` command as desired.

You can of course edit your docker image to also include commands to start and connect a pod.