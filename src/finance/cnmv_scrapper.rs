//! cnmw_scrapper.rs
//!
//! Module that includes logic related to the extraction of data from the web page
//! of the Spanish _Comisión Nacional de Mercado de Valores (CNMV)_.

use crate::finance::IbexCompany;
use crate::finance::{AliveShortPositions, ShortPosition};
use date::Date;
use reqwest;
use scraper::{Html, Selector};
use tracing::{debug, trace};

/// `enum` to handle what endpoints of the CNMV's API are supported by this module.
enum EndpointSel {
    /// EP -> `Consultas a registros oficiales>Entidades emisoras: Información
    /// regulada>Posiciones cortas>Notificaciones de posiciones cortas`
    ShortEP,
}

/// Data type that checks whether a response for a short position request succeeded or not.
#[derive(Debug)]
pub struct ShortResponse(String);

impl ShortResponse {
    /// Use this method to check whether a response of the GET method returned valid
    /// content or not.
    pub fn parse(s: String) -> Result<Self, CNMVError> {
        match s.find("No se han encontrado datos disponibles") {
            Some(_) => match s.find("Serie histórica") {
                Some(_) => Ok(Self(s)),
                None => Err(CNMVError::UnknownCompany),
            },
            None => Ok(Self(s)),
        }
    }
}

impl AsRef<str> for ShortResponse {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Handler to extract data from the CNMV web page.
///
/// # Description
///
/// This object includes several methods that extract information from the CNMV's web
/// page. That page offers information a bit sparsely, and it's difficult to get all
/// the needed data with a few clicks.
///
/// The current list of supported features is:
/// - Extraction of the active short positions of a company (`Consultas a registros oficiales>Entidades emisoras: Información regulada>Posiciones cortas>Notificaciones de posiciones cortas`).
///
/// The endpoint of the web page expects a formal ID, thus using tickers or regular names
/// is not allowed. To avoid handling such type of information, this object works with
/// [IbexCompany] data objects. These are data objects that gather all the information
/// for a particular company.
///
/// The methods of this object are asynchronous which means they must be executed inside
/// a thread executer such as [Tokio](https://docs.rs/tokio/latest/tokio/).
pub struct CNMVProvider {
    /// The main path of the URL.
    base_url: String,
    /// Path extension for the _PosicionesCortas_ endpoint.
    short_ext: String,
}

impl Default for CNMVProvider {
    /// Default implementation delegates to [CNMVProvider::new].
    fn default() -> Self {
        Self::new()
    }
}

impl CNMVProvider {
    /// Class constructor.
    pub fn new() -> CNMVProvider {
        CNMVProvider {
            base_url: String::from("https://www.cnmv.es"),
            short_ext: String::from("Portal/Consultas/EE/PosicionesCortas.aspx?nif="),
        }
    }

    /// Internal method that executes a GET to the CNMV's web page endpoints.
    async fn collect_data(
        &self,
        endpoint: EndpointSel,
        stock_id: &str,
    ) -> Result<ShortResponse, CNMVError> {
        // Select the endpoint that shall be used for the requested GET.
        let endpoint = match endpoint {
            EndpointSel::ShortEP => &self.short_ext[..],
        };

        debug!("GET requested for the CNMV endpoint: {endpoint}");

        let resp = reqwest::get(format!("{}/{}{stock_id}", self.base_url, endpoint))
            .await
            .map_err(|e| CNMVError::ExternalError(e.to_string()))?;
        if resp.status().as_u16() != 200 {
            Err(CNMVError::ExternalError(resp.status().as_str().to_string()))
        } else {
            let response = ShortResponse::parse(
                resp.text()
                    .await
                    .map_err(|e| CNMVError::InternalError(e.to_string()))?,
            )?;
            trace!("Response: {:#?}", response);
            Ok(response)
        }
    }

