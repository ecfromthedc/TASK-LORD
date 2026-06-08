//! TASK LORD — local Kanban that clocks every project across your agents and
//! sessions, then lets you cook a card back into a fresh, handed-off session.

mod config;
mod deepseek;
mod handoff;
mod harvest;
mod llm;
mod model;
mod ollama;
mod serve;
mod sources;
mod store;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "tasklord", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Scan sources, summarize, and rebuild the board (db + board.json).
    Harvest {
        /// Skip the local LLM; use fast heuristics only.
        #[arg(long)]
        no_llm: bool,
    },
    /// Serve the live launchpad board (opens the browser).
    Serve,
    /// Dismiss a task by id so it never repopulates.
    Dismiss { id: String },
    /// Restore a dismissed task.
    Undismiss { id: String },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Cmd::Harvest { no_llm } => harvest::run(!no_llm).await?,
        Cmd::Serve => serve::run().await?,
        Cmd::Dismiss { id } => {
            store::dismiss(&id, "cli")?;
            println!("dismissed {id}");
        }
        Cmd::Undismiss { id } => {
            store::undismiss(&id)?;
            println!("restored {id}");
        }
    }
    Ok(())
}
