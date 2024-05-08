use logos::{Logos, SpannedIter};
use rust_decimal::Decimal;

use crate::currency::Currency;

use super::ParseError;

fn parse_currency_pair(s: &str) -> Option<(Currency, Currency)> {
    let mut iter = s.split('-');
    let from = iter.next()?;
    let to = iter.next()?;
    Some((from.parse().ok()?, to.parse().ok()?))
}

#[derive(Logos, Clone, Debug, PartialEq)]
#[logos(skip r"[ \t\n\f]+")]
pub enum Token<'token> {
    #[token("let")]
    Let,

    #[token("fn")]
    Fn,

    #[token("struct")]
    Struct,

    #[token("enum")]
    Enum,

    #[token("async")]
    Async,

    #[token("await")]
    Await,

    #[token("drop")]
    Drop,

    #[token("if")]
    If,

    #[token("else")]
    Else,

    #[token("while")]
    While,

    #[token("for")]
    For,

    #[token("trait")]
    Trait,

    #[regex("[A-Z]{2,5}-[A-Z]{2,5}", |lex| parse_currency_pair(lex.slice()))]
    CurrencyPair((Currency, Currency)),

    #[regex("[A-Z]{2,5}", |lex| lex.slice().parse().ok())]
    Currency(Currency),

    #[regex("(true|false)", |lex| lex.slice().parse().ok())]
    Boolean(bool),

    #[regex(r#""([^"\\]|\\["\\bnfrt]|u[a-fA-F0-9]{4})*""#, |lex| lex.slice())]
    String(&'token str),

    #[regex("[_a-zA-Z][_0-9a-zA-Z\\-]*", |lex| lex.slice())]
    Text(&'token str),

    #[regex(r#"\d+"#, |lex| lex.slice().parse().ok())]
    Integer(u128),

    #[regex(r#"\d+\.\d+"#, |lex| lex.slice().parse().ok())]
    Decimal(Decimal),

    #[token("#")]
    Sharp,

    #[token("@")]
    At,

    #[token("*")]
    Asterisk,

    #[token("/")]
    Slash,

    #[token(".")]
    Dot,

    #[token("[")]
    LBracket,

    #[token("]")]
    RBracket,

    #[token("<")]
    Lt,

    #[token(">")]
    Gt,

    #[token("<=")]
    Le,

    #[token(">=")]
    Ge,

    #[token("==")]
    Eq,

    #[token("!=")]
    Ne,

    #[token("(")]
    LParen,

    #[token(")")]
    RParen,

    #[token("{")]
    LBrace,

    #[token("}")]
    RBrace,

    #[token("$")]
    Dollar,

    #[token("+")]
    Add,

    #[token("-")]
    Sub,

    #[token(",")]
    Comma,

    #[token(":")]
    Colon,

    #[token("::")]
    DoubleColon,

    #[token(";")]
    Semicolon,

    #[token("=")]
    Assign,

    #[token("|")]
    BitwiseOr,

    #[token("||")]
    LogicalOr,

    #[token("&")]
    BitwiseAnd,

    #[token("&&")]
    LogicalAnd,

    #[token("->")]
    RArrow,
}

pub type Spanned<Tok, Loc, Error> = Result<(Loc, Tok, Loc), Error>;

pub struct Lexer<'input> {
    token_stream: SpannedIter<'input, Token<'input>>,
}

impl<'input> Lexer<'input> {
    pub fn new(input: &'input str) -> Self {
        Self {
            token_stream: Token::lexer(input).spanned(),
        }
    }
}

impl<'input> Iterator for Lexer<'input> {
    type Item = Spanned<Token<'input>, usize, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.token_stream.next().map(|(token, span)| match token {
            Ok(token) => Ok((span.start, token, span.end)),
            Err(_) => Err(ParseError::InvalidToken(span.clone())),
        })
    }
}
