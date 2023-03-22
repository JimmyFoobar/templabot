#[macro_use]
extern crate log;

use clap::Parser;
use dir_cmp::{full::compare_dirs, Filter, Options};
use git2::build::RepoBuilder;
use regex::Regex;
use serde_derive::Deserialize;
use std::{
    fs::{self, create_dir_all},
    io,
    path::{Path, PathBuf},
};
use toml;

pub mod utils;

// Top level struct to hold the TOML data.
#[derive(Deserialize)]
struct Data {
    entries: Vec<Entry>,
}

#[derive(Deserialize)]
struct Entry {
    template: String,
    repos: Vec<String>,
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Path to compare
    #[arg(long)]
    pat: Option<String>,
    /// Path to compare
    #[arg(long)]
    repo_list: PathBuf,
}

fn main() {
    env_logger::init();

    let cli = Cli::parse();

    let contents = fs::read_to_string(cli.repo_list).expect("reading file with repo list");
    let data: Data = toml::from_str(&contents).expect("serializing repo list");

    //TODO enable proxy use
    let mut repo_builder = RepoBuilder::new();

    //prepare compare options
    let regex = Regex::new(r"\.git$").unwrap();
    let filter = Filter::Exclude(vec![regex]);

    /* let diff_options = Options {
        ignore_left_only: false,
        ignore_right_only: true,
        filter: Some(filter),
        ignore_equal: true,
        recursive: true,
    }; */

    for entry in data.entries {
        let template_path = Path::new(&entry.template);
        debug!("template: {:?}", template_path);

        for repo in entry.repos {
            let url_with_pat = match &cli.pat {
                Some(pat) => add_pat(repo, pat),
                None => repo,
            };
            process_single_repo(&url_with_pat, template_path, &mut repo_builder);
        }
    }
}

fn process_single_repo(git_url: &str, template_path: &Path, repo_builder: &mut RepoBuilder) {
    let temp_repo_dir = tempfile::Builder::new()
        .prefix("repo_")
        .tempdir()
        .expect("creating temp dir");
    info!("cloning repo: {} to {:?}", git_url, temp_repo_dir.path());

    let repo = match repo_builder.clone(&git_url, temp_repo_dir.path()) {
        Ok(repo) => repo,
        Err(e) => panic!("failed to clone: {}", e),
    };

    //let compare_results = compare_dirs(template_path, temp_repo_dir.path(), diff_options.clone()).unwrap();
    //info!("compare_result: {:?}", compare_results);

    //create branch
    //repo.set_head("refs/head/my_branch").unwrap(); // TODO create dynamic name

    add_changed_files_from_template(template_path, temp_repo_dir.path());

    utils::add_commit_push(&repo, "refs/heads/my_branch", "commit message");
}

fn add_changed_files_from_template(template_path: &Path, repo_path: &Path) {
    let diff_options = Options {
        ignore_left_only: false,
        ignore_right_only: true,
        filter: None,
        ignore_equal: true,
        recursive: true,
    };

    let compare_results = compare_dirs(template_path, repo_path, diff_options.clone()).unwrap();
    info!("compare_result: {:?}", compare_results);

    for dir_cmp_entry in compare_results {
        match dir_cmp_entry {
            dir_cmp::full::DirCmpEntry::Left(path) => {
                let new_file = copy_with_parents(&path, template_path, repo_path);
                info!("added file: {:?}", new_file);
            }
            _ => continue,
        }
    }
}

#[cfg(test)]
mod test_add_changed_files_from_template {
    use super::*;
    use std::fs;

    fn init_logger() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[test]
    fn simple() {
        init_logger();
        //prepare left dir
        let template_dir = tempfile::Builder::new().tempdir().unwrap();
        let template_file = template_dir.path().join("sample.txt");
        fs::write(template_file.as_path(), b"Some text").unwrap();
        let repo_dir = tempfile::Builder::new().tempdir().unwrap();

        add_changed_files_from_template(template_dir.path(), repo_dir.path());

        assert_eq!(
            "Some text",
            fs::read_to_string(repo_dir.path().join("sample.txt")).unwrap()
        );
    }
}

//copy a file including it's parents to a target folder
fn copy_with_parents(file: &Path, base_dir: &Path, target_dir: &Path) -> PathBuf {
    // construct copy target
    let file_relatetive_to_base_dir = file.strip_prefix(base_dir).unwrap();

    let new_file = target_dir.join(file_relatetive_to_base_dir);
    _ = create_dir_all(new_file.parent().unwrap()).unwrap();
    _ = fs::copy(file, &new_file).unwrap();
    new_file
}

#[cfg(test)]
mod test_copy_with_parents {
    use super::*;
    use std::fs;

    fn init_logger() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[test]
    fn simple() {
        init_logger();
        //prepare left dir
        let from = tempfile::Builder::new().tempdir().unwrap();
        let from_file = from.path().join("sample.txt");
        fs::write(from_file.as_path(), b"Some text").unwrap();
        let to = tempfile::Builder::new().tempdir().unwrap();

        let new_file = copy_with_parents(&from_file, from.path(), to.path());

        assert_eq!("Some text", fs::read_to_string(new_file).unwrap());
    }

    #[test]
    fn sub_dir() {
        init_logger();
        //prepare left dir
        let from = tempfile::Builder::new().tempdir().unwrap();
        let from_sub = from.path().join("sub_dir");
        fs::create_dir(from_sub.as_path()).unwrap();
        let from_file = from_sub.as_path().join("sample.txt");
        fs::write(from_file.as_path(), b"Some text").unwrap();
        let to = tempfile::Builder::new().tempdir().unwrap();

        let new_file = copy_with_parents(&from_file, from.path(), to.path());

        assert_eq!("Some text", fs::read_to_string(new_file).unwrap());
    }
}
fn add_pat(repo_url: String, pat: &str) -> String {
    repo_url.replace(
        "https://github.com",
        &format!("https://{}:@github.com", pat),
    )
}

#[cfg(test)]
mod tests_add_pat {
    use super::*;

    #[test]
    fn foo() {
        assert_eq!(
            add_pat("https://github.com/foo/bar".to_string(), &"pat".to_string()),
            "https://pat:@github.com/foo/bar"
        );
    }
}
