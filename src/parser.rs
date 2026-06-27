#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Word(String),
    Pipe,           // |
    RedirectIn,     // <
    RedirectOut,    // >
    RedirectAppend, // >>
    And,            // &&
    Or,             // ||
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cmd {
    pub argv: Vec<String>,
    pub stdin_from: Option<String>,
    pub stdout_to: Option<String>,
    pub stdout_append: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pipeline {
    pub commands: Vec<Cmd>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainOp {
    None,
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainedPipeline {
    pub pipelines: Vec<(Pipeline, ChainOp)>,
}

pub fn tokenize(input: &str, last_status: i32) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
            continue;
        }

        // Check for operators
        if c == '|' {
            chars.next();
            if chars.peek() == Some(&'|') {
                chars.next();
                tokens.push(Token::Or);
            } else {
                tokens.push(Token::Pipe);
            }
            continue;
        }
        if c == '&' {
            chars.next();
            if chars.peek() == Some(&'&') {
                chars.next();
                tokens.push(Token::And);
            } else {
                return Err("unexpected token '&'".to_string());
            }
            continue;
        }
        if c == '<' {
            chars.next();
            tokens.push(Token::RedirectIn);
            continue;
        }
        if c == '>' {
            chars.next();
            if chars.peek() == Some(&'>') {
                chars.next();
                tokens.push(Token::RedirectAppend);
            } else {
                tokens.push(Token::RedirectOut);
            }
            continue;
        }

        // Otherwise, parse a word (potentially containing quotes)
        let mut word = String::new();
        let mut in_word = false;

        while let Some(&c) = chars.peek() {
            if c.is_whitespace() || c == '|' || c == '&' || c == '<' || c == '>' {
                break;
            }
            in_word = true;
            if c == '\'' {
                chars.next(); // consume opening single quote
                let mut closed = false;
                while let Some(qc) = chars.next() {
                    if qc == '\'' {
                        closed = true;
                        break;
                    }
                    word.push(qc);
                }
                if !closed {
                    return Err("unterminated quote".to_string());
                }
            } else if c == '"' {
                chars.next(); // consume opening double quote
                let mut closed = false;
                while let Some(qc) = chars.next() {
                    if qc == '"' {
                        closed = true;
                        break;
                    }
                    if qc == '$' {
                        if chars.peek() == Some(&'?') {
                            chars.next();
                            word.push_str(&last_status.to_string());
                        } else {
                            let mut var_name = String::new();
                            while let Some(&vc) = chars.peek() {
                                if vc.is_alphanumeric() || vc == '_' {
                                    var_name.push(vc);
                                    chars.next();
                                } else {
                                    break;
                                }
                            }
                            if var_name.is_empty() {
                                word.push('$');
                            } else {
                                let val = std::env::var(&var_name).unwrap_or_default();
                                word.push_str(&val);
                            }
                        }
                    } else {
                        word.push(qc);
                    }
                }
                if !closed {
                    return Err("unterminated quote".to_string());
                }
            } else if c == '$' {
                chars.next(); // consume '$'
                if chars.peek() == Some(&'?') {
                    chars.next();
                    word.push_str(&last_status.to_string());
                } else {
                    let mut var_name = String::new();
                    while let Some(&vc) = chars.peek() {
                        if vc.is_alphanumeric() || vc == '_' {
                            var_name.push(vc);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    if var_name.is_empty() {
                        word.push('$');
                    } else {
                        let val = std::env::var(&var_name).unwrap_or_default();
                        word.push_str(&val);
                    }
                }
            } else {
                word.push(c);
                chars.next();
            }
        }

        if in_word {
            tokens.push(Token::Word(word));
        }
    }

    Ok(tokens)
}

fn parse_single_pipeline(tokens: &[Token]) -> Result<Pipeline, String> {
    if tokens.is_empty() {
        return Err("empty pipeline stage".to_string());
    }

    // Split by Pipe (|)
    let mut stages = Vec::new();
    let mut current_stage = Vec::new();

    for token in tokens {
        if token == &Token::Pipe {
            if current_stage.is_empty() {
                return Err("syntax error: empty command in pipeline".to_string());
            }
            stages.push(current_stage);
            current_stage = Vec::new();
        } else {
            current_stage.push(token.clone());
        }
    }

    if current_stage.is_empty() && !stages.is_empty() {
        return Err("syntax error: empty command in pipeline".to_string());
    }
    stages.push(current_stage);

    let mut commands = Vec::new();
    for stage in stages {
        let mut argv = Vec::new();
        let mut stdin_from = None;
        let mut stdout_to = None;
        let mut stdout_append = None;

        let mut iter = stage.into_iter().peekable();
        while let Some(token) = iter.next() {
            match token {
                Token::Word(w) => {
                    argv.push(w);
                }
                Token::RedirectIn => {
                    if let Some(Token::Word(filename)) = iter.next() {
                        stdin_from = Some(filename);
                    } else {
                        return Err("syntax error: missing filename for stdin redirection (<)".to_string());
                    }
                }
                Token::RedirectOut => {
                    if let Some(Token::Word(filename)) = iter.next() {
                        stdout_to = Some(filename);
                    } else {
                        return Err("syntax error: missing filename for stdout redirection (>)".to_string());
                    }
                }
                Token::RedirectAppend => {
                    if let Some(Token::Word(filename)) = iter.next() {
                        stdout_append = Some(filename);
                    } else {
                        return Err("syntax error: missing filename for append redirection (>>)".to_string());
                    }
                }
                _ => {
                    return Err("syntax error: unexpected token inside pipeline stage".to_string());
                }
            }
        }

        if argv.is_empty() {
            return Err("syntax error: empty command".to_string());
        }

        commands.push(Cmd {
            argv,
            stdin_from,
            stdout_to,
            stdout_append,
        });
    }

    Ok(Pipeline { commands })
}

pub fn parse(tokens: &[Token]) -> Result<ChainedPipeline, String> {
    if tokens.is_empty() {
        return Ok(ChainedPipeline { pipelines: Vec::new() });
    }

    let mut pipelines = Vec::new();
    let mut current_pipeline_tokens = Vec::new();

    let mut iter = tokens.iter().peekable();
    while let Some(token) = iter.next() {
        match token {
            Token::And | Token::Or => {
                let op = if token == &Token::And { ChainOp::And } else { ChainOp::Or };
                if current_pipeline_tokens.is_empty() {
                    return Err("syntax error: unexpected token before logical operator".to_string());
                }
                let pipeline = parse_single_pipeline(&current_pipeline_tokens)?;
                pipelines.push((pipeline, op));
                current_pipeline_tokens.clear();
            }
            _ => {
                current_pipeline_tokens.push(token.clone());
            }
        }
    }

    if current_pipeline_tokens.is_empty() {
        return Err("syntax error: trailing logical operator".to_string());
    }

    let last_pipeline = parse_single_pipeline(&current_pipeline_tokens)?;
    pipelines.push((last_pipeline, ChainOp::None));

    Ok(ChainedPipeline { pipelines })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenizer_simple() {
        let tokens = tokenize("echo one two   three", 0).unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Word("echo".to_string()),
                Token::Word("one".to_string()),
                Token::Word("two".to_string()),
                Token::Word("three".to_string()),
            ]
        );
    }

    #[test]
    fn test_tokenizer_quotes() {
        let tokens = tokenize("echo \"a  b\" 'c  d'", 0).unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Word("echo".to_string()),
                Token::Word("a  b".to_string()),
                Token::Word("c  d".to_string()),
            ]
        );
    }

    #[test]
    fn test_tokenizer_operators() {
        let tokens = tokenize("ls -l | grep \".rs\" > out.txt", 0).unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Word("ls".to_string()),
                Token::Word("-l".to_string()),
                Token::Pipe,
                Token::Word("grep".to_string()),
                Token::Word(".rs".to_string()),
                Token::RedirectOut,
                Token::Word("out.txt".to_string()),
            ]
        );
    }

    #[test]
    fn test_tokenizer_unterminated_quote() {
        assert!(tokenize("echo \"hello", 0).is_err());
    }

    #[test]
    fn test_tokenizer_expansion() {
        let path_val = std::env::var("PATH").unwrap_or_default();
        let tokens = tokenize("echo $PATH '$PATH' \"$PATH\" $?", 127).unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Word("echo".to_string()),
                Token::Word(path_val.clone()),
                Token::Word("$PATH".to_string()),
                Token::Word(path_val),
                Token::Word("127".to_string()),
            ]
        );
    }

    #[test]
    fn test_parser_simple() {
        let tokens = tokenize("ls -l", 0).unwrap();
        let cp = parse(&tokens).unwrap();
        assert_eq!(cp.pipelines.len(), 1);
        let (pipeline, op) = &cp.pipelines[0];
        assert_eq!(*op, ChainOp::None);
        assert_eq!(pipeline.commands.len(), 1);
        assert_eq!(pipeline.commands[0].argv, vec!["ls", "-l"]);
    }

    #[test]
    fn test_parser_pipe() {
        let tokens = tokenize("ls | wc -l", 0).unwrap();
        let cp = parse(&tokens).unwrap();
        assert_eq!(cp.pipelines.len(), 1);
        let pipeline = &cp.pipelines[0].0;
        assert_eq!(pipeline.commands.len(), 2);
        assert_eq!(pipeline.commands[0].argv, vec!["ls"]);
        assert_eq!(pipeline.commands[1].argv, vec!["wc", "-l"]);
    }

    #[test]
    fn test_parser_redirection() {
        let tokens = tokenize("sort < names.txt > sorted.txt", 0).unwrap();
        let cp = parse(&tokens).unwrap();
        assert_eq!(cp.pipelines.len(), 1);
        let cmd = &cp.pipelines[0].0.commands[0];
        assert_eq!(cmd.argv, vec!["sort"]);
        assert_eq!(cmd.stdin_from, Some("names.txt".to_string()));
        assert_eq!(cmd.stdout_to, Some("sorted.txt".to_string()));
    }

    #[test]
    fn test_parser_chaining() {
        let tokens = tokenize("echo a && echo b || echo c", 0).unwrap();
        let cp = parse(&tokens).unwrap();
        assert_eq!(cp.pipelines.len(), 3);
        assert_eq!(cp.pipelines[0].1, ChainOp::And);
        assert_eq!(cp.pipelines[0].0.commands[0].argv, vec!["echo", "a"]);
        assert_eq!(cp.pipelines[1].1, ChainOp::Or);
        assert_eq!(cp.pipelines[1].0.commands[0].argv, vec!["echo", "b"]);
        assert_eq!(cp.pipelines[2].1, ChainOp::None);
        assert_eq!(cp.pipelines[2].0.commands[0].argv, vec!["echo", "c"]);
    }
}

