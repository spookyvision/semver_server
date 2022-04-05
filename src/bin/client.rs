use std::{
    error::Error,
    fmt::Debug,
    io::{Read, Write},
    net::{Shutdown, TcpStream},
    thread,
};

use log::{debug, error, info};
use semver_repo::{
    api::{AddResult, ApiRequest, FindAllContainingResult, FindExactResult},
    CrateKind,
};
use semver_repo::{Metadata, SemVer};
use serde::de::DeserializeOwned;

trait ResponseHandler {
    fn handle(&self, serialized: &str);
}

impl ResponseHandler for ApiRequest {
    fn handle(&self, serialized: &str) {
        fn log_response(context: String, d: impl Debug) {
            info!("← {}: {:?}", context, d)
        }

        fn deserialize<T: DeserializeOwned>(serialized: &str) -> T {
            let res: T = serde_json::from_str(serialized)
                .expect(&format!("invalid response: {serialized:?}"));
            res
        }

        match self {
            ApiRequest::FindExact(query) => {
                let res: FindExactResult = deserialize(serialized);
                log_response(format!("find '{}'", query), res);
            }
            ApiRequest::FindAllContaining(query) => {
                let res: FindAllContainingResult = deserialize(serialized);
                log_response(format!("find all containing '{}'", query), res);
            }
            ApiRequest::AddCrate(m, _version) => {
                let res: AddResult = deserialize(serialized);
                log_response(format!("Add new crate '{}'", m.name()), res);
            }
            ApiRequest::AddRelease(name, version) => {
                let res: AddResult = deserialize(serialized);
                log_response(format!("Add version {} to crate '{}'", version, name), res);
            }
        }
    }
}

fn crate_data(name: impl AsRef<str>, major: u16) -> (Metadata, SemVer) {
    (
        Metadata::new(name, "Busy Person", CrateKind::Binary),
        SemVer::new(major, 0, 0),
    )
}
fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();
    let binary_name = "hello_bin".to_string();
    let (md, sv) = crate_data(&binary_name, 1);
    let (md2, sv2) = crate_data("hello_moon", 2);

    let requests = vec![
        ApiRequest::AddCrate(md, sv),
        ApiRequest::AddCrate(md2, sv2),
        ApiRequest::AddRelease("who?".to_string(), SemVer::new(1, 0, 0)),
        ApiRequest::AddRelease(binary_name.clone(), SemVer::new(1, 0, 4)),
        ApiRequest::AddRelease(binary_name.clone(), SemVer::new(1, 0, 1)),
        ApiRequest::AddRelease(binary_name.clone(), SemVer::new(1, 0, 5)),
        ApiRequest::FindExact(binary_name.clone()),
        ApiRequest::FindExact("stuxnet".to_string()),
        ApiRequest::FindAllContaining("moon".to_string()),
    ];

    let parallel = false;

    let mut threads = vec![];
    for request in requests {
        if parallel {
            threads.push(thread::spawn(|| match do_request(request) {
                Ok(_) => {}
                Err(e) => error!("{}", e),
            }));
        } else {
            do_request(request)?;
        }
    }

    for thread in threads {
        thread.join().unwrap();
    }

    Ok(())
}

fn do_request(request: ApiRequest) -> Result<(), Box<dyn Error>> {
    let request_json: String = serde_json::to_string(&request)?;
    debug!("→ {}", request_json);
    let port = std::env::var("REPO_PORT").unwrap_or("7878".to_string());
    let mut connection = TcpStream::connect(format!("127.0.0.1:{}", port))?;
    writeln!(connection, "{}", request_json)?;
    connection.shutdown(Shutdown::Write)?;
    let mut buffer = String::new();
    connection.read_to_string(&mut buffer)?;
    request.handle(buffer.as_str());
    Ok(())
}
