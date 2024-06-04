// Copyright 2024 Felipe Torres González

use crate::finance::IbexCompany;
use std::fs::read_to_string;
use std::{collections::HashMap, fmt};
use toml::Table;
use tracing::{debug, info};

/// An implementation of the [Market][market] trait for the Ibex35 index.
///
/// The Ibex35 index includes the 35 values whose negotiation is the highest for all
/// the Spanish exchanges. This usually means that the 35 most capitalized companies
/// are included in this index, however this might be not true, as the index consider
/// several aspects to include/exclude companies from it.
///
/// This implementation is mainly a container for [IbexCompany][super::IbexCompany]
/// whose key trait is that these companies must be included in the Ibex35 index.
///
/// [market]: https://docs.rs/finance_api/0.1.0/finance_api/trait.Market.html
pub struct Ibex35Market {
    name: String,
    open_time: String,
    close_time: String,
    currency: String,
    company_map: HashMap<String, IbexCompany>,
}

/// The [Market] trait object only allows reading data once is built.
unsafe impl Sync for Ibex35Market {}
unsafe impl Send for Ibex35Market {}

impl Ibex35Market {
    /// Constructor of the [Ibex35Market] object.
    ///
    /// # Description
    ///
    /// The constructor shall receive a collection of companies that are part of
    /// the Ibex35 at the moment of the instantiation.
    ///
    /// Each entry of the collection is identified by the company's ticker and
    /// an object that implements the [Company] trait as value.
    ///
    /// The constructor has no logic to check whether the input companies are compliant
    /// with the invariant of the [Ibex35Market], this means that valid companies must
    /// be input at instantiation time, and external logic must ensure an instantiation
    /// of this object complies with the invariant (for example, if there's a change in
    /// the composition of the index).
    pub fn new(company_map: HashMap<String, IbexCompany>) -> Self {
        Ibex35Market {
            name: String::from("BME Ibex35 Index"),
            open_time: String::from("08:00:00"),
            close_time: String::from("16:30:00"),
            currency: String::from("euro"),
            company_map,
        }
    }

    /// Get the name of the Market, for example: _NASDAQ100_ or _IBEX35_.
    pub fn market_name(&self) -> &str {
        &self.name
    }

    /// Get a list of the stocks included in the market.
    ///
    /// # Description
    ///
    /// This method should build a list with the ticker identifier for each stock
    /// that is included in the market.
    ///
    /// ## Returns
    ///
    /// A vector with references to the tickers.
    pub fn list_tickers(&self) -> Vec<&String> {
        let mut tickers = Vec::new();
        self.company_map.keys().for_each(|c| tickers.push(c));

        tickers
    }

    /// Get a reference to a [Company] object included in the market.
    ///
    /// # Description
    ///
    /// This method searches for stocks identified by `name` in the market. The given
    /// name is applied in a regular expression. This means that if the `name` is too
    /// ambiguous, multiple stocks might match it. For example, if **Bank** is given as
    /// `name`, multiple stocks might match such string.
    ///
    /// ## Returns
    ///
    /// A wrapped vector with a list of references to stock descriptors (objects that
    /// implement the [Company] trait) that match `name`. `None` is returned when no
    /// stocks have been found matching `name` with their respective names.
    pub fn stock_by_name(&self, name: &str) -> Option<Vec<&IbexCompany>> {
        let mut stocks = Vec::new();

        for stock in self.company_map.values() {
            let stock_lowercase = stock.name().to_ascii_lowercase();
            if stock_lowercase.contains(&name.to_ascii_lowercase()) {
                stocks.push(stock);
            }
        }

        if !stocks.is_empty() {
            Some(stocks)
        } else {
            None
        }
    }

    /// Get a reference to a [Company] object included in the market.
    ///
    /// # Description
    ///
    /// This method searches for a stock whose ticker is equal to `ticker`. An
    /// exhaustive match is applied between `ticker` and the ticker of a Company.
    /// This means that partial tickers won't produce a match.
    ///
    /// ## Returns
    ///
    /// In contrast to the method [stock_by_name](Market::stock_by_name), this method will
    /// return a wrapped reference to an object that implements the `Company` trait
    /// whose ticker is equal to `ticker`, otherwise `None` will be returned.
    pub fn stock_by_ticker(&self, ticker: &str) -> Option<&IbexCompany> {
        if let Some(stock) = self.company_map.get(ticker) {
            Some(stock)
        } else {
            None
        }
    }

    /// Get the open time of the market (UTC).
    ///
    /// # Description
    ///
    /// Ibex35 opens at 8:00:00 GMT
    pub fn open_time(&self) -> &str {
        &self.open_time
    }

    /// Get the close time of the market (UTC).
    ///
    /// # Description
    ///
    /// Ibex35 closes at 16:30:00 GMT
    pub fn close_time(&self) -> &str {
        &self.close_time
    }

    /// Get the currency code (ISO 4217) of the market.
    ///
    /// # Description
    ///
    /// Ibex35's currency is Euro
    pub fn currency(&self) -> &str {
        &self.currency
    }

