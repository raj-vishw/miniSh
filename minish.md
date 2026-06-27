> **Build a Unix shell** that runs real programs, wires multi-stage pipelines, and redirects I/O. By the end you do real work inside a shell you wrote.

<span style="background:#ff0000;color:#fff;padding:2px 8px;border-radius:6px;font-size:12px">Hard</span>

| | |
|---|---|
| **Stack** | `std` only (`std::process`, `std::env`, `std::fs`), zero external crates |
| **Deliverable** | GitHub repo + README (must answer: why can `cd` not be an external program?) |
| **Graded via** | `minish -c "..."` diffed byte-for-byte against `bash -c "..."` |
| **Due** | end of course |

### What you'll build

A REPL that prints a prompt, tokenizes a command line (with quoting), and executes it: external programs with arguments, multi-stage pipelines running concurrently, `<` and `>` redirection, and three builtins. This is the process layer of the OS: how programs are found, started, connected, and reaped.

```
minish> ls -l | grep ".rs" | wc -l
14
minish> sort < names.txt > sorted.txt
minish> cd /tmp
```

**Hard rules:** no `unsafe`. No shelling out to `sh -c` (automatic fail). Out of scope permanently: signals, job control, `&&`/`||`/`;`, `$VAR` expansion, globbing. Ctrl-C killing your shell is accepted.

### Core requirements

- [ ] Tokenizer: whitespace splitting, `"..."` and `'...'` grouping (identical behavior), operators `|` `<` `>`, operators inside quotes are literal
- [ ] Parser: `pipeline := command ( "|" command )*`, `command := word+ redirection*` - produces a plain struct, spawns nothing
- [ ] Execution via `Command::new` (std does PATH lookup); spawn failure prints `minish: command not found: <name>`, status 127, shell keeps running
- [ ] Pipelines: all stages spawned **concurrently** (`Stdio::piped()`, hand `child.stdout.take()` to the next stage), then wait on all; pipeline status = last command's status
- [ ] Redirection: `> file` truncates stdout, `< file` feeds stdin; unopenable file prints the OS error, command does not run
- [ ] Builtins in-process: `cd <dir>` (no arg: `$HOME`), `pwd`, `exit`; builtin names beat PATH
- [ ] No panics on any input; no zombie children ever
- [ ] `minish -c "cmd"` runs one line and exits with its status

### Starter

```rust
// parser.rs output. exec.rs is the ONLY module that
// spawns processes; lexer and parser are pure functions
// you can unit-test without executing anything.
pub struct Pipeline {
    pub commands: Vec<Cmd>,
}

pub struct Cmd {
    pub argv: Vec<String>,       // argv[0] is the program
    pub stdin_from: Option<String>,  // < file
    pub stdout_to: Option<String>,   // > file
}
```

### Acceptance tests

Tests 1-4: `minish -c "..."` stdout byte-identical to `bash -c "..."`.

- [ ] `echo one two   three` (whitespace collapsing)
- [ ] `echo "a  b" 'c  d'` (quoting)
- [ ] `printf 'b\na\nc\n' | sort | head -2` (3-stage pipeline)
- [ ] `echo data > /tmp/mt.txt` then `cat < /tmp/mt.txt` (both redirections)
- [ ] `cd /tmp` then `pwd` as successive interactive lines prints `/tmp` (builtin mutates shell state)
- [ ] `yes | head -3` terminates with three `y` lines (the concurrency test: sequential implementations hang forever)
- [ ] `minish -c "nosuchcmd"` exits 127; interactively the shell survives and reprompts
- [ ] Unterminated quote and `ls | | wc` each print an error and a fresh prompt, never a panic
- [ ] After the pipeline test, `ps` shows no `<defunct>` children

### Stretch goals

- [ ] `>>` append redirection
- [ ] Exit status tracking + `&&` / `||` chaining
- [ ] `$VAR` and `$?` expansion (double quotes expand, single quotes do not)
- [ ] Hand-rolled PATH resolution: split `$PATH`, find the executable yourself, 127 on miss

> 💡 **Tip:** write the lexer and parser as pure functions with unit tests before touching `std::process`. Most of your bugs will be quoting bugs, and you want to find them without spawning anything.

### Resources

**Concepts**
- [OSTEP ch. 5: Process API](https://pages.cs.wisc.edu/~remzi/OSTEP/cpu-api.pdf) - fork, exec, wait, zombies, and why the shell is the motivating example for all of it; read this first
- [Tutorial: Write a Shell in C](https://brennan.io/2015/01/16/write-a-shell-in-c/) - the classic walkthrough of the same project one level down, with raw fork/exec instead of `Command::new`; read it to see what std is hiding from you (do not port it - your tokenizer, pipes, and redirection go further than it does)
- [The Missing Semester: the shell](https://missing.csail.mit.edu/2020/course-shell/) - power-user fluency in the thing you are about to rebuild

**The real thing**
- [`std::process::Command`](https://doc.rust-lang.org/std/process/struct.Command.html) and [`Stdio`](https://doc.rust-lang.org/std/process/struct.Stdio.html) - your entire toolbox
- `strace -f ./minish -c "ls | wc"` - watch your safe Rust shell make the raw fork/exec/pipe syscalls underneath
- `man bash`, section SHELL GRAMMAR - what the full-fat version of your Part 2 grammar looks like
