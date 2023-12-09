fn main() {
    let config = slint_build::CompilerConfiguration::new()
        .with_library_paths([
            ("parameter".into(), "../../nih_plug_slint/slint/parameter.slint".into()),
        ].into());

    slint_build::compile_with_config("main.slint", config).unwrap();
}
