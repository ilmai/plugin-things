# Gain Plugin Example

A minimal example audio effect plugin demonstrating the `plugin-things` framework. 

## Building

> Append `--release` to any command below for a release build.

### Plugin Bundles (CLAP & VST3)

```sh
cargo xtask bundle gain-plugin
```

### Standalone App

```sh
cargo run -p gain-plugin --features standalone
```

### Standalone App with Live Preview

Hot-reload the Slint UI when modifying .slint UI files without recompiling the app.

```sh
SLINT_LIVE_PREVIEW=1 cargo run -p gain-plugin --features=standalone,slint/live-preview
```
