use clap::{arg, builder::Command};
extern crate pest;
#[macro_use]
extern crate pest_derive;
use pest::Parser;

#[derive(Parser)]
#[grammar = "rust-src/grammar/grammar.pest"]
struct NixRunnerParser;

fn build_cli() -> Command {
    Command::new("nix-runner")
        .author("Evan Mattiza, <emattiza@gmail.com>")
        .about("An executor for runnable nix scripts")
        .arg(arg!(<in_script>))
        .arg(
            arg!([cmd] ... "commands to run")
                .num_args(1..)
                .allow_hyphen_values(true)
                .trailing_var_arg(true),
        )
}

fn main() {
    let matches = build_cli().get_matches();
    dbg!(matches);
}

#[cfg(test)]
mod test {
    use crate::{build_cli, NixRunnerParser};

    #[test]
    fn test_gets_script_and_subcommand_args() {
        let cmd = build_cli();
        let matches = cmd.get_matches_from(vec!["nix-runner", "test.py", "--poo", "pee"]);
        let script: &str = matches.get_one::<String>("in_script").unwrap();
        let commands: Vec<_> = matches.get_many::<String>("cmd").unwrap().collect();
        assert_eq!(script, "test.py");
        assert_eq!(commands, vec!["--poo", "pee"]);
    }
    #[test]
    fn test_parses_simple_file() {
        let parse = NixRunnerParser::parse(Rule::field, "-273.15");
        assert_eq!(parse, "-273.15")
    }
}
