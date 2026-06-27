# MiniSH — A Unix Shell in Rust

MiniSH is a custom Unix shell written from scratch in Rust, utilizing only the standard library. It supports pipeline construction, I/O redirection, logical chaining (`&&`/`||`), environment variable expansion, and built-in commands.

---

## The Big Question: Why Can `cd` Not Be an External Program?

In Unix-like operating systems, process environments are strictly isolated. Each process maintains its own independent metadata, including its **Current Working Directory (CWD)**.

If `cd` were implemented as an external program (e.g., an executable at `/bin/cd`):
1. When the user types `cd /tmp`, the parent shell process would spawn a child process (using `fork` and `exec`) to run the `cd` executable.
2. The child process would execute the `chdir` system call, changing its own CWD to `/tmp`.
3. The child process would complete its execution and exit.
4. Because the child process has a completely isolated environment, its change of directory **does not affect the parent shell process's environment**. The shell's CWD remains unchanged.

For directory changes to affect subsequent commands in the shell session, the `chdir` system call (in Rust, `std::env::set_current_dir`) **must run inside the main shell process itself**. Thus, `cd` (along with `exit` and `pwd`) must be implemented as a shell **builtin** executed in-process.

---

## Key Features & Design

### 1. Tokenizer & Parser
* Pure functions implemented in `src/parser.rs`.
* Supports whitespace splitting with quoting rules (both `'...'` and `"..."` treat operators as literal text).
* Implements operator precedence rules for pipelines (`|`), input/output redirection (`<`, `>`, and `>>`), and logical chaining (`&&`, `||`).
* Safely catches syntax errors (e.g., `ls | | wc`, unterminated quotes) and recovers without panicking.

### 2. Execution Engine (`src/exec.rs`)
* Spawns all pipeline stages **concurrently** using `Stdio::piped()`, passing the stdout of each child process directly to the stdin of the next.
* Monitors child process execution and calls `child.wait()` on all successfully spawned stages to guarantee that **no zombie (defunct) processes** are left in the system.
* Pipeline exit status is determined by the last stage in the pipeline.

### 3. Redirections & Builtins
* Supports input redirection (`<`), output redirection (`>`), and append redirection (`>>`).
* If any redirection target file cannot be opened (e.g., permission denied), the shell prints the OS error, aborts the command execution early, and sets the stage exit code to `1`.
* In-process builtins: `cd` (defaulting to `$HOME` if no arguments), `pwd`, and `exit` (supporting custom exit status codes).
* Builtins beat executables in the system `$PATH`.

### 4. Stretch Goals Implemented
* **Append Redirection**: `>>` works for appending stdout to a file.
* **Logical Chaining**: Support for `&&` and `||` operators using left-to-right associative short-circuiting.
* **Variable Expansion**: `$VAR` and `$?` (exit status of the last command) expand in unquoted and double-quoted strings, but are treated literally inside single quotes.

---

## Usage

### Run the REPL
Start the interactive shell:
```bash
cargo run
```

### Single Command Execution (`-c`)
Execute a command string and exit with its status code:
```bash
cargo run -- -c "echo Hello World && pwd"
```

---

## Test Verification

A full verification script `test_all.sh` is included. It compiles the shell, runs 10 distinct acceptance tests (validating piping, concurrency, logical operators, redirection, and variable expansion), and diffs the output byte-for-byte against standard `bash`.

Run the test suite with:
```bash
./test_all.sh
```
