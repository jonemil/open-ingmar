# Monkey C examples

Each directory is a standalone Connect IQ project containing its own manifest, Jungle configuration, source, resources, and build instructions. These are independently written reference inputs for Garmin's SDK and future Rust toolchain experiments.

- [hello-world](hello-world/README.md): minimal watch application showing a greeting and a numeric probe.
- [minimal-console](minimal-console/README.md): one target function calling `println("Hello!")`, with basic watch-app lifecycle scaffolding and no custom rendering.

Both example build scripts use [scripts/build-example.sh](../scripts/build-example.sh) for the SDK, Java, device-data, and key setup. Output names follow the example directory name.

Keep generated artifacts in each project's ignored `build/` directory. Signing keys and SDK installations stay outside the examples.
