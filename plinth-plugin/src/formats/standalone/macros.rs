#[macro_export]
macro_rules! export_standalone {
    ($plugin:ty) => {
        fn main() {
            ::plinth_plugin::standalone::run_standalone::<$plugin>();
        }
    };
}
