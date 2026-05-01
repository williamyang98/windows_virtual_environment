fn main() -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    println!("Working directory: {}", cwd.display());
    println!("Environment variables");
    for (key, value) in std::env::vars() {
        println!("{key} = {value}")
    }
    Ok(())
}
