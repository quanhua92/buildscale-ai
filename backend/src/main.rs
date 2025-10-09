use backend::load_config;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Hello from the binary!");

    // Load configuration using lib.rs method
    let config = load_config()?;

    // Print configuration using Display implementation
    println!("Loaded configuration:");
    println!("{}", config);

    Ok(())
}
