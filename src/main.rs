mod app;
mod debug;
mod model;
mod renderer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    app::run()
}
