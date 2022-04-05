use std::io::BufReader;
use std::net::{TcpListener, TcpStream};
use std::{env, io::prelude::*};

use log::{debug, error};
use semver_repo::api::{ApiError, ApiResult, FindAllContainingResult};
use semver_repo::{
    api::{ApiRequest, FindExactResult},
    RepoError, Repository,
};
use serde::Serialize;
use thiserror::Error;

trait JsonResponse: Serialize {
    fn to_json(&self) -> String;
}

impl<T: Serialize> JsonResponse for Result<T, RepoError> {
    fn to_json(&self) -> String {
        // to avoid moving out of self, first convert to Result<&T, &E> using `as_ref()`
        // then we need to dereference the repo error again so the generated `From` impl
        // is available.
        self.as_ref().map_err(|e| ApiError::from(*e)).to_json()
    }
}

fn internal_error() -> String {
    let err: ApiResult<()> = Err(ApiError::Internal);
    serde_json::to_string(&err).unwrap()
}

impl<T: Serialize> JsonResponse for Result<T, ApiError> {
    fn to_json(&self) -> String {
        match serde_json::to_string(&self) {
            Ok(s) => s,

            // safe fallback
            Err(_) => internal_error(),
        }
    }
}

#[test]
fn ensure_safe_json() {
    todo!();
    // use std::collections::HashMap;
    // type DangerMap = HashMap<u32, u32>;
    // let mut danger: DangerMap = HashMap::default();
    // danger.insert(1, 2);
    // let fails: Result<_, ApiError> = Ok(danger);
    // let cmp: Result<DangerMap, ApiError> = Err(ApiError::Internal);
    // assert_eq!(fails.to_json(), cmp.to_json())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();
    let store = option_env!("REPO_STORE").ok_or(anyhow::anyhow!("missing REPO_STORE env var"))?;
    let mut repository = Repository::new(store);

    let port = env::var("REPO_PORT").unwrap_or("7878".to_string());
    let addr = format!("127.0.0.1:{}", port);
    log::info!("serving at {}", addr);
    let listener = TcpListener::bind(addr)?;

    for connection in listener.incoming() {
        let mut stream = match connection {
            Ok(stream) => stream,
            Err(e) => {
                error!("Connection error: {:?}", e);
                continue;
            }
        };

        let response = handle(&mut stream, &mut repository);
        debug!("sending response: {response}");
        match write!(stream, "{}", response) {
            Ok(_) => {}
            Err(e) => error!("error writing to stream: {:?}", e),
        }
    }

    Ok(())
}

#[derive(Error, Debug)]
enum ParseError {
    #[error("unreadable")]
    Unreadable,
    #[error("garbage: {0}")]
    Garbage(String),
}

fn parse_request(stream: &mut TcpStream) -> Result<ApiRequest, ParseError> {
    let mut buf = String::new();
    stream
        .read_to_string(&mut buf)
        .map_err(|_| ParseError::Unreadable)?;
    serde_json::from_str(&buf).map_err(|_| ParseError::Garbage(buf))
}

fn handle(stream: &mut TcpStream, repository: &mut Repository) -> String {
    let request = match parse_request(stream) {
        Ok(request) => request,
        Err(e) => {
            log::warn!("could not parse request - {}", e);
            return internal_error();
        }
    };

    match request {
        ApiRequest::FindExact(crate_name) => {
            let res: FindExactResult =
                Ok(repository.find_exact(&crate_name).map(|crt| crt.to_owned()));
            res.to_json()
        }
        ApiRequest::AddCrate(metadata, version) => {
            repository.add_crate(metadata, version).to_json()
        }
        ApiRequest::AddRelease(name, version) => repository.add_release(name, version).to_json(),
        ApiRequest::FindAllContaining(name) => {
            let res: FindAllContainingResult = Ok(repository
                .find_containing(name)
                .into_iter()
                .cloned()
                .collect());
            res.to_json()
        }
    }
}
