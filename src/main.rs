use clap::{Parser, Subcommand};
use mnemo::protocol::Command;
use mnemo::repl::Repl;

#[derive(Parser, Debug)]
#[command(name = "mnemo")]
#[command(about = "Agent memory database")]
struct Cli {
    #[arg(long, default_value = "default")]
    agent_id: String,

    #[arg(long)]
    repl: bool,

    #[arg(long)]
    mcp: bool,

    #[arg(long, short)]
    version: bool,

    #[command(subcommand)]
    command: Option<MnemoCommand>,
}

#[derive(Subcommand, Debug)]
enum MnemoCommand {
    /// Remember a new memory
    Remember {
        #[arg(value_name = "TEXT")]
        text: String,
        #[arg(long, default_value = "semantic")]
        memory_type: String,
        #[arg(long)]
        importance: Option<f64>,
    },
    /// Recall memories matching a query
    Recall {
        #[arg(value_name = "QUERY")]
        query: String,
        #[arg(long)]
        memory_type: Option<String>,
        #[arg(long, default_value = "10")]
        limit: usize,
    },
    /// Show memory status
    Status,
    /// Initialize database
    Init,
    /// Consolidate memories
    Consolidate {
        #[arg(value_name = "FROM")]
        from: String,
        #[arg(value_name = "TO")]
        to: String,
    },
    /// Extract implicit memories from natural language text
    Extract {
        #[arg(value_name = "TEXT")]
        text: String,
    },
    /// Forget a memory by id
    Forget {
        #[arg(value_name = "ID")]
        id: String,
    },
    /// Process natural language with implicit intent
    Bind {
        #[arg(value_name = "TEXT")]
        text: String,
    },
    /// Set or show configuration
    Pragma {
        #[arg(value_name = "KEY")]
        key: Option<String>,
        #[arg(value_name = "VALUE")]
        value: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    if cli.version {
        println!("mnemo 0.1.0");
        return;
    }

    if cli.mcp {
        mnemo::mcp::serve_stdio(&cli.agent_id);
        return;
    }

    if cli.repl {
        let mut repl = Repl::new(&cli.agent_id).expect("Failed to initialize REPL");
        repl.run();
        return;
    }

    if let Some(cmd) = cli.command {
        let mut repl = Repl::new(&cli.agent_id).expect("Failed to initialize");

        let response = match cmd {
            MnemoCommand::Remember { text, memory_type, importance } => {
                repl.execute(Command::Remember {
                    content: text,
                    memory_type,
                    metadata: importance.map(|i| vec![("importance".to_string(), i.to_string())]).unwrap_or_default(),
                })
            }
            MnemoCommand::Recall { query, memory_type, limit } => {
                repl.execute(Command::Recall {
                    query,
                    memory_types: memory_type.map(|t| vec![t]).unwrap_or_default(),
                    conditions: vec![],
                    limit,
                })
            }
            MnemoCommand::Status => repl.execute(Command::Status),
            MnemoCommand::Init => repl.execute(Command::Init),
            MnemoCommand::Consolidate { from, to } => {
                repl.execute(Command::Consolidate { from, to, conditions: vec![] })
            }
            MnemoCommand::Extract { text } => {
                repl.execute(Command::Extract { text })
            }
            MnemoCommand::Bind { text } => {
                repl.execute(Command::Bind { text })
            }
            MnemoCommand::Forget { id } => {
                repl.execute(Command::Forget { id: Some(id), conditions: vec![] })
            }
            MnemoCommand::Pragma { key, value } => {
                repl.execute(Command::Pragma { key, value })
            }
        };
        println!("{}", response);
    } else {
        println!("Use --repl for interactive mode or a subcommand for one-shot usage.");
        println!("Run 'mnemo --help' for more information.");
    }
}
