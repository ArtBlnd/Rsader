#[derive(strum::Display, strum::EnumString, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Currency {
    KRW,
    USDT,
    XRP,
    BTC,
    ARB,
    ETH,
    APT,
    SOL,
    SUI,
    AERGO,
    ATOM,
    IQ,
    XEM,
    QTUM,
    TRX,
    STRK,
    EOS,
    PEPE,
    DOGE,
    NEO,
    WLD,
    BIOT,
    POLA,
    BIGTIME,
    ONG,
    AGI,
    ACE,
    SHIB,
    HBAR,
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
