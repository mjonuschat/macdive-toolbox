use nom::Finish;
use nom::Parser;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::{alphanumeric1, digit0, digit1, multispace0};
use nom::combinator::{all_consuming, eof, map, opt, value};
use nom::error::Error;
use nom::multi::many_till;
use nom::sequence::{delimited, terminated};

fn sp_term(input: &str) -> nom::IResult<&str, &str> {
    terminated(alt((tag("sp."), tag("spp."))), multispace0).parse(input)
}

fn sp_term_with_index(input: &str) -> nom::IResult<&str, String> {
    map((sp_term, multispace0, digit0), |(term, _, digit)| {
        format!("{term}{digit}")
    })
    .parse(input)
}

fn sp_range(input: &str) -> nom::IResult<&str, String> {
    map(
        (
            sp_term_with_index,
            opt(delimited(multispace0, tag("-"), multispace0)),
            opt(sp_term_with_index),
        ),
        |(sp1, _, sp2)| match sp2 {
            Some(sp2) => format!("{sp1}-{sp2}"),
            None => sp1,
        },
    )
    .parse(input)
}

fn number_range(input: &str) -> nom::IResult<&str, String> {
    map(
        (
            delimited(multispace0, digit1, multispace0),
            tag("-"),
            delimited(multispace0, digit1, multispace0),
        ),
        |(p1, p2, p3)| [p1, p2, p3].join(" "),
    )
    .parse(input)
}

fn stop_words(input: &str) -> nom::IResult<&str, &str> {
    delimited(multispace0, tag("cf."), multispace0).parse(input)
}

fn word(input: &str) -> nom::IResult<&str, &str> {
    delimited(multispace0, alphanumeric1, alt((stop_words, multispace0))).parse(input)
}

fn species_name(input: &str) -> nom::IResult<&str, String> {
    map(
        all_consuming(many_till(
            word,
            alt((sp_range, number_range, value("".to_string(), eof))),
        )),
        |(words, _)| words.join(" "),
    )
    .parse(input)
}

/// Sanitizes a species name by stripping trailing sp. ranges, number ranges,
/// and "cf." qualifiers, returning only the genus/species words.
///
/// # Errors
///
/// Returns [`crate::error::Error::ParseError`] if the input cannot be parsed
/// as a valid species name pattern.
pub fn sanitize_species_name(input: &str) -> crate::error::Result<String> {
    match species_name(input).finish() {
        Ok((_remaining, name)) => {
            if input != name {
                tracing::trace!(
                    original_name = input,
                    normalized_name = name,
                    "Normalized name"
                );
            }
            Ok(name)
        }
        Err(Error { input, code }) => Err(crate::error::Error::ParseError(format!(
            "{:?}: {input}",
            code
        ))),
    }
}

#[cfg(test)]
mod test {
    fn normalize_sp_range(input: &str) -> String {
        super::sp_range(input).map(|(_, s)| s).unwrap()
    }

    fn sanitize_name(input: &str) -> String {
        super::sanitize_species_name(input).unwrap()
    }

    #[test]
    fn test_sp_range() {
        assert_eq!("sp.1-sp.2", normalize_sp_range("sp.1-sp.2"));
        assert_eq!("sp.1-sp.5", normalize_sp_range("sp.1 - sp.5"));
        assert_eq!("sp.1-sp.5", normalize_sp_range("sp.1-sp.5"));
        assert_eq!("sp.1-sp.4", normalize_sp_range("sp.1 - sp. 4"));
        assert_eq!("sp.1-sp.4", normalize_sp_range("sp.1 -sp.4"));
        assert_eq!("sp.1-sp.4", normalize_sp_range("sp. 1- sp. 4"));
        assert_eq!("sp.1-sp.4", normalize_sp_range("sp. 1-sp. 4"));
        assert_eq!("sp.1-sp.4", normalize_sp_range("sp.1- sp.4"));
    }

    #[test]
    fn test_sanitize_name() {
        assert_eq!("Comaster schlegelii", sanitize_name("Comaster schlegelii"));
        assert_eq!("Diadema", sanitize_name("Diadema sp.1 - sp.4"));
        assert_eq!("Eunice australis", sanitize_name("Eunice cf. australis"));
        assert_eq!("Phrikoceros", sanitize_name("Phrikoceros sp.1-sp.2"));
        assert_eq!(
            "Hamodactylus noumeae",
            sanitize_name("Hamodactylus cf. noumeae 1 - 4")
        );
    }
}
