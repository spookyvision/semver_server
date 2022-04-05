use serde::{Deserialize, Serialize};

use crate::{Crate, Metadata, RepoError, SemVer};

#[derive(Debug, Serialize, Deserialize)]
pub enum ApiRequest {
    FindExact(String),
    FindAllContaining(String),
    AddCrate(Metadata, SemVer),
    AddRelease(String, SemVer),
}

use thiserror::Error;
#[derive(Error, Debug, Serialize, Deserialize)]
pub enum ApiError {
    #[error("internal")]
    Internal,
    #[error("{0:?}")]
    Repo(#[from] RepoError),
}

pub type ApiResult<T> = Result<T, ApiError>;
pub type AddResult = ApiResult<()>;
pub type FindExactResult = ApiResult<Option<Crate>>;
pub type FindAllContainingResult = ApiResult<Vec<Crate>>;
