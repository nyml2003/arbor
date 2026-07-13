use std::{env, path::PathBuf, process::ExitCode};

use game_data_import::{ImportOptions, PINNED_COMMIT, generate_to_path};

const USAGE: &str = "usage: game-data-import --source DIR --output FILE --version-group IDENTIFIER [--locale zh-Hans] [--source-commit SHA]";

struct CliOptions {
    source: PathBuf,
    output: PathBuf,
    import: ImportOptions,
}

fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Option<CliOptions>, String> {
    let mut source = None;
    let mut output = None;
    let mut version_group = None;
    let mut locale = "zh-Hans".to_owned();
    let mut source_commit = PINNED_COMMIT.to_owned();
    let mut args = args.into_iter();
    while let Some(flag) = args.next() {
        if matches!(flag.as_str(), "--help" | "-h") {
            return Ok(None);
        }
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag.as_str() {
            "--source" => source = Some(PathBuf::from(value)),
            "--output" => output = Some(PathBuf::from(value)),
            "--version-group" => version_group = Some(value),
            "--locale" => locale = value,
            "--source-commit" => source_commit = value,
            _ => return Err(format!("unknown argument: {flag}")),
        }
    }
    Ok(Some(CliOptions {
        source: source.ok_or_else(|| "--source is required".to_owned())?,
        output: output.ok_or_else(|| "--output is required".to_owned())?,
        import: ImportOptions {
            locale,
            source_commit,
            version_group: version_group.ok_or_else(|| "--version-group is required".to_owned())?,
        },
    }))
}

fn main() -> ExitCode {
    let options = match parse_args(env::args().skip(1)) {
        Ok(Some(options)) => options,
        Ok(None) => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Err(error) => {
            eprintln!("{error}\n{USAGE}");
            return ExitCode::FAILURE;
        }
    };
    match generate_to_path(&options.source, &options.output, &options.import) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse_args;

    #[test]
    fn requires_an_explicit_version_group() {
        let error = parse_args([
            "--source".into(),
            "source".into(),
            "--output".into(),
            "output".into(),
        ])
        .err()
        .unwrap();
        assert_eq!(error, "--version-group is required");
    }

    #[test]
    fn help_does_not_require_a_value() {
        assert!(parse_args(["--help".into()]).unwrap().is_none());
    }
}
