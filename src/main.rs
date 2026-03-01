use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
#[command(
    name = "sarang",
    about = "The Sarang AI-native policy language compiler",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Parse and validate a Sarang source file, reporting any diagnostics
    Check {
        /// Path to the .sarang source file
        file: PathBuf,
    },
    /// Compile a Sarang source file to Policy IR JSON
    Compile {
        /// Path to the .sarang source file
        file: PathBuf,
        /// Output path for the JSON file (defaults to stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Pretty-print the AST of a Sarang source file
    Inspect {
        /// Path to the .sarang source file
        file: PathBuf,
    },
    /// Print the Sarang version
    Version,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Version => {
            println!("sarang {}", env!("CARGO_PKG_VERSION"));
        }
        Command::Check { file } => {
            let source = match read_source(&file) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("error: {e}");
                    process::exit(1);
                }
            };
            // Future phases will plug in: lex → parse → validate
            let _ = source;
            println!("sarang check: not yet implemented (file read OK, {} bytes)", source.len());
        }
        Command::Compile { file, output } => {
            let source = match read_source(&file) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("error: {e}");
                    process::exit(1);
                }
            };
            let _ = (source, output);
            println!("sarang compile: not yet implemented");
        }
        Command::Inspect { file } => {
            let source = match read_source(&file) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("error: {e}");
                    process::exit(1);
                }
            };
            let _ = source;
            println!("sarang inspect: not yet implemented");
        }
    }
}

fn read_source(path: &PathBuf) -> Result<String, String> {
    if !path.exists() {
        return Err(format!("file not found: {}", path.display()));
    }
    match path.extension().and_then(|e| e.to_str()) {
        Some("sarang") => {}
        Some(ext) => {
            return Err(format!(
                "expected a .sarang file, got .{ext}: {}",
                path.display()
            ));
        }
        None => {
            return Err(format!(
                "expected a .sarang file, no extension: {}",
                path.display()
            ));
        }
    }
    std::fs::read_to_string(path).map_err(|e| format!("could not read {}: {e}", path.display()))
}
