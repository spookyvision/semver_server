use semver_repo::{CrateKind, Metadata, Repository, SemVer};

fn main() -> anyhow::Result<()> {
    let store = option_env!("SEMVER_REPO").ok_or(anyhow::anyhow!("missing SEMVER_REPO env var"))?;
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

fn loop_forever() -> ! {
    loop {}
}

fn never() {
    let yes_please = true;

    let nice_number = match yes_please {
        true => 13,
        false => return, // -> !
    };
}