    /// Get a reference to a [Company] object included in the market.
    ///
    /// # Description
    ///
    /// This method searches for stocks identified by `name` in the market. The given
    /// name is applied in a regular expression. This means that if the `name` is too
    /// ambiguous, multiple stocks might match it. For example, if **Bank** is given as
    /// `name`, multiple stocks might match such string.
    ///
    /// ## Returns
    ///
    /// A wrapped vector with a list of references to stock descriptors (objects that
    /// implement the [Company] trait) that match `name`. `None` is returned when no
    /// stocks have been found matching `name` with their respective names.
    pub fn get_companies(&self) -> Vec<&IbexCompany> {
        self.company_map.values().collect()
    }
}

impl fmt::Display for Ibex35Market {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.market_name())
    }
}

impl fmt::Debug for Ibex35Market {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("")
            .field(&self.market_name())
            .field(&self.open_time())
            .field(&self.close_time())
            .field(&self.currency())
            .field(&self.get_companies())
            .finish()
    }
}

/// Helper function to build an [Ibex35Market] object from a file.
///
/// # Description
///
/// This function parses a TOML file with descriptors for companies, and builds
/// a HashMap with the tickers as keys, and [IbexCompany] as values. This collection
/// can be fed straight to [Ibex35Market::new].
///
/// An example of descriptor would be:
///
/// ```toml
/// [<BME TICKER>]
/// full_name = <Full name of the company (legal name)>
/// name = <Most used contraction of the name>
/// isin = <ISIN>
/// ticker = <BME TICKER>
/// extra_id = <NIF>
/// ```
///
/// ## Arguments
///
/// - _path_: a string that points to the TOML file.
///
/// ## Returns
///
/// An `enum` `Result<T, &str>` in which `T` implements the [Market] trait, and
/// the `str` indicates an error message.
pub fn load_ibex35_companies(path: &str) -> Result<Ibex35Market, &'static str> {
    info!("File {path} will be parsed to find stock descriptors.");

    let toml_parsed = match read_to_string(path) {
        Ok(data) => data,
        Err(_) => return Err("Error opening the input file"),
    };

    let table = match toml_parsed.parse::<Table>() {
        Ok(data) => data,
        Err(_) => return Err("Could not parse the file as a TOML table"),
    };

    let mut map: HashMap<String, IbexCompany> = HashMap::new();

    for key in table.keys() {
        debug!("Found company descriptor for {key}");
        let fname = table[key]["full_name"].as_str().unwrap();
        let sname = table[key]["full_name"].as_str().unwrap();
        let ticker = table[key]["ticker"].as_str().unwrap();
        let isin = table[key]["isin"].as_str().unwrap();
        let nif = table[key]["extra_id"].as_str().unwrap();

        let company = IbexCompany::new(Some(fname), sname, ticker, isin, Some(nif));

        map.insert(String::from(ticker), company);
    }

    Ok(Ibex35Market::new(map))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::{fixture, rstest};
    use std::collections::HashMap;

    #[fixture]
    fn ibex35_companies() -> HashMap<String, IbexCompany> {
        let mut companies = HashMap::<String, IbexCompany>::new();

        companies.insert(
            String::from("AENA"),
            IbexCompany::new(
                Some("AENA S.A."),
                "AENA",
                "AENA",
                "ES0105046009",
                Some("A86212420"),
            ),
        );

        companies.insert(
            String::from("AMS"),
            IbexCompany::new(
                Some("Amadeus IT Holding S.A."),
                "AMADEUS",
                "AMS",
                "ES0109067019",
                Some("A-84236934"),
            ),
        );

        companies.insert(
            String::from("CLNX"),
            IbexCompany::new(
                Some("Cellnex Telecom S.A."),
                "CELLNEX",
                "CLNX",
                "ES0105066007",
                Some("A64907306"),
            ),
        );

        companies
    }

    // Test case for the creation of a IbexMarket object.
    #[rstest]
    fn new(ibex35_companies: HashMap<String, IbexCompany>) {
        let market = Ibex35Market::new(ibex35_companies);

        assert_eq!(market.get_companies().len(), 3);
    }

    // Test case for the implementation of the Market trait.
    #[rstest]
    fn interface(ibex35_companies: HashMap<String, IbexCompany>) {
        let market = Ibex35Market::new(ibex35_companies);

        // Let's check that we get the same amount of companies using these methods:
        assert_eq!(market.get_companies().len(), market.list_tickers().len());
        // Check for the company search.
        assert!(market.stock_by_name("CELLNEX").is_some());
        assert!(market.stock_by_name("cell").is_some());
        assert!(market.stock_by_name("Grifols").is_none());
        // Check for companies by ticker.
        assert!(market.stock_by_ticker("SAN").is_none());
        assert!(market.stock_by_ticker("AENA").is_some());
        assert!(market.stock_by_ticker("CLNX").is_some());
    }
}
