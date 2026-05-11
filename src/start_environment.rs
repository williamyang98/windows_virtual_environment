use anyhow::Context;
use clap::Parser;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use strfmt::strfmt;

#[derive(Clone, Debug, Parser)]
#[command(
    version, author,
    about = "Start executable with custom environment variables to create a portable virtual environment",
    long_about = None,
)]
struct Args {
    // Executable working directory
    #[arg(short = 'd', long, value_parser = validate_is_directory, help = "Default is directory containing executable")]
    executable_working_directory: Option<PathBuf>,
    // Environment path
    #[arg(short = 'p', long, default_value = "./env", value_parser = validate_is_directory_empty_or_exists)]
    environment_path: PathBuf,
    // Environment config
    #[arg(short = 'c', long, default_value = "./environment_config.json", value_parser = validate_is_file)]
    environment_config: PathBuf,
    // Username
    #[arg(short = 'u', long, default_value = "default")]
    username: String,
    // Executable path
    #[arg(value_parser = validate_is_file)]
    executable_path: PathBuf,
    // Executable arguments
    executable_arguments: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Config {
    directories: HashMap<String, String>,
    seed_directories: Vec<String>,
    override_variables: HashMap<String, String>,
    pass_through_variables: Vec<String>,
}

fn validate_is_file(s: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(s);
    if path.exists() && path.is_file() {
        Ok(path)
    } else {
        Err("Filepath does not exist".into())
    }
}

fn validate_is_directory_empty_or_exists(s: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(s);
    if path.exists() && !path.is_dir() {
        Err("Cannot write to existing path that is not a directory".into())
    } else {
        Ok(path)
    }
}

fn validate_is_directory(s: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(s);
    if path.exists() && path.is_dir() {
        Ok(path)
    } else {
        Err("Directory does not exist".into())
    }
}

fn create_environment_directory(path: &Path) {
    if path.exists() {
        return;
    }
    match std::fs::create_dir_all(path) {
        Ok(()) => log::info!("Created environment directory: {}", &path.display()),
        Err(err) => log::error!("Failed to create environment directory: {}: {}", &path.display(), err),
    }
}

fn main() -> anyhow::Result<()> {
    simple_logger::SimpleLogger::new()
        .with_level(log::LevelFilter::Info)
        .env()
        .with_colors(true)
        .without_timestamps()
        .init()?;

    let args = Args::parse();
    let config_file = std::fs::read_to_string(&args.environment_config)
        .with_context(|| format!("Failed to read environment configuration file: {}", &args.environment_config.display()))?;
    let config: Config = serde_json::from_str(&config_file)
        .with_context(|| format!("Failed to parse configuration file: {}", &args.environment_config.display()))?;

    create_environment_directory(&args.environment_path);

    log::info!("Starting with username: {}", &args.username);
    let parent_environment: HashMap<String, String> = std::env::vars().collect();

    // create environment variables
    let mut environment = HashMap::<String, String>::new();
    let mut update_variable = |key: &str, new_value: &str| {
        if let Some(old_value) = environment.insert(key.to_string(), new_value.to_string()) {
            log::warn!("Overrode environment variable key={key}, old_value={old_value}, new_value={new_value}");
        };
    };
    let formatters = HashMap::<String, String>::from([
        ("root".to_string(), args.environment_path.to_string_lossy().into_owned()),
        ("username".to_string(), args.username),
    ]);

    for key in &config.pass_through_variables {
        match parent_environment.get(key) {
            None => log::warn!("Missing environment variable: {}", key),
            Some(value) => update_variable(key, value),
        }
    }
    for (key, fmt) in &config.override_variables {
        let value = strfmt(fmt, &formatters)
            .with_context(|| format!("Failed to format override variable: key={key}, value={fmt}"))?;
        update_variable(key, &value);
    }
    for fmt in &config.seed_directories {
        let path_string = strfmt(fmt, &formatters)
            .with_context(|| format!("Failed to format seed directory: {fmt}"))?;
        let rel_path = PathBuf::from(path_string);
        match std::path::absolute(rel_path.as_path()) {
            Ok(abs_path) => create_environment_directory(&abs_path),
            Err(err) => log::error!("Failed to create seed directory: {} because {}", rel_path.display(), err),
        }
    }
    for (key, fmt) in &config.directories {
        let path_string = strfmt(fmt, &formatters)
            .with_context(|| format!("Failed to format directory: key={key}, value={fmt}"))?;
        let rel_path = PathBuf::from(path_string);
        match std::path::absolute(rel_path.as_path()) {
            Ok(abs_path) => {
                create_environment_directory(&abs_path);
                update_variable(key, &abs_path.to_string_lossy());
            },
            Err(err) => log::error!("Failed to create directory: {} because {}", rel_path.display(), err),
        }
    }

    // determine current working directory
    let mut cwd = args.executable_working_directory.clone();
    if cwd.is_none() {
        if let Some(executable_parent) = args.executable_path.parent() {
            let new_cwd = executable_parent.to_path_buf();
            cwd = Some(new_cwd);
        } else {
            log::warn!("Unable to determine current working directory of executable: {}", args.executable_path.display());
        }
    }
    if cwd.is_none() {
        match std::env::current_dir() {
            Ok(new_cwd) => {
                cwd = Some(new_cwd);
            },
            Err(err) => log::error!("Unable to determine working directory of launcher's location for executable: {}", err),
        }
    }

    // start process
    let mut command = std::process::Command::new(&args.executable_path);
    command.args(args.executable_arguments.as_slice());
    if let Some(cwd) = cwd {
        log::info!("Setting executable working directory to: {}", cwd.display());
        command.current_dir(cwd);
    }
    command.env_clear();
    command.stdin(std::process::Stdio::null());
    command.stdout(std::process::Stdio::inherit());
    command.stderr(std::process::Stdio::inherit());
    command.envs(&environment);

    let mut child = command.spawn()
        .with_context(|| format!("Failed to launch executable: {}", &args.executable_path.display()))?;
    let exit_code = child.wait()
        .with_context(|| "Failed to wait for process to finish")?;
    log::info!("Process finished with exit code: {}", exit_code);

    Ok(())
}
