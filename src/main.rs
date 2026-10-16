mod app;
mod config;
mod debug;
mod model;
mod picking;
mod renderer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    app::run(config::AppConfig::default())
}
