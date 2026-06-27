use std::fs::{File, OpenOptions};
use std::io;
use std::process::{Command, Stdio, Child};
use crate::parser::{Cmd, Pipeline, ChainOp, ChainedPipeline};

pub fn run_builtin_in_process(name: &str, args: &[String]) -> i32 {
    match name {
        "cd" => {
            if args.len() > 1 {
                eprintln!("minish: cd: too many arguments");
                return 1;
            }
            let target = if args.is_empty() {
                match std::env::var("HOME") {
                    Ok(home) => home,
                    Err(_) => {
                        eprintln!("minish: cd: HOME not set");
                        return 1;
                    }
                }
            } else {
                args[0].clone()
            };
            if let Err(e) = std::env::set_current_dir(&target) {
                eprintln!("minish: cd: {}: {}", target, e);
                1
            } else {
                0
            }
        }
        "pwd" => {
            match std::env::current_dir() {
                Ok(path) => {
                    println!("{}", path.display());
                    0
                }
                Err(e) => {
                    eprintln!("minish: pwd: {}", e);
                    1
                }
            }
        }
        "exit" => {
            if args.len() > 1 {
                eprintln!("minish: exit: too many arguments");
                1
            } else {
                let status = if args.is_empty() {
                    0
                } else {
                    match args[0].parse::<i32>() {
                        Ok(s) => s,
                        Err(_) => {
                            eprintln!("minish: exit: {}: numeric argument required", args[0]);
                            2
                        }
                    }
                };
                std::process::exit(status);
            }
        }
        _ => unreachable!(),
    }
}

pub fn execute_standalone_builtin(cmd: &Cmd) -> i32 {
    // Validate stdin redirection
    if let Some(ref stdin_file) = cmd.stdin_from {
        if let Err(e) = File::open(stdin_file) {
            eprintln!("minish: {}: {}", stdin_file, e);
            return 1;
        }
    }

    // Handle stdout redirection
    let mut redirect_file: Option<File> = None;
    if let Some(ref stdout_file) = cmd.stdout_to {
        match OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(stdout_file)
        {
            Ok(file) => {
                redirect_file = Some(file);
            }
            Err(e) => {
                eprintln!("minish: {}: {}", stdout_file, e);
                return 1;
            }
        }
    } else if let Some(ref append_file) = cmd.stdout_append {
        match OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .open(append_file)
        {
            Ok(file) => {
                redirect_file = Some(file);
            }
            Err(e) => {
                eprintln!("minish: {}: {}", append_file, e);
                return 1;
            }
        }
    }

    let name = &cmd.argv[0];
    let args = &cmd.argv[1..];

    if name == "pwd" && redirect_file.is_some() {
        let mut file = redirect_file.unwrap();
        match std::env::current_dir() {
            Ok(path) => {
                use std::io::Write;
                if let Err(e) = writeln!(file, "{}", path.display()) {
                    eprintln!("minish: pwd: {}", e);
                    1
                } else {
                    0
                }
            }
            Err(e) => {
                eprintln!("minish: pwd: {}", e);
                1
            }
        }
    } else {
        run_builtin_in_process(name, args)
    }
}

pub fn execute_pipeline(pipeline: Pipeline) -> i32 {
    let commands = pipeline.commands;
    let n = commands.len();
    if n == 0 {
        return 0;
    }

    // Standalone builtin runs in-process to mutate shell state
    if n == 1 {
        let cmd = &commands[0];
        let name = &cmd.argv[0];
        if name == "cd" || name == "pwd" || name == "exit" {
            return execute_standalone_builtin(cmd);
        }
    }

    // Otherwise, spawn concurrently
    let mut children: Vec<(usize, Child)> = Vec::new();
    let mut prev_stdout: Option<std::process::ChildStdout> = None;
    let mut failed_spawn_idx: Option<(usize, i32)> = None;

    let current_exe = std::env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("minish"));

    for (i, cmd) in commands.iter().enumerate() {
        // Handle stdin redirection
        let stdin_opt = if let Some(ref stdin_file) = cmd.stdin_from {
            match File::open(stdin_file) {
                Ok(file) => Stdio::from(file),
                Err(e) => {
                    eprintln!("minish: {}: {}", stdin_file, e);
                    failed_spawn_idx = Some((i, 1));
                    break;
                }
            }
        } else if let Some(stdout) = prev_stdout.take() {
            Stdio::from(stdout)
        } else {
            Stdio::inherit()
        };

        // Handle stdout redirection
        let stdout_opt = if let Some(ref stdout_file) = cmd.stdout_to {
            match OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(stdout_file)
            {
                Ok(file) => Stdio::from(file),
                Err(e) => {
                    eprintln!("minish: {}: {}", stdout_file, e);
                    failed_spawn_idx = Some((i, 1));
                    break;
                }
            }
        } else if let Some(ref append_file) = cmd.stdout_append {
            match OpenOptions::new()
                .write(true)
                .create(true)
                .append(true)
                .open(append_file)
            {
                Ok(file) => Stdio::from(file),
                Err(e) => {
                    eprintln!("minish: {}: {}", append_file, e);
                    failed_spawn_idx = Some((i, 1));
                    break;
                }
            }
        } else if i < n - 1 {
            Stdio::piped()
        } else {
            Stdio::inherit()
        };

        // Determine if builtin or external
        let name = &cmd.argv[0];
        let mut child_cmd = if name == "cd" || name == "pwd" || name == "exit" {
            let mut c = Command::new(&current_exe);
            c.arg("--builtin");
            c.arg(name);
            c.args(&cmd.argv[1..]);
            c
        } else {
            let mut c = Command::new(name);
            c.args(&cmd.argv[1..]);
            c
        };

        child_cmd.stdin(stdin_opt).stdout(stdout_opt);

        match child_cmd.spawn() {
            Ok(mut child) => {
                if i < n - 1 {
                    prev_stdout = child.stdout.take();
                }
                children.push((i, child));
            }
            Err(e) => {
                if e.kind() == io::ErrorKind::NotFound {
                    eprintln!("minish: command not found: {}", name);
                } else {
                    eprintln!("minish: {}: {}", name, e);
                }
                failed_spawn_idx = Some((i, 127));
                break;
            }
        }
    }

    // Wait on all spawned children to prevent zombies
    let mut last_status = 0;
    for (idx, mut child) in children {
        match child.wait() {
            Ok(status) => {
                if idx == n - 1 {
                    last_status = status.code().unwrap_or(0);
                }
            }
            Err(_) => {}
        }
    }

    // If one of the stages failed to spawn (redirection or command not found)
    if let Some((_idx, status)) = failed_spawn_idx {
        last_status = status;
    }

    last_status
}

pub fn execute_chained_pipeline(cp: ChainedPipeline) -> i32 {
    let mut last_status = 0;
    let mut skip_next = false;

    for (pipeline, op) in cp.pipelines {
        if skip_next {
            // Determine next skip_next value based on the skipped pipeline's operator
            if op == ChainOp::And && last_status != 0 {
                skip_next = true;
            } else if op == ChainOp::Or && last_status == 0 {
                skip_next = true;
            } else {
                skip_next = false;
            }
            continue;
        }

        last_status = execute_pipeline(pipeline);

        if op == ChainOp::And && last_status != 0 {
            skip_next = true;
        } else if op == ChainOp::Or && last_status == 0 {
            skip_next = true;
        } else {
            skip_next = false;
        }
    }

    last_status
}
