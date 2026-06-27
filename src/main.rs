mod parser;
mod exec;

use std::io::{self, Write};
use parser::{tokenize, parse};
use exec::{execute_chained_pipeline, run_builtin_in_process};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Hidden builtin execution subcommand used for pipelines
    if args.len() >= 3 && args[1] == "--builtin" {
        let name = &args[2];
        let builtin_args = &args[3..];
        let code = run_builtin_in_process(name, builtin_args);
        std::process::exit(code);
    }

    // -c mode: runs one command line and exits with its status
    if args.len() >= 3 && args[1] == "-c" {
        let cmd_str = &args[2];
        let last_status = 0;
        let tokens = match tokenize(cmd_str, last_status) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("minish: {}", e);
                std::process::exit(127);
            }
        };

        let cp = match parse(&tokens) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("minish: {}", e);
                std::process::exit(2);
            }
        };

        let status = execute_chained_pipeline(cp);
        std::process::exit(status);
    }

    // Interactive REPL mode
    let mut last_status = 0;
    loop {
        print!("minish> ");
        if let Err(_) = io::stdout().flush() {
            break;
        }

        let mut line = String::new();
        match io::stdin().read_line(&mut line) {
            Ok(0) => {
                // EOF (Ctrl-D)
                break;
            }
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                let tokens = match tokenize(trimmed, last_status) {
                    Ok(t) => t,
                    Err(e) => {
                        eprintln!("minish: {}", e);
                        last_status = 127;
                        continue;
                    }
                };

                let cp = match parse(&tokens) {
                    Ok(p) => p,
                    Err(e) => {
                        eprintln!("minish: {}", e);
                        last_status = 2;
                        continue;
                    }
                };

                last_status = execute_chained_pipeline(cp);
            }
            Err(e) => {
                eprintln!("minish: read error: {}", e);
                break;
            }
        }
    }
}
