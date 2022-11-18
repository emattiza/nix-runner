use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{anychar, char, line_ending, satisfy},
    combinator::{complete, eof, value},
    multi::{many1, many_till, separated_list0},
    sequence::separated_pair,
    AsChar, IResult,
};

#[derive(Debug, PartialEq, Clone)]
pub struct NixOption {
    key: String,
    value: String,
}

#[derive(Debug, PartialEq, Clone)]
pub struct NixRegistryRef {
    old_ref: String,
    new_ref: String,
}

#[derive(Debug, PartialEq)]
pub struct NixRunnerArgs {
    // #!pure (optional, once)
    pure: Option<bool>,
    // #!nix-option <key> <value> (optional, repeated)
    nix_option: Vec<NixOption>,
    // #!registry <old-ref> <new-ref> (optional, repeated)
    registry: Vec<NixRegistryRef>,
    // #!package <pkg-name> (optional, repeated)
    package: Vec<String>,
    // #!command <cmd-name> (optional, once)
    command: Option<String>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum ShebangArg {
    PureLine(bool),
    NixOptionLine(NixOption),
    NixRegistryLine(NixRegistryRef),
    PackageLine(String),
    CommandLine(String),
}

impl Default for NixRunnerArgs {
    fn default() -> Self {
        Self {
            pure: Some(false),
            nix_option: vec![],
            registry: vec![],
            package: vec![],
            command: Some("bash".to_string()),
        }
    }
}

fn comment_char(i: &str) -> IResult<&str, ()> {
    value((), char('#'))(i)
}

fn shebang_sequence(i: &str) -> IResult<&str, &str> {
    tag("#!")(i)
}

fn shebang_commands(i: &str) -> IResult<&str, &str> {
    let (cap, _rest) = shebang_sequence(i)?;
    return Ok(("", cap));
}

fn pure_match(i: &str) -> IResult<&str, ShebangArg> {
    let (_cap, rest) = shebang_commands(i)?;
    value(ShebangArg::PureLine(true), tag("pure"))(rest)
}

fn command_identifier(i: &str) -> IResult<&str, String> {
    many1(satisfy(|c| {
        c.is_alphanum() || c == '-' || c == '_' || c == '.'
    }))(i)
    .map(|(cap, chars)| (cap, chars.into_iter().collect()))
}

fn command_match(i: &str) -> IResult<&str, ShebangArg> {
    let (_cap, rest) = shebang_commands(i)?;
    let (script_name, _rest) = tag("command ")(rest)?;
    let (rest, name) = command_identifier(script_name)?;
    Ok((rest, ShebangArg::CommandLine(name)))
}

fn package_identifier(i: &str) -> IResult<&str, String> {
    command_identifier(i)
}

fn package_match(i: &str) -> IResult<&str, ShebangArg> {
    let (_cap, rest) = shebang_commands(i)?;
    let (script_name, _rest) = tag("package ")(rest)?;
    let (rest, name) = command_identifier(script_name)?;
    Ok((rest, ShebangArg::PackageLine(name)))
}

fn nix_url_reference_identifier(i: &str) -> IResult<&str, String> {
    many1(satisfy(|c| {
        c.is_alphanum()
            || c == '-'
            || c == '_'
            || c == '.'
            || c == ':'
            || c == '/'
            || c == '?'
            || c == '='
    }))(i)
    .map(|(cap, chars)| (cap, chars.into_iter().collect()))
}

fn nix_registry_match(i: &str) -> IResult<&str, ShebangArg> {
    let (_cap, rest) = shebang_commands(i)?;
    let (references_remainder, _rest) = tag("registry ")(rest)?;
    let (rest, (old_ref, new_ref)) = separated_pair(
        nix_url_reference_identifier,
        satisfy(|c| c == ' '),
        nix_url_reference_identifier,
    )(references_remainder)?;
    let registry_ref = NixRegistryRef { old_ref, new_ref };
    Ok((rest, ShebangArg::NixRegistryLine(registry_ref)))
}

fn nix_option_match(i: &str) -> IResult<&str, ShebangArg> {
    let (_cap, rest) = shebang_commands(i)?;
    let (kv_remainder, _rest) = tag("nix-option ")(rest)?;
    let (rest, (key, value)) = separated_pair(
        command_identifier,
        satisfy(|c| c == ' '),
        command_identifier,
    )(kv_remainder)?;
    Ok((rest, ShebangArg::NixOptionLine(NixOption { key, value })))
}

fn any_arg_match(i: &str) -> IResult<&str, ShebangArg> {
    let matches = (
        nix_option_match,
        nix_registry_match,
        pure_match,
        command_match,
        package_match,
    );
    alt(matches)(i)
}

fn many_arg_match(i: &str) -> IResult<&str, Vec<ShebangArg>> {
    separated_list0(line_ending, any_arg_match)(i)
}

fn parse_nix_runner_file(i: &str) -> IResult<&str, NixRunnerArgs> {
    let mut default = NixRunnerArgs::default();
    let (_, rest) = shebang_sequence(i)?;
    let (rest, _) = many_till(anychar, line_ending)(rest)?;
    let (rest, args) = many_arg_match(rest)?;
    let (rest, _) = line_ending(rest)?;
    let (rest, body): (&str, (Vec<char>, &str)) = complete(many_till(anychar, eof))(rest)?;

    for arg in args {
        match arg {
            ShebangArg::PureLine(purity) => {
                default.pure = Some(purity);
            }
            ShebangArg::NixOptionLine(option) => {
                default.nix_option.push(option);
            }
            ShebangArg::NixRegistryLine(registry_ref) => {
                default.registry.push(registry_ref);
            }
            ShebangArg::PackageLine(package) => {
                default.package.push(package);
            }
            ShebangArg::CommandLine(command) => {
                default.command = Some(command);
            }
        }
    }
    Ok((rest, default))
}

#[cfg(test)]
mod test {
    use crate::parser::any_arg_match;
    use crate::parser::nix_option_match;

