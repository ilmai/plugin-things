vst3_path := if os() == "windows" { "C:/Program\\ Files/Common\\ Files/VST3/Dev/" } else if os() == "linux" { "~/.vst3/" } else if os() == "macos" { "~/Library/Audio/Plug-Ins/VST3/" } else { error("Unexpected OS") }
clap_path := if os() == "windows" { "C:/Program\\ Files/Common\\ Files/CLAP/Dev/" } else if os() == "linux" { "~/.clap/" } else if os() == "macos" { "~/Library/Audio/Plug-Ins/CLAP/" } else { error("Unexpected OS") }

default:
    @just --choose

[arg("target", pattern="debug|release")]
bundle-gain-plugin target="debug":
    cargo xtask-{{ target }} bundle gain-plugin {{ if target == "debug" { "" } else { "--release" } }}
    
[arg("target", pattern="debug|release")]
install-gain-plugin target="debug": bundle-gain-plugin
    cargo xtask-{{ target }} bundle gain-plugin {{ if target == "debug" { "" } else { "--release" } }}
    cp -rf ./target/bundled/gain-plugin.vst3 {{ vst3_path }}
    cp -rf ./target/bundled/gain-plugin.clap {{ clap_path }}

[arg("target", pattern="debug|release")]
run-gain-standalone target="debug":
    cargo run {{ if target == "release" { "--release" } else { "" } }} -p gain-plugin --features standalone

[arg("target", pattern="debug|release")]
run-gain-standalone-live target="debug" $SLINT_LIVE_PREVIEW="1":
    cargo run {{ if target == "release" { "--release" } else { "" } }} -p gain-plugin --features=standalone,slint/live-preview
