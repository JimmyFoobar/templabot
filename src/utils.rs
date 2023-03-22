use git2::{Direction, Repository};

fn add_all(repo: &git2::Repository) {
    let mut index = repo.index().unwrap();
    index
        .add_all(["."], git2::IndexAddOption::DEFAULT, None)
        .unwrap();
    index.write().unwrap();
}

fn commit(repo: &git2::Repository, branch: &str, message: &str) {
    let mut index = repo.index().unwrap();
    let oid = index.write_tree().unwrap();
    let signature = repo.signature().unwrap();
    let parent_commit = repo.head().unwrap().peel_to_commit().unwrap();
    let tree = repo.find_tree(oid).unwrap();
    repo.commit(
        Some(branch),
        &signature,
        &signature,
        message,
        &tree,
        &[&parent_commit],
    )
    .unwrap();
}

fn push(repo: &Repository, branch: &str) -> Result<(), git2::Error> {
    let mut remote = repo.find_remote("origin")?;
    let mut cb = git2::RemoteCallbacks::new();
    cb.credentials(|_url, username_from_url, _allowed_types| {
        debug!("url: {:?}", _url);
        debug!("username_from_url: {:?}", username_from_url);
        git2::Cred::userpass_plaintext(username_from_url.unwrap(), "")
    });
    remote.connect_auth(Direction::Push, Some(cb), None)?;
    remote.push(&[format!("{}:{}", branch, branch)], None)
}

pub(crate) fn add_commit_push(repo: &git2::Repository, branch: &str, message: &str) {
    add_all(repo);
    commit(repo, branch, message);
    push(repo, branch).unwrap();
}
