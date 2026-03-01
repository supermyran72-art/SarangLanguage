use clap::{Parser, Subcommand};
use sarang::common::span::SourceFile;
use sarang::emit;
use sarang::ir;
use sarang::parser;
use sarang::validator;
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
            let source = read_source_or_exit(&file);
            let filename = file.display().to_string();
            let sf = SourceFile::new(&filename, &source);

            let program = match parser::parse(&source) {
                Ok(prog) => prog,
                Err(diags) => {
                    eprint!("{}", diags.render_to_string(&sf));
                    process::exit(1);
                }
            };

            let diags = validator::validate(&program);
            if diags.warning_count() > 0 || diags.has_errors() {
                eprint!("{}", diags.render_to_string(&sf));
            }
            if diags.has_errors() {
                process::exit(1);
            }

            println!("ok: {filename}");
        }
        Command::Compile { file, output } => {
            let source = read_source_or_exit(&file);
            let filename = file.display().to_string();
            let sf = SourceFile::new(&filename, &source);

            let program = match parser::parse(&source) {
                Ok(prog) => prog,
                Err(diags) => {
                    eprint!("{}", diags.render_to_string(&sf));
                    process::exit(1);
                }
            };

            let diags = validator::validate(&program);
            if diags.warning_count() > 0 || diags.has_errors() {
                eprint!("{}", diags.render_to_string(&sf));
            }
            if diags.has_errors() {
                process::exit(1);
            }

            let policy = match ir::lower(&program) {
                Ok(ir) => ir,
                Err(e) => {
                    eprintln!("internal error: {e}");
                    process::exit(2);
                }
            };

            match output {
                Some(path) => {
                    let file = match std::fs::File::create(&path) {
                        Ok(f) => f,
                        Err(e) => {
                            eprintln!("error: could not create {}: {e}", path.display());
                            process::exit(1);
                        }
                    };
                    if let Err(e) = emit::emit_json_pretty_to_writer(&policy, file) {
                        eprintln!("error: failed to write JSON: {e}");
                        process::exit(1);
                    }
                }
                None => {
                    let json = emit::emit_json_pretty(&policy).unwrap_or_else(|e| {
                        eprintln!("error: failed to serialize JSON: {e}");
                        process::exit(1);
                    });
                    println!("{json}");
                }
            }
        }
        Command::Inspect { file } => {
            let source = read_source_or_exit(&file);
            let filename = file.display().to_string();
            let sf = SourceFile::new(&filename, &source);

            let program = match parser::parse(&source) {
                Ok(prog) => prog,
                Err(diags) => {
                    eprint!("{}", diags.render_to_string(&sf));
                    process::exit(1);
                }
            };

            println!("{:#?}", program);
        }
    }
}

fn read_source_or_exit(path: &PathBuf) -> String {
    match read_source(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(1);
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
