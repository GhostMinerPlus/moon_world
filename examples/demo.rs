mod app;

fn main() {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info,wgpu=warn,demo=debug,moon_world=debug"),
    )
    .init();

    app::Application::new().run().unwrap()
}
