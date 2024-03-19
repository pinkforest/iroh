use std::path::{Path, PathBuf};

use anyhow::{ensure, Context, Result};
use clap::Parser;
use iroh::client::quic::Iroh as IrohRpc;

use crate::config::{ConsoleEnv, NodeConfig};

use self::blob::{BlobAddOptions, BlobSource};
use self::rpc::RpcCommands;
use self::start::RunType;

pub(crate) mod author;
pub(crate) mod blob;
pub(crate) mod console;
pub(crate) mod doc;
pub(crate) mod doctor;
pub(crate) mod node;
pub(crate) mod rpc;
pub(crate) mod start;
pub(crate) mod tag;

/// iroh is a tool for syncing bytes
/// https://iroh.computer/docs
#[derive(Parser, Debug, Clone)]
#[clap(version, verbatim_doc_comment)]
pub(crate) struct Cli {
    #[clap(subcommand)]
    pub(crate) command: Commands,

    /// Path to the configuration file.
    #[clap(long)]
    pub(crate) config: Option<PathBuf>,

    /// Start an iroh node in the background.
    #[clap(long, global = true)]
    start: bool,

    /// Send log output to specified file descriptor.
    #[cfg(unix)]
    #[clap(long)]
    pub(crate) log_fd: Option<i32>,
}

#[derive(Parser, Debug, Clone)]
pub(crate) enum Commands {
    /// Start an iroh node
    ///
    /// A node is a long-running process that serves data and connects to other nodes.
    /// The console, doc, author, blob, node, and tag commands require a running node.
    ///
    /// start optionally takes a `--add SOURCE` option, which can be a file or a folder
    /// to serve on startup. Data can also be added after startup with commands like
    /// `iroh blob add` or by adding content to documents.
    Start {
        /// Optionally add a file or folder to the node.
        ///
        /// If set to `STDIN`, the data will be read from stdin.
        ///
        /// When left empty no content is added.
        #[clap(long)]
        add: Option<BlobSource>,

        /// Options when adding data.
        #[clap(flatten)]
        add_options: BlobAddOptions,
    },

    /// Open the iroh console
    ///
    /// The console is a REPL for interacting with a running iroh node.
    /// For more info on available commands, see https://iroh.computer/docs/api
    Console,

    #[clap(flatten)]
    Rpc(#[clap(subcommand)] RpcCommands),

    /// Diagnostic commands for the relay protocol.
    Doctor {
        /// Commands for doctor - defined in the mod
        #[clap(subcommand)]
        command: self::doctor::Commands,
    },
}

impl Cli {
    pub(crate) async fn run(self, data_dir: &Path) -> Result<()> {
        match self.command {
            Commands::Console => {
                let env = ConsoleEnv::for_console(data_dir)?;
                if self.start {
                    let config = NodeConfig::from_env(self.config.as_deref())?;
                    start::run_with_command(
                        &config,
                        data_dir,
                        RunType::SingleCommandNoAbort,
                        |iroh| async move { console::run(&iroh, &env).await },
                    )
                    .await
                } else {
                    let iroh = IrohRpc::connect(data_dir).await.context("rpc connect")?;
                    console::run(&iroh, &env).await
                }
            }
            Commands::Rpc(command) => {
                let env = ConsoleEnv::for_cli(data_dir)?;
                if self.start {
                    let config = NodeConfig::from_env(self.config.as_deref())?;
                    start::run_with_command(
                        &config,
                        data_dir,
                        RunType::SingleCommandAbortable,
                        |iroh| async move { command.run(&iroh, &env).await },
                    )
                    .await
                } else {
                    let iroh = IrohRpc::connect(data_dir).await.context("rpc connect")?;
                    command.run(&iroh, &env).await
                }
            }
            Commands::Start { add, add_options } => {
                // if adding data on start, exit early if the path doesn't exist
                if let Some(BlobSource::Path(ref path)) = add {
                    ensure!(
                        path.exists(),
                        "Cannot provide nonexistent path: {}",
                        path.display()
                    );
                }
                let config = NodeConfig::from_env(self.config.as_deref())?;

                let add_command = add.map(|source| blob::BlobCommands::Add {
                    source,
                    options: add_options,
                });

                start::run_with_command(
                    &config,
                    data_dir,
                    RunType::UntilStopped,
                    |client| async move {
                        match add_command {
                            None => Ok(()),
                            Some(command) => command.run(&client).await,
                        }
                    },
                )
                .await
            }
            Commands::Doctor { command } => {
                let config = NodeConfig::from_env(self.config.as_deref())?;
                self::doctor::run(command, &config).await
            }
        }
    }
}