    use super::command_match;
    use super::comment_char;
    use super::many_arg_match;
    use super::nix_registry_match;
    use super::package_match;
    use super::pure_match;
    use super::shebang_commands;
    use super::shebang_sequence;
    use super::NixOption;
    use super::NixRegistryRef;
    use super::ShebangArg;

    #[test]
    fn test_bash_comment() {
        let results = comment_char("#");
        assert_eq!(Ok(("", ())), results)
    }

    #[test]
    fn test_bash_shebang() {
        let results = shebang_sequence("#!");
        assert_eq!(Ok(("", "#!")), results)
    }
    #[test]
    fn test_bash_shebang_with_contents() {
        let results = shebang_commands("#!/usr/bin/env nix");
        assert_eq!(Ok(("", "/usr/bin/env nix")), results)
    }

    #[test]
    fn test_pure_match() {
        let results = pure_match("#!pure").map(|(_, opt)| opt);
        let expected = Ok(ShebangArg::PureLine(true));
        assert_eq!(expected, results)
    }

    #[test]
    fn test_command_match() {
        let results = command_match("#!command bash").map(|(_, opt)| opt);
        let expected = Ok(ShebangArg::CommandLine("bash".to_string()));
        assert_eq!(expected, results);
    }

    #[test]
    fn test_package_match() {
        let results = package_match("#!package bash").map(|(_, opt)| opt);
        let expected = Ok(ShebangArg::PackageLine("bash".to_string()));
        assert_eq!(expected, results)
    }

    #[test]
    fn test_registry_match() {
        let old_ref = "nixpkgs".to_string();
        let new_ref =
            "github:NixOS/nixpkgs/0080a93cdf255b27e466116250b14b2bcd7b843b?dir=modules".to_string();
        let shebang_line = format!("#!registry {} {}", old_ref, new_ref);

        let results = nix_registry_match(shebang_line.as_str()).map(|(_, opt)| opt);
        let expected = Ok(ShebangArg::NixRegistryLine(NixRegistryRef {
            old_ref,
            new_ref,
        }));
        assert_eq!(expected, results)
    }
    #[test]
    fn test_option_match() {
        let key = "experimental-features".to_string();
        let value = "flakes".to_string();
        let shebang_line = format!("#!nix-option {} {}", key, value);
        let results = nix_option_match(shebang_line.as_str()).map(|(_, opt)| opt);
        let expected = Ok(ShebangArg::NixOptionLine(NixOption { key, value }));
        assert_eq!(expected, results)
    }

    #[test]
    fn test_any_arg_match() {
        let results = any_arg_match("#!command bash").map(|(_, opt)| opt);
        let expected = Ok(ShebangArg::CommandLine("bash".to_string()));
        assert_eq!(expected, results);
    }

    #[test]
    fn test_many_arg_match() {
        let results = many_arg_match("#!package bash\n#!command bash\nset -euox pipefail");
        let expected = Ok((
            "\nset -euox pipefail",
            vec![
                ShebangArg::PackageLine("bash".to_string()),
                ShebangArg::CommandLine("bash".to_string()),
            ],
        ));
        assert_eq!(expected, results)
    }
}
