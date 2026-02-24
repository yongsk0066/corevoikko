# docker

Ubuntu 24.04-based Docker container for running the original C++ libvoikko with the latest Finnish vocabulary. Useful for quick access to a working Voikko environment without local compilation.

Note: This container builds the legacy C++ libvoikko (via autotools), not the Rust implementation. For the Rust/WASM version, use `libvoikko/js/` (npm) or build from `libvoikko/rust/`.

## Usage

```bash
cd docker
docker compose up --build
docker exec -ti docker-voikkocontainer-1 python3 -c \
  'from libvoikko import Voikko; print(Voikko("fi").analyze("alusta"))'
```

## What It Builds

The Dockerfile installs foma, builds C++ libvoikko from the upstream master, compiles the Finnish dictionary with scientific vocabulary, and installs the Python module. The resulting image provides `libvoikko.so` and the Python `libvoikko` module ready to use.