    /// Method that checks alive short positions of a stock.
    ///
    /// # Description
    ///
    /// This method checks CNMV's web page to retrieve if a stock has open short positions
    /// against it. Only alive positions are retrieved. The information is encapsulated
    /// in a [AliveShortPositions] struct.
    ///
    /// ## Arguments
    ///
    /// - _stock_: An instance of an [IbexCompany].
    ///
    /// ## Returns
    ///
    /// The method returns a `Result` enum that indicates whether there was an issue checking
    /// the web page. Regardless of the amount of short positions, the result will be `Ok` if
    /// the request to the web page was successful. Open positions are included in the
    /// [positions](AliveShortPositions::positions) field of the struct. If there is no open
    /// position at the moment of checking, an empty collection is included.
    pub async fn short_positions(
        &self,
        stock: &IbexCompany,
    ) -> Result<AliveShortPositions, CNMVError> {
        let id = match stock.extra_id() {
            Some(id) => id,
            None => return Err(CNMVError::UnknownCompany),
        };

        let raw_data = self.collect_data(EndpointSel::ShortEP, id).await?;

        let document = Html::parse_document(raw_data.as_ref());
        let selector_td = Selector::parse("td").unwrap();
        let selector_tr = Selector::parse("tr").unwrap();

        let mut positions = Vec::new();

        for element_tr in document.select(&selector_tr) {
            let mut owner: String = String::from("dummy");
            let mut weight: f32 = 0.0;
            let mut date: String = String::from("nodate");
            for td in element_tr.select(&selector_td) {
                if let Some(x) = td.attr("class") {
                    if x == "Izquierda" {
                        owner = String::from(td.text().next().unwrap().trim());
                    }
                } else if let Some(x) = td.attr("data-th") {
                    if x == "% sobre el capital" {
                        weight = td
                            .text()
                            .next()
                            .unwrap()
                            .replace(',', ".")
                            .parse::<f32>()
                            .unwrap();
                    } else if x == "Fecha de la posición" {
                        date = String::from(td.text().next().unwrap());
                    }
                }
            }
            if &owner[..] != "dummy" {
                positions.push(ShortPosition {
                    owner,
                    weight,
                    date,
                });
            }
        }

        let mut total = 0.0;
        positions
            .iter()
            .for_each(|position| total += position.weight);
        let date = Date::today_utc();

        Ok(AliveShortPositions {
            total,
            positions,
            date,
        })
    }
}

/// Error types for the CNMV handler.
#[derive(Debug)]
pub enum CNMVError {
    /// Error given when the passed company is not recognized by the CNMV' API.
    UnknownCompany,
    /// Error from the external API (CNMV).
    ExternalError(String),
    /// Error for the internal methods.
    InternalError(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::finance::IbexCompany;
    use rstest::{fixture, rstest};

    #[fixture]
    fn a_company() -> IbexCompany {
        IbexCompany::new(
            Some("Grifols"),
            "GRIFOLS",
            "GRF",
            "ES0171996087",
            Some("A-58389123"),
        )
    }

    #[fixture]
    fn not_a_company() -> IbexCompany {
        IbexCompany::new(
            Some("Not A Company"),
            "NoCompany",
            "NOC",
            "0",
            Some("A44901010"),
        )
    }

    #[rstest]
    fn collect_data_existing_company(a_company: IbexCompany) {
        // Prepare the test
        let provider = CNMVProvider::new();

        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                // Send a request to the external API
                let raw_content = provider
                    .collect_data(EndpointSel::ShortEP, a_company.extra_id().unwrap())
                    .await;
                assert!(raw_content.is_ok());
            })
    }

    #[rstest]
    fn collect_data_non_existing_company(not_a_company: IbexCompany) {
        // Prepare the test
        let provider = CNMVProvider::new();

        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                // Send a request to the external API
                let raw_content = provider
                    .collect_data(EndpointSel::ShortEP, not_a_company.extra_id().unwrap())
                    .await;

                assert!(raw_content.is_err());
            })
    }

    #[rstest]
    fn short_position_valid_company(a_company: IbexCompany) {
        // Prepare the test
        let provider = CNMVProvider::new();

        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                // Send a request to the external API
                let short_position = provider.short_positions(&a_company).await;
                assert!(short_position.is_ok());
                println!(
                    "Short position of {}:{:#?}",
                    a_company,
                    short_position.unwrap()
                );
            })
    }

    #[rstest]
    fn short_position_non_valid_company(not_a_company: IbexCompany) {
        // Prepare the test
        let provider = CNMVProvider::new();

        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                // Send a request to the external API
                let short_position = provider.short_positions(&not_a_company).await;
                assert!(short_position.is_err());
            })
    }
}
