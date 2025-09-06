use std::{fs, io, path::Path};

use anyhow::anyhow;
use git2::{ObjectType, Oid, Repository, StatusOptions, TreeWalkMode, TreeWalkResult};
use sha2::{Digest, Sha256};

pub fn is_git_repo_clean() -> Result<bool, git2::Error> {
    let repo = Repository::open(".")?;

    let mut opts = StatusOptions::new();
    opts.include_untracked(true).recurse_untracked_dirs(true);

    let statuses = repo.statuses(Some(&mut opts))?;

    Ok(statuses.is_empty())
}

pub fn read_committed_files_in_dir(commit: &str, dir: &str) -> anyhow::Result<Vec<String>> {
    let repo = Repository::open(".")?;

    let oid = Oid::from_str(commit)?;
    let com = repo.find_commit(oid)?;
    let tree = com.tree()?;

    let subtree = tree
        .get_name(dir)
        .ok_or_else(|| anyhow!("Directory '{}' not found in commit '{}'", dir, commit))?
        .to_object(&repo)?
        .peel_to_tree()?;

    let mut files = Vec::new();

    subtree.walk(TreeWalkMode::PreOrder, |_, entry| {
        if entry.kind() == Some(ObjectType::Blob)
            && entry.name().is_some_and(|name| name.ends_with(".toml"))
            && let Ok(blob) = repo.find_blob(entry.id())
            && let Ok(content) = str::from_utf8(blob.content())
        {
            files.push(content.to_string());
        }

        TreeWalkResult::Ok
    })?;

    Ok(files)
}

pub fn dir_hash(path: &Path) -> io::Result<String> {
    let mut entries: Vec<_> = fs::read_dir(path)?.filter_map(Result::ok).collect();

    entries.sort_by_key(|e| e.path());

    let mut hasher = Sha256::new();

    for entry in entries {
        let path = entry.path();

        if path.is_file() && path.extension().is_some_and(|ext| ext == "toml") {
            let content = fs::read(&path)?;
            hasher.update(&content);
        } else if path.is_dir() {
            let sub_hash = dir_hash(&path)?;
            hasher.update(sub_hash.as_bytes());
        }
    }

    Ok(format!("{:x}", hasher.finalize()))
}
