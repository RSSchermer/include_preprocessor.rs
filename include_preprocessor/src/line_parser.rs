use std::path::Path;

use nom::branch::alt;
use nom::bytes::complete::{is_not, tag};
use nom::character::complete::{char, line_ending, not_line_ending, space0, space1};
use nom::combinator::{not, opt, peek};
use nom::error::ErrorKind;
use nom::sequence::{delimited, tuple};
use nom::IResult;

#[derive(PartialEq, Debug)]
pub enum Line<'a> {
    Text,
    Include(IncludePath<'a>),
    PragmaOnce,
}

#[derive(PartialEq, Debug)]
pub enum IncludePath<'a> {
    Angle(&'a Path),
    Quote(&'a Path),
}

pub fn parse_line(input: &str) -> IResult<&str, Line> {
    alt((line_pragma_once, line_text, line_include))(input)
}

pub fn skip_line(input: &str) -> &str {
    let res: IResult<&str, (&str, &str), (&str, ErrorKind)> =
        tuple((not_line_ending, line_ending))(input);

    res.unwrap_or(("", ("", ""))).0
}

fn line_text(input: &str) -> IResult<&str, Line> {
    let (rem, _) = tuple((
        not(peek(tuple((tag("#include"), space1)))),
        not_line_ending,
        opt(line_ending),
    ))(input)?;

    Ok((rem, Line::Text))
}

fn line_pragma_once(input: &str) -> IResult<&str, Line> {
    let (rem, _) = tuple((tag("#pragma"), space1, tag("once"), space0, line_ending))(input)?;

    Ok((rem, Line::PragmaOnce))
}

fn line_include(input: &str) -> IResult<&str, Line> {
    let (rem, (_, _, path, _, _)) =
        tuple((tag("#include"), space1, include_path, space0, line_ending))(input)?;

    Ok((rem, Line::Include(path)))
}

fn include_path(input: &str) -> IResult<&str, IncludePath> {
    alt((angle_path, quote_path))(input)
}

fn angle_path(input: &str) -> IResult<&str, IncludePath> {
    let (rem, target) = delimited(char('<'), is_not(">\r\n"), char('>'))(input)?;

    Ok((rem, IncludePath::Angle(target.as_ref())))
}

fn quote_path(input: &str) -> IResult<&str, IncludePath> {
    let (rem, target) = delimited(char('"'), is_not("\"\r\n"), char('"'))(input)?;

    Ok((rem, IncludePath::Quote(target.as_ref())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_line() {
        let rem = "\
        Text line \n\
        #pragma once\n\
        #pragma     once     \n\
        #include <angle_path>\n\
        #include \"quote_path\"\n\
        \n\
        #unknowndirective\n\
        #pragma unknown\n\
        #include <angle_path_unclosed\n\
        #include \"quote_path_unclosed\n\
        #include undelimited\n\
        ";

        let res = parse_line(rem);

        assert!(res.is_ok());

        let (rem, line) = res.unwrap();

        assert_eq!(line, Line::Text);

        let res = parse_line(rem);

        assert!(res.is_ok());

        let (rem, line) = res.unwrap();

        assert_eq!(line, Line::PragmaOnce);

        let res = parse_line(rem);

        assert!(res.is_ok());

        let (rem, line) = res.unwrap();

        assert_eq!(line, Line::PragmaOnce);

        let res = parse_line(rem);

        assert!(res.is_ok());

        let (rem, line) = res.unwrap();

        assert_eq!(
            line,
            Line::Include(IncludePath::Angle("angle_path".as_ref()))
        );

        let res = parse_line(rem);

        assert!(res.is_ok());

        let (rem, line) = res.unwrap();

        assert_eq!(
            line,
            Line::Include(IncludePath::Quote("quote_path".as_ref()))
        );

        let res = parse_line(rem);

        assert!(res.is_ok());

        let (rem, line) = res.unwrap();

        assert_eq!(line, Line::Text);

        let res = parse_line(rem);

        assert!(res.is_ok());

        let (rem, line) = res.unwrap();

        assert_eq!(line, Line::Text);

        let res = parse_line(rem);

        assert!(res.is_ok());

        let (rem, line) = res.unwrap();

        assert_eq!(line, Line::Text);

        let res = parse_line(rem);

        assert!(res.is_err());

        let rem = skip_line(rem);

        let res = parse_line(rem);

        assert!(res.is_err());

        let rem = skip_line(rem);

        let res = parse_line(rem);

        assert!(res.is_err());

        let rem = skip_line(rem);
    }
}
