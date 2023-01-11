target=aarch64-unknown-linux-musl
ip=`ifconfig en0 | grep inet | cut -d " " -f 2`

.PHONY: clean
clean:
	docker images -f "dangling=true" | grep -v kindest | awk 'NR!=1{print $$3}' | xargs docker rmi

.PHONY: build
build:
	cargo zigbuild --release --target=${target} -p evolve_backend

.PHONY: image
image: build
	docker build --no-cache -f backend.dockerfile  --build-arg target=${target} --build-arg ip=${ip} -t yuexclusive/evolve_backend:latest .
	make clean

.PHONY: run
run: image
	docker run --rm -p 8881:8881 -it yuexclusive/evolve_backend:latest

.PHONY: image_nginx
image_nginx: build
	docker build --no-cache -f backend_nginx.dockerfile  --build-arg target=${target} --build-arg ip=${ip} -t yuexclusive/evolve_backend_nginx:latest .
	make clean

.PHONY: run_nginx
run_nginx: image_nginx
	docker run --rm -p 8881:80 -it yuexclusive/evolve_backend_nginx:latest

.PHONY: openapi
openapi:
	openapi-generator generate -i http://localhost:8881/api-doc/user.json -g rust -o ./.openapi