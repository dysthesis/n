use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::{escaped_transform, is_not, tag, take_while_m_n, take_while1},
    character::{
        complete::{alpha1, char, multispace0, one_of},
        streaming::multispace1,
    },
    combinator::{cut, map, map_res, value},
    error::{ContextError, context},
    sequence::{delimited, preceded, terminated, tuple},
};

use crate::document::Document;

pub enum Query {
    Contains { key: String, value: String },
    Not(Box<Query>),
    And(Box<Query>, Box<Query>),
    Or(Box<Query>, Box<Query>),
    Xor(Box<Query>, Box<Query>),
}

impl Query {
    /// Check if a document matches the given query
    pub fn matches(&self, document: &Document) -> bool {
        match self {
            Query::Contains { key, value } => document
                .get_metadata(key)
                .map_or_else(|| false, |target| target.contains(value)),
            Query::Not(query) => !query.matches(document),
            Query::And(left, right) => left.matches(document) && right.matches(document),
            Query::Or(left, right) => left.matches(document) || right.matches(document),
            Query::Xor(left, right) => left.matches(document) ^ right.matches(document),
        }
    }
    pub fn parse(input: &str) -> Result<Query, nom::error::Error<&str>> {
        fn ident(i: &str) -> IResult<&str, &str> {
            context("identifier", preceded(multispace0, alpha1)).parse(i)
        }

        fn str_lit(i: &str) -> IResult<&str, &str> {
            delimited(
                preceded(multispace0, char('"')),
                context("string", cut(is_not("\""))),
                char('"'),
            )
            .parse(i)
        }

        fn is_bare_atom_char(c: char) -> bool {
            !c.is_whitespace() && c != '(' && c != ')'
        }

        /// Parse an unquoted atom such as foo-bar, 123, @x, ε=mc².
        fn bare_atom(i: &str) -> IResult<&str, String> {
            map(take_while1(is_bare_atom_char), str::to_owned).parse(i)
        }

        fn single_quoted_string(i: &str) -> IResult<&str, String> {
            delimited(
                char('\''),
                escaped_transform(
                    is_not("'\\"),
                    '\\',
                    alt((
                        value("\\", char('\\')),
                        value("\'", char('\'')),
                        value("\n", char('n')),
                        value("\r", char('r')),
                        value("\t", char('t')),
                    )),
                ),
                char('\''),
            )
            .parse(i)
        }

        fn double_quoted_string(i: &str) -> IResult<&str, String> {
            delimited(
                char('"'),
                escaped_transform(
                    is_not("\"\\"),
                    '\\',
                    alt((
                        value("\\", char('\\')),
                        value("\"", char('"')),
                        value("\n", char('n')),
                        value("\r", char('r')),
                        value("\t", char('t')),
                    )),
                ),
                char('"'),
            )
            .parse(i)
        }
        fn atom(i: &str) -> IResult<&str, String> {
            preceded(
                multispace0,
                alt((double_quoted_string, single_quoted_string, bare_atom)),
            )
            .parse(i)
        }

        fn s_exp<'a, F>(
            inner: F,
        ) -> impl Parser<&'a str, Output = Query, Error = nom::error::Error<&'a str>>
        where
            F: Parser<&'a str, Output = Query, Error = nom::error::Error<&'a str>>,
            <F as nom::Parser<&'a str>>::Error: ContextError<&'a str>,
        {
            delimited(
                preceded(multispace0, char('(')),
                preceded(multispace0, inner),
                context("closing paren", cut(preceded(multispace0, char(')')))),
            )
        }

        fn parse_contains(i: &str) -> IResult<&str, Query> {
            let inner = map(
                preceded(
                    terminated(tag("contains"), multispace1),
                    cut(tuple((atom, preceded(multispace1, atom)))),
                ),
                |(key, value)| Query::Contains { key, value },
            );
            s_exp(inner).parse(i)
        }

        fn parse_not(i: &str) -> IResult<&str, Query> {
            let inner = map(
                preceded(terminated(tag("not"), multispace1), cut(parse_query)),
                |q| Query::Not(Box::new(q)),
            );
            s_exp(inner).parse(i)
        }

        /// Helper for the three binary connectives.
        fn parse_binary<'a>(
            name: &'static str,
            ctor: fn(Box<Query>, Box<Query>) -> Query,
        ) -> impl FnMut(&'a str) -> IResult<&'a str, Query> {
            move |i: &'a str| {
                let inner = map(
                    preceded(
                        terminated(tag(name), multispace1),
                        cut(tuple((parse_query, preceded(multispace1, parse_query)))),
                    ),
                    |(lhs, rhs)| ctor(Box::new(lhs), Box::new(rhs)),
                );
                s_exp(inner).parse(i)
            }
        }

        fn parse_and(i: &str) -> IResult<&str, Query> {
            parse_binary("and", Query::And)(i)
        }
        fn parse_or(i: &str) -> IResult<&str, Query> {
            parse_binary("or", Query::Or)(i)
        }
        fn parse_xor(i: &str) -> IResult<&str, Query> {
            parse_binary("xor", Query::Xor)(i)
        }

        fn parse_query(i: &str) -> IResult<&str, Query> {
            preceded(
                multispace0,
                alt((parse_contains, parse_not, parse_and, parse_or, parse_xor)),
            )
            .parse(i)
        }

        // WARN: Fix this unwrap; I'm only doing this to get it to work for now
        let (rest, q) = parse_query(input).unwrap();
        if rest.trim().is_empty() {
            Ok(q)
        } else {
            // Figure out actual error reporting when this works
            Err(nom::error::Error::new(rest, nom::error::ErrorKind::Not))
        }
    }
}
