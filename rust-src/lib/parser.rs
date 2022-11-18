use nom::{
    bytes::complete::tag,
    character::complete::{char, satisfy},
    combinator::value,
    multi::many1,
    sequence::separated_pair,
    AsChar, IResult,
};

#[derive(Debug, PartialEq)]
pub struct NixRunnerArgs {
    // #!pure (optional, once)
    pure: Option<bool>,
    // #!nix-option <key> <value> (optional, repeated)
    nix_option: Vec<(String, String)>,
    // #!registry <old-ref> <new-ref> (optional, repeated)
    registry: Vec<(String, String)>,
    // #!package <pkg-name> (optional, repeated)
    package: Vec<String>,
    // #!command <cmd-name> (optional, once)
    command: Option<String>,
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

fn pure_match(i: &str) -> IResult<&str, Option<bool>> {
    let (_cap, rest) = shebang_commands(i)?;
    value(Some(true), tag("pure"))(rest)
}

fn command_identifier(i: &str) -> IResult<&str, String> {
    many1(satisfy(|c| {
        c.is_alphanum() || c == '-' || c == '_' || c == '.'
    }))(i)
    .map(|(cap, chars)| (cap, chars.into_iter().collect()))
}

fn command_match(i: &str) -> IResult<&str, Option<String>> {
    let (_cap, rest) = shebang_commands(i)?;
    let (script_name, _rest) = tag("command ")(rest)?;
    let (rest, name) = command_identifier(script_name)?;
    Ok((rest, Some(name)))
}

fn package_identifier(i: &str) -> IResult<&str, String> {
    command_identifier(i)
}

fn package_match(i: &str) -> IResult<&str, String> {
    let (_cap, rest) = shebang_commands(i)?;
    let (script_name, _rest) = tag("package ")(rest)?;
    let (rest, name) = command_identifier(script_name)?;
    Ok((rest, name))
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

fn registry_match(i: &str) -> IResult<&str, (String, String)> {
    let (_cap, rest) = shebang_commands(i)?;
    let (references_remainder, _rest) = tag("registry ")(rest)?;
    let (rest, (old_ref, new_ref)) = separated_pair(
        nix_url_reference_identifier,
        satisfy(|c| c == ' '),
        nix_url_reference_identifier,
    )(references_remainder)?;
    Ok((rest, (old_ref, new_ref)))
}

fn nix_option_match(i: &str) -> IResult<&str, (String, String)> {
    let (_cap, rest) = shebang_commands(i)?;
    let (kv_remainder, _rest) = tag("nix-option ")(rest)?;
    let (rest, (key, value)) = separated_pair(
        command_identifier,
        satisfy(|c| c == ' '),
        command_identifier,
    )(kv_remainder)?;
    Ok((rest, (key, value)))
}

#[cfg(test)]
mod test {
    use crate::parser::nix_option_match;

    use super::command_match;
    use super::comment_char;
    use super::package_match;
    use super::pure_match;
    use super::registry_match;
    use super::shebang_commands;
    use super::shebang_sequence;

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
        assert_eq!(Ok(Some(true)), results)
    }

    #[test]
    fn test_command_match() {
        let results = command_match("#!command bash").map(|(_, opt)| opt);
        assert_eq!(Ok(Some("bash".to_string())), results);
    }

    #[test]
    fn test_package_match() {
        let results = package_match("#!package bash").map(|(_, opt)| opt);
        assert_eq!(Ok("bash".to_string()), results)
    }

    #[test]
    fn test_registry_match() {
        let old_ref = "nixpkgs".to_string();
        let new_ref =
            "github:NixOS/nixpkgs/0080a93cdf255b27e466116250b14b2bcd7b843b?dir=modules".to_string();
        let shebang_line = format!("#!registry {} {}", old_ref, new_ref);

        let results = registry_match(shebang_line.as_str()).map(|(_, opt)| opt);
        assert_eq!(Ok((old_ref, new_ref)), results)
    }
    #[test]
    fn test_option_match() {
        let key = "experimental-features".to_string();
        let value = "flakes".to_string();
        let shebang_line = format!("#!nix-option {} {}", key, value);
        dbg!(shebang_line.clone());
        let results = nix_option_match(shebang_line.as_str()).map(|(_, opt)| opt);
        assert_eq!(Ok((key, value)), results)
    }
}
