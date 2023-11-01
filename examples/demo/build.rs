fn main() {
    let config = slint_build::CompilerConfiguration::new()
        .with_include_paths(vec![
            "../../nih_plug_slint/components/".into(),
        ]);

    slint_build::compile_with_config("main.slint", config).unwrap();
}
