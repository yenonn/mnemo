use clap::Parser;
use mnemo::repl::Repl;
use tracing::info;

#[derive(Parser, Debug)]
#[command(name = "mnemo")]
#[command(about = "Agent memory database")]
struct Cli {
    #[arg(long, default_value = "default")]
    agent_id: String,

    #[arg(long)]
    repl: bool,

    #[arg(long, short)]
    version: bool,
}

fn main() {
    let cli = Cli::parse();

    if cli.version {
        println!("mnemo 0.1.0");
        return;
    }

    println!("Agent: {}", cli.agent_id);
    if cli.repl {
        println!("REPL mode not yet implemented");
    }
}
