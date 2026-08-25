# Runtime Launcher v4

`launch.sh` is a thin facade. It locates `.workdir/config/config.yaml` and delegates the runtime to the Kalesa binary with `--launch-config`.

The Rust runtime deserializes schema v3 with `serde_yaml`, validates the manifest, resolves paths relative to the game directory, validates the detected binary against the selected runner, applies `launch.env`, preserves `launch.args`, applies `launch.wrappers`, and appends arguments passed directly to `launch.sh`.

No YAML parsing is performed in Bash. The runtime does not use `yq`, `eval`, or `sh -c`.
