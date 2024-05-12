use serde::{Deserialize, Serialize};

#[derive(
    Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, rune::Any, strum::Display,
)]
pub enum Currency {
    #[rune(constructor)]
    KRW,
    #[rune(constructor)]
    USDT,
    #[rune(constructor)]
    XRP,
    #[rune(constructor)]
    BTC,
    #[rune(constructor)]
    ARB,
    #[rune(constructor)]
    ETH,
    #[rune(constructor)]
    APT,
    #[rune(constructor)]
    SOL,
    #[rune(constructor)]
    SUI,
    #[rune(constructor)]
    AERGO,
    #[rune(constructor)]
    ATOM,
    #[rune(constructor)]
    IQ,
    #[rune(constructor)]
    XEM,
    #[rune(constructor)]
    QTUM,
    #[rune(constructor)]
    TRX,
    #[rune(constructor)]
    STRK,
    #[rune(constructor)]
    EOS,
    #[rune(constructor)]
    PEPE,
    #[rune(constructor)]
    DOGE,
    #[rune(constructor)]
    NEO,
    #[rune(constructor)]
    WLD,
    #[rune(constructor)]
    BIOT,
    #[rune(constructor)]
    POLA,
    #[rune(constructor)]
    BIGTIME,
    #[rune(constructor)]
    ONG,
    #[rune(constructor)]
    AGI,
    #[rune(constructor)]
    ACE,
    #[rune(constructor)]
    SHIB,
    #[rune(constructor)]
    HBAR,
    #[rune(constructor)]
    GLM,
}

pub trait CurrencyPairStringifier {
    fn stringify(c1: Currency, c2: Currency) -> Option<String>;
}

pub struct NoDelimiterCurrencyPairStringifier;
impl CurrencyPairStringifier for NoDelimiterCurrencyPairStringifier {
    fn stringify(c1: Currency, c2: Currency) -> Option<String> {
        Some(format!("{:?}{:?}", c1, c2))
    }
}

pub struct CurrencyPairDelimiterStringifier<const DELIMITER: char>;

impl<const DELIMITER: char> CurrencyPairStringifier
    for CurrencyPairDelimiterStringifier<DELIMITER>
{
    fn stringify(c1: Currency, c2: Currency) -> Option<String> {
        Some(format!("{:?}{}{:?}", c1, DELIMITER, c2))
    }
}
