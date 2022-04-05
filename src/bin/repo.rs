use semver_repo::{CrateKind, Repository};
use semver_repo::{Metadata, SemVer};

fn main() -> anyhow::Result<()> {
    let store = option_env!("SEMVER_REPO").ok_or(anyhow::anyhow!(
        "missing SEMVER_REPO environment variable. Re-run with e.g.\n \
        SEMVER_REPO=/tmp/store.json cargo run"
    ))?;
    let mut repo = Repository::new(store);
    println!("repo: {repo:?}");

    println!("find crate: {:?}", repo.find_exact("linux.exe"));

    repo.add_crate(
        Metadata::new(
            "linux.exe".to_string(),
            "Linus Torvalds".to_string(),
            CrateKind::Binary,
        ),
        SemVer::new(1, 0, 0),
    )?;

    println!(
        "find crate, second attempt: {:?}",
        repo.find_exact("linux.exe")
    );

    Ok(())
}
