//! This crate is a simple library for [subspedia](https://www.subspedia.tv/) based on api the
//! provided by site

extern crate hyper;
extern crate hyper_tls;
extern crate tokio;
extern crate futures;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;

use futures::{Future, Stream};
use std::sync::{Arc, Mutex};
use std::borrow::Cow;

/// An enumeration of possible error which can occur during http requests and the parsing of json
/// returned by the api
#[derive(Debug)]
pub enum FetchError {
    Http(hyper::Error),
    Json(serde_json::Error),
}

impl From<hyper::Error> for FetchError {
    fn from(err: hyper::Error) -> FetchError {
        FetchError::Http(err)
    }
}

impl From<serde_json::Error> for FetchError {
    fn from(err: serde_json::Error) -> FetchError {
        FetchError::Json(err)
    }
}

/// Trait that requests have to implement.
pub trait Request {
    type Response: serde::de::DeserializeOwned + std::fmt::Debug + std::marker::Send;

    /// Return api url based on type of response that are you looking for.
    fn url(&self) -> Cow<'static, str>;
}

/// Struct for store the television series in translation.
#[derive(Deserialize, Debug)]
pub struct SerieTraduzione {
    id_serie: usize,
    nome_serie: String,
    link_serie: String,
    id_thetvdb: usize,
    num_stagione: usize,
    num_episodio: usize,
    stato: String,
}

/// Struct to perform the request for the series that are in translation.
pub struct ReqSerieTraduzione;

impl Request for ReqSerieTraduzione {
    type Response = SerieTraduzione;

    fn url(&self) -> Cow<'static, str> {
        Cow::Borrowed("https://www.subspedia.tv/API/serie_traduzione")
    }
}

/// Struct for store the television series available on site.
#[derive(Deserialize, Debug)]
pub struct Serie {
    id_serie: usize,
    nome_serie: String,
    link_serie: String,
    id_thetvdb: usize,
    stato: String,
    anno: usize
}

/// Struct to perform the request for the all series available on site.
pub struct ReqElencoSerie;

impl Request for ReqElencoSerie {
    type Response = Serie;

    fn url(&self) -> Cow<'static, str> {
        Cow::Borrowed("https://www.subspedia.tv/API/elenco_serie")
    }
}

/// Struct for store the subtitles.
#[derive(Deserialize, Debug)]
pub struct Sottotitolo {
    id_serie: usize,
    nome_serie: String,
    ep_titolo: String,
    num_stagione: usize,
    num_episodio: usize,
    immagine: String,
    link_sottotitoli: String,
    link_serie: String,
    link_file: String,
    descrizione: String,
    id_thetvdb: usize,
    data_uscita: String,
    grazie: usize
}

/// Struct to perform the request for the last subtitles translated.
pub struct ReqUltimiSottotitoli;

impl Request for ReqUltimiSottotitoli {
    type Response = Sottotitolo;

    fn url(&self) -> Cow<'static, str> {
        Cow::Borrowed("https://www.subspedia.tv/API/ultimi_sottotitoli")
    }
}

/// Struct to perform the subtitle request for a series.
pub struct ReqSottotitoliSerie {
    id: usize,
}

impl ReqSottotitoliSerie {
    /// Create a new ReqSottotitoliSerie with the given series id.
    pub fn new(id: usize) -> ReqSottotitoliSerie {
        ReqSottotitoliSerie {id}
    }
}

impl Request for ReqSottotitoliSerie {
    type Response = Sottotitolo;

    fn url(&self) -> Cow<'static, str> {
        Cow::Owned(format!("https://www.subspedia.tv/API/sottotitoli_serie?serie={}", self.id))
    }
}

///Makes a request based on given type
///
/// # Errors
///
/// Returns error if something gone wrong during http requests and parsing json.
///
/// # Exemple
///
///```
///extern crate subspedia;
///
///use subspedia::ReqSerieTraduzione;
///
///fn main() {
///    println!("{:#?}", subspedia::get(ReqSerieTraduzione).unwrap());
///}
/// ```
pub fn get<R: 'static + Request>(req: R) -> Result<Vec<R::Response>, FetchError>
{
    let url = req.url().parse().unwrap();
    let result = Arc::new(Mutex::new(Vec::new()));

    let tmp = Arc::clone(&result);

    tokio::run(futures::lazy(move || {
        fetch_json::<R::Response>(url)
            // use the parsed vector
            .map(move |mut serie| {
                tmp.lock().unwrap().append(&mut serie);
            })
            // if there was an error print it
            .map_err(|e| {
                match e {
                    FetchError::Http(e) => eprintln!("http error: {}", e),
                    FetchError::Json(e) => eprintln!("json parsing error: {}", e),
                }
            })
    }));

    Ok(Arc::try_unwrap(result).unwrap().into_inner().unwrap())
}

fn fetch_json<T>(url: hyper::Uri) -> impl Future<Item=Vec<T>, Error=FetchError>
    where T: serde::de::DeserializeOwned + std::fmt::Debug
{
    let https = hyper_tls::HttpsConnector::new(4).unwrap();
    let client = hyper::Client::builder()
        .build::<_, hyper::Body>(https);

    client
        // Fetch the url...
        .get(url)
        // And then, if we get a response back...
        .and_then(|res| {
            // asynchronously concatenate chunks of the body
            res.into_body().concat2()
        })
        .from_err::<FetchError>()
        // use the body after concatenation
        .and_then(|body| {
            // try to parse as json with serde_json
            let serie = serde_json::from_slice(&body)?;
            Ok(serie)
        })
}