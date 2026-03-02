use clap::{Parser, Subcommand};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "gunita", about = "Household memory bank and player")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Path to config file
    #[arg(short, long, global = true)]
    pub config: Option<PathBuf>,

    /// Path to data directory
    #[arg(long, global = true)]
    pub data_dir: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Run the HTTP server
    Serve {
        /// Host to bind to
        #[arg(long)]
        host: Option<String>,

        /// Port to bind to
        #[arg(short, long)]
        port: Option<u16>,
    },
}

#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
pub struct Config {
    pub server: ServerConfig,
    pub salita: SalitaConfig,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(default)]
pub struct SalitaConfig {
    pub url: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 7777,
        }
    }
}

impl Default for SalitaConfig {
    fn default() -> Self {
        Self {
            url: "http://localhost:6969".to_string(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            salita: SalitaConfig::default(),
        }
    }
}

impl Config {
    pub fn load(cli: &Cli) -> anyhow::Result<Self> {
        let data_dir = Self::data_dir(cli);
        let config_path = cli
            .config
            .clone()
            .unwrap_or_else(|| data_dir.join("config.toml"));

        let mut config: Config = if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            toml::from_str(&content)?
        } else {
            Config::default()
        };

        // CLI overrides for serve command
        let Command::Serve { ref host, ref port } = cli.command;
        if let Some(ref h) = host {
            config.server.host = h.clone();
        }
        if let Some(p) = port {
            config.server.port = *p;
        }

        // Environment variable override for salita URL
        if let Ok(url) = std::env::var("SALITA_URL") {
            config.salita.url = url;
        }

        Ok(config)
    }

    pub fn data_dir(cli: &Cli) -> PathBuf {
        cli.data_dir.clone().unwrap_or_else(|| {
            dirs::home_dir()
                .expect("Could not determine home directory")
                .join(".gunita")
        })
    }

    pub fn db_path(cli: &Cli) -> PathBuf {
        Self::data_dir(cli).join("gunita.db")
    }
}
