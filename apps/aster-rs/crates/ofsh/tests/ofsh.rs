use ofsh::{
    builtin_registry, execute, resolve, ArgumentKind, CommandRegistry, CommandResult, CommandSpec,
    CompletionEngine, CompletionKind, ExecutionError, Lexer, OfshError, OfshSession, Parser,
    RedirectionOperator, TokenKind,
};

#[test]
fn lexer_handles_quotes_pipes_and_redirects() {
    let tokens = Lexer::new(r#"echo "a\nb" 'raw' """long""" | grep b >> out.txt"#)
        .tokenize()
        .unwrap();
    let kinds = tokens.iter().map(|token| token.kind).collect::<Vec<_>>();

    assert_eq!(
        kinds,
        vec![
            TokenKind::Word,
            TokenKind::StringDouble,
            TokenKind::StringSingle,
            TokenKind::StringTriple,
            TokenKind::Pipe,
            TokenKind::Word,
            TokenKind::Word,
            TokenKind::RedirectAppend,
            TokenKind::Word,
            TokenKind::Eof,
        ]
    );
    assert_eq!(tokens[1].value, "a\nb");
    assert_eq!(tokens[3].value, "long");
}

#[test]
fn lexer_reports_unclosed_quote() {
    let error = Lexer::new(r#"echo "unterminated"#).tokenize().unwrap_err();

    assert!(error.message.contains("Unclosed double quote"));
}

#[test]
fn lexer_reports_invalid_escape() {
    let error = Lexer::new(r#"echo "\x""#).tokenize().unwrap_err();

    assert!(error.message.contains("Invalid escape sequence"));
}

#[test]
fn parser_builds_pipeline_and_redirection_ast() {
    let tokens = Lexer::new("cat /mem/a.txt | grep needle > out.txt")
        .tokenize()
        .unwrap();
    let statement = Parser::new(&tokens).parse().unwrap();

    assert_eq!(statement.pipeline.commands.len(), 2);
    assert_eq!(statement.pipeline.commands[0].name.value, "cat");
    assert_eq!(statement.pipeline.commands[0].args[0].value, "/mem/a.txt");
    assert_eq!(statement.pipeline.commands[1].name.value, "grep");
    assert_eq!(
        statement.redirection.as_ref().unwrap().operator,
        RedirectionOperator::Truncate
    );
    assert_eq!(statement.redirection.unwrap().target.value, "out.txt");
}

#[test]
fn parser_requires_plain_word_command_name() {
    let tokens = Lexer::new(r#""echo" hello"#).tokenize().unwrap();
    let error = Parser::new(&tokens).parse().unwrap_err();

    assert!(error.message.contains("plain word"));
}

#[test]
fn registry_registers_unregisters_and_lists_commands() {
    let mut registry = CommandRegistry::new();
    registry.register(CommandSpec::new("x", "test command"), |_| {
        CommandResult::output("ok")
    });

    assert!(registry.has("x"));
    assert_eq!(registry.list(), vec!["x".to_string()]);
    assert_eq!(registry.get("x").unwrap().call(&[]).output, "ok");
    assert_eq!(registry.unregister("x").unwrap().name, "x");
    assert!(!registry.has("x"));
}

#[test]
fn resolver_rejects_unknown_commands() {
    let tokens = Lexer::new("missing arg").tokenize().unwrap();
    let statement = Parser::new(&tokens).parse().unwrap();
    let error = resolve(&statement, &CommandRegistry::new()).unwrap_err();

    assert_eq!(error.command_name, "missing");
}

#[test]
fn executor_runs_registered_single_command() {
    let mut registry = CommandRegistry::new();
    registry.register(CommandSpec::new("join", "join args"), |args| {
        CommandResult::output(args.join(","))
    });
    let tokens = Lexer::new("join a b").tokenize().unwrap();
    let statement = Parser::new(&tokens).parse().unwrap();
    resolve(&statement, &registry).unwrap();

    let result = execute(&statement, &registry).unwrap();

    assert_eq!(result.output, "a,b");
    assert!(!result.exit);
}

#[test]
fn executor_rejects_pipes_and_redirection_for_v1() {
    let registry = builtin_registry();
    let pipe = Parser::new(&Lexer::new("echo a | echo b").tokenize().unwrap())
        .parse()
        .unwrap();
    let redirect = Parser::new(&Lexer::new("echo a > out.txt").tokenize().unwrap())
        .parse()
        .unwrap();

    assert!(matches!(
        execute(&pipe, &registry),
        Err(ExecutionError::UnsupportedFeature {
            feature: "pipe",
            ..
        })
    ));
    assert!(matches!(
        execute(&redirect, &registry),
        Err(ExecutionError::UnsupportedFeature {
            feature: "redirection",
            ..
        })
    ));
}

#[test]
fn session_executes_builtin_commands() {
    let session = OfshSession::with_builtins();

    assert_eq!(session.execute("echo hello").unwrap().output, "hello\n");
    assert!(session
        .execute("help")
        .unwrap()
        .output
        .contains("Commands:"));
    assert!(session.execute("exit").unwrap().exit);
    assert!(session.execute("quit").unwrap().exit);
}

#[test]
fn session_returns_structured_errors() {
    let session = OfshSession::with_builtins();

    assert!(matches!(
        session.execute("missing"),
        Err(OfshError::Resolve(_))
    ));
    assert!(matches!(
        session.execute("echo a | echo b"),
        Err(OfshError::Execute(ExecutionError::UnsupportedFeature {
            feature: "pipe",
            ..
        }))
    ));
}

#[test]
fn empty_session_input_is_noop() {
    let session = OfshSession::with_builtins();

    assert_eq!(session.execute("   ").unwrap(), CommandResult::output(""));
}

#[test]
fn completion_returns_command_and_argument_candidates() {
    let mut registry = CommandRegistry::new();
    registry.register(
        CommandSpec::new("/theme", "Switch theme").with_arg_completions([
            "dark",
            "light",
            "high-contrast",
        ]),
        |_| CommandResult::output(""),
    );
    registry.register(
        CommandSpec::new("/model", "Switch model")
            .with_arg_completions(["deepseek-chat", "deepseek-reasoner"]),
        |_| CommandResult::output(""),
    );
    let completion = CompletionEngine::new(&registry);

    assert_eq!(
        completion
            .complete("/")
            .into_iter()
            .map(|item| item.value)
            .collect::<Vec<_>>(),
        vec!["/model".to_string(), "/theme".to_string()]
    );
    assert_eq!(
        completion
            .complete("/theme l")
            .into_iter()
            .map(|item| (item.kind, item.value))
            .collect::<Vec<_>>(),
        vec![(CompletionKind::Argument, "light".to_string())]
    );
    assert_eq!(
        completion
            .complete("/model ")
            .into_iter()
            .map(|item| item.value)
            .collect::<Vec<_>>(),
        vec!["deepseek-chat".to_string(), "deepseek-reasoner".to_string()]
    );
}

#[test]
fn parser_preserves_argument_kinds() {
    let tokens = Lexer::new(r#"echo word "double" 'single' """triple""""#)
        .tokenize()
        .unwrap();
    let statement = Parser::new(&tokens).parse().unwrap();
    let kinds = statement.pipeline.commands[0]
        .args
        .iter()
        .map(|arg| arg.kind)
        .collect::<Vec<_>>();

    assert_eq!(
        kinds,
        vec![
            ArgumentKind::Word,
            ArgumentKind::StringDouble,
            ArgumentKind::StringSingle,
            ArgumentKind::StringTriple,
        ]
    );
}
