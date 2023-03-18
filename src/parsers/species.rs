use anyhow::bail;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::{alphanumeric1, digit0, digit1, multispace0};
use nom::combinator::{all_consuming, eof, map, opt, value};
use nom::error::Error;
use nom::multi::many_till;
use nom::sequence::{delimited, terminated, tuple};
use nom::{Finish, IResult};

fn sp_term(input: &str) -> IResult<&str, &str> {
    terminated(alt((tag("sp."), tag("spp."))), multispace0)(input)
}

fn sp_term_with_index(input: &str) -> IResult<&str, String> {
    map(tuple((sp_term, multispace0, digit0)), |(term, _, digit)| {
        format!("{term}{digit}")
    })(input)
}

fn sp_range(input: &str) -> IResult<&str, String> {
    map(
        tuple((
            sp_term_with_index,
            opt(delimited(multispace0, tag("-"), multispace0)),
            opt(sp_term_with_index),
        )),
        |(sp1, _, sp2)| match sp2 {
            Some(sp2) => format!("{sp1}-{sp2}"),
            None => sp1,
        },
    )(input)
}

fn number_range(input: &str) -> IResult<&str, String> {
    map(
        tuple((
            delimited(multispace0, digit1, multispace0),
            tag("-"),
            delimited(multispace0, digit1, multispace0),
        )),
        |(p1, p2, p3)| [p1, p2, p3].join(" "),
    )(input)
}
fn stop_words(input: &str) -> IResult<&str, &str> {
    delimited(multispace0, tag("cf."), multispace0)(input)
}

fn word(input: &str) -> IResult<&str, &str> {
    delimited(multispace0, alphanumeric1, alt((stop_words, multispace0)))(input)
}

fn species_name(input: &str) -> IResult<&str, String> {
    map(
        all_consuming(many_till(
            word,
            alt((sp_range, number_range, value("".to_string(), eof))),
        )),
        |(words, _)| words.join(" "),
    )(input)
}

pub(crate) fn sanitize_species_name(input: &str) -> anyhow::Result<String> {
    match species_name(input).finish() {
        Ok((_remaining, name)) => {
            if input != name {
                tracing::trace!(
                    original_namp = input,
                    normalized_name = name,
                    "Normalized name"
                );
            }
            Ok(name)
        }
        Err(Error { input, code }) => bail!("Error: {:?}, Input: {input}", code),
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
