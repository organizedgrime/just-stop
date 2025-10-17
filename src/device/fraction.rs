use confique::Config;
use v4l::Fraction;

#[derive(Config, Debug, Clone)]
pub struct JustFraction {
    pub numerator: u32,
    pub denominator: u32,
}

impl From<Fraction> for JustFraction {
    fn from(value: Fraction) -> Self {
        JustFraction {
            numerator: value.numerator,
            denominator: value.denominator,
        }
    }
}

impl From<JustFraction> for Fraction {
    fn from(value: JustFraction) -> Self {
        Fraction {
            numerator: value.numerator,
            denominator: value.denominator,
        }
    }
}
