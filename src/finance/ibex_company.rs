// Copyright 2024 Felipe Torres González

//! Simplified Finance Library implementation for Ibex indexes and companies.
//!
//! The [Finance Library][financelib] defines an API that needs to be implemented for
//! a particular use case. This library implements such API for [Ibex indexes][ibexindexes]
//! and companies that are included in these indexes but without implementing the trait.
//!
//! This design decision is due to an integration problem with [Teloxide][teloxide]. This
//! might get fixed in future versions.
//!
//! [financelib]: https://github.com/felipet/finance_api
//! [ibexindexes]: https://www.bolsasymercados.es/bme-exchange/en/Indices/Ibex

use std::fmt;

/// An relaxed implementation of the [Company][company] trait for a company that
/// is included in some index of the Ibex family.
///
/// [company]: https://docs.rs/finance_api/0.1.0/finance_api/trait.Company.html
/// [teloxide]: https://docs.rs/teloxide/latest/teloxide/
#[derive(Clone)]
pub struct IbexCompany {
    /// This is the full legal name of the company. Optional.
    full_name: Option<String>,
    /// This is the usual name for the company, usually some sort of contraction
    /// of the _full name_.
    short_name: String,
    /// The identifier of the company in the market.
    ticker: String,
    /// The _International Securities Identification Number_.
    isin: String,
    /// A local identifier for Spanish companies. This is optional as some companies,
    /// which are included in an Ibex index, might be registered in another country.
    nif: Option<String>,
}

impl IbexCompany {
    /// Constructor of the [IbexCompany] object.
    ///
    /// # Description
    ///
    /// The constructor shall receive all the information related to a stock.
    ///
    /// ## Arguments
    ///
    /// - _fname_: Optional full name of the company. Useful for companies with very long names,
    ///            such as IAG (International Airlines Group).
    /// - _sname_: Short name. Usually part of the full name or the ticker.
    /// - _ticker_: The ticker of the company in the IBEX35 market.
    /// - _isin_: The ISIN number.
    /// - _nif_: _Número de Identificación Fiscal_. It is only applicable to Spanish companies, hence optional.
    ///
    /// Input values are not checked to ensure those comply with the expected format.
    pub fn new(
        fname: Option<&str>,
        sname: &str,
        ticker: &str,
        isin: &str,
        nif: Option<&str>,
    ) -> IbexCompany {
        IbexCompany {
            full_name: fname.map(String::from),
            short_name: String::from(sname),
            ticker: String::from(ticker),
            isin: String::from(isin),
            nif: nif.map(String::from),
        }
    }

    /// Get the most common name of the stock.
    pub fn name(&self) -> &str {
        &self.short_name
    }

    /// Get the legal or full name of the stock.
    ///
    /// # Description
    ///
    /// This method might return `None` if a full name was not provided for a
    /// particular stock. This is common in cases in which the short name is equal
    pub fn full_name(&self) -> Option<&String> {
        self.full_name.as_ref()
    }

    /// Get the [ISIN](https://en.wikipedia.org/wiki/International_Securities_Identification_Number)
    /// of a stock.
    pub fn isin(&self) -> &str {
        &self.isin
    }

    /// Get the ticker of a stock.
    pub fn ticker(&self) -> &str {
        &self.ticker
    }

    /// Get the NIF of a stock.
    ///
    /// # Description
    ///
    /// Some countries add extra identity numbers to the companies, and these are useful for
    /// checking information related to the stock in national registries. As example, companies
    /// whose headquarters are registered in Spain, have an ID number called `NIF`. The property
    /// `extra_id` allows storing this information.
    ///
    /// ## Returns
    ///
    /// `None` when no special ID is linked to the stock. An ID otherwise.
    pub fn extra_id(&self) -> Option<&String> {
        self.nif.as_ref()
    }
}

impl fmt::Display for IbexCompany {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.ticker(), self.name())
    }
}

impl fmt::Debug for IbexCompany {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("")
            .field(&self.full_name())
            .field(&self.name())
            .field(&self.ticker())
            .field(&self.isin())
            .field(&self.extra_id())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use rstest::{fixture, rstest};

    // Fixture that builds a Company that belongs to the Ibex35 and is Spanish
    // (i.e. has NIF).
    #[fixture]
    fn spanish_company() -> IbexCompany {
        IbexCompany::new(
            Some("Banco Santander"),
            "SANTANDER",
            "SAN",
            "ES0113900J37",
            Some("A39000013"),
        )
    }

    // Fixture that builds a Company that belongs to the Ibex35 but it is not
    // registered in Spain (i.e. has no NIF).
    #[fixture]
    fn foreign_company() -> IbexCompany {
        IbexCompany::new(
            Some("Ferrovial S.E."),
            "FERROVIAL",
            "FER",
            "NL0015001FS8",
            None,
        )
    }

    #[rstest]
    fn build_ibex_company_1(spanish_company: IbexCompany) {
        println!("Test1 -- Test expects values for a Spanish company of the Ibex35");
        println!("Company -> {spanish_company}");
        assert_eq!("Banco Santander", spanish_company.full_name().unwrap());
        assert_eq!("SANTANDER", spanish_company.name());
        assert_eq!("ES0113900J37", spanish_company.isin());
        assert_eq!("A39000013", spanish_company.extra_id().unwrap());
    }

    #[rstest]
    fn build_ibex_company_2(foreign_company: IbexCompany) {
        println!("Test2 -- Test expects values for a non-Spanish company of the Ibex35");
        println!("Company -> {foreign_company}");
        assert_eq!(None, foreign_company.extra_id());
    }
}
