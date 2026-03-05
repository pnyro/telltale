mod app;
mod daemon;
mod notify;
mod output;
mod sources;

use std::error::Error;
use std::time::SystemTime;

use clap::{Parser, Subcommand, ValueEnum};
use telltale_core::{Severity, Store};

#[derive(Parser)]
#[command(name = "telltale")]
#[command(about = "Proactive system event monitor")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Daemon,
    Simulate {
        #[arg(long, default_value_t = 5)]
        interval: u64,
        #[arg(long, default_value_t = 0)]
        count: u64,
    },
    Status,
    Recent {
        #[arg(long, default_value_t = 20)]
        limit: usize,
        #[arg(long, value_enum)]
        severity: Option<SeverityArg>,
    },
    Rules {
        #[command(subcommand)]
        command: Option<RulesCommand>,
    },
}

#[derive(Subcommand)]
enum RulesCommand {
    List,
    Show { id: String },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum SeverityArg {
    Critical,
    Warning,
    Info,
}

fn run() -> Result<(), Box<dyn Error + Send + Sync>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Daemon => daemon::run(),
        Commands::Simulate { interval, count } => daemon::run_simulated(interval, count),
        Commands::Status => status_command(),
        Commands::Recent { limit, severity } => recent_command(limit, severity),
        Commands::Rules { command } => rules_command(command),
    }
}

fn status_command() -> Result<(), Box<dyn Error + Send + Sync>> {
    let db_path = app::database_path()?;
    let db_exists = db_path.exists();
    let rule_count = app::rules_for_current_os().len();

    let (checkpoint, total_alerts) = if db_exists {
        let store = Store::open(&db_path)?;
        (
            store.get_state("last_event_timestamp")?,
            store.count_alerts().unwrap_or(0),
        )
    } else {
        (None, 0)
    };

    println!("database_exists: {}", db_exists);
    println!("database_path: {}", db_path.display());
    println!("rules_loaded: {}", rule_count);
    println!(
        "last_checkpoint: {}",
        checkpoint.unwrap_or_else(|| "none".to_string())
    );
    println!("total_alerts: {}", total_alerts);

    Ok(())
}

fn recent_command(
    limit: usize,
    severity: Option<SeverityArg>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let db_path = app::database_path()?;
    if !db_path.exists() {
        println!("No database found at {}", db_path.display());
        return Ok(());
    }

    let severity_filter = severity.map(|item| match item {
        SeverityArg::Critical => Severity::Critical,
        SeverityArg::Warning => Severity::Warning,
        SeverityArg::Info => Severity::Info,
    });

    let store = Store::open(&db_path)?;
    let alerts = store.get_recent(limit, severity_filter)?;

    if alerts.is_empty() {
        println!("No alerts found.");
        return Ok(());
    }

    for alert in alerts {
        println!(
            "[{}] {} {} ({})",
            severity_label(alert.severity),
            format_timestamp(alert.last_seen),
            alert.title,
            alert.rule_id
        );
        println!("  fingerprint: {}", alert.fingerprint);
        println!("  count: {}", alert.occurrence_count);
        println!("  action: {}", alert.recommended_action);
        println!();
    }

    Ok(())
}

fn rules_command(command: Option<RulesCommand>) -> Result<(), Box<dyn Error + Send + Sync>> {
    let rules = app::rules_for_current_os();

    match command.unwrap_or(RulesCommand::List) {
        RulesCommand::List => {
            for rule in rules {
                println!(
                    "{} [{}] {}",
                    rule.id,
                    severity_label(rule.severity),
                    rule.title
                );
            }
        }
        RulesCommand::Show { id } => {
            let rule = rules
                .into_iter()
                .find(|item| item.id == id)
                .ok_or_else(|| format!("rule not found: {id}"))?;

            println!("id: {}", rule.id);
            println!("severity: {}", severity_label(rule.severity));
            println!("title: {}", rule.title);
            println!("description: {}", rule.description);
            println!("recommended_action: {}", rule.recommended_action);
            println!("cooldown_secs: {}", rule.cooldown_secs);
        }
    }

    Ok(())
}

fn severity_label(severity: Severity) -> &'static str {
    match severity {
        Severity::Critical => "critical",
        Severity::Warning => "warning",
        Severity::Info => "info",
    }
}

fn format_timestamp(ts: SystemTime) -> String {
    match ts.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(duration) => duration.as_secs().to_string(),
        Err(_) => "0".to_string(),
    }
}

fn main() {
    if let Err(err) = run() {
        eprintln!("fatal: {err}");
        std::process::exit(1);
    }
}
