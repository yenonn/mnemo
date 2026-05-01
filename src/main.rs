use clap::Parser;
use mnemo::repl::Repl;

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

    if cli.repl {
        let mut repl = Repl::new(&cli.agent_id).expect("Failed to initialize REPL");
        repl.run();
    } else {
        println!("Use --repl for interactive mode or subcommands for one-shot");
    }
}
