mod app;
mod model;
mod renderer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    app::run()
}
