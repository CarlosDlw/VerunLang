use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "parser/verun.pest"]
pub struct VerunParser;

pub type ParseResult<'i> = Result<pest::iterators::Pairs<'i, Rule>, pest::error::Error<Rule>>;

pub fn parse(input: &str) -> ParseResult<'_> {
    VerunParser::parse(Rule::program, input)
}
