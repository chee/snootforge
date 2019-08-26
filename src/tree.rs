use crate::missing::Missing;
use crate::repository::Repository;
use chrono::prelude::*;
use std::cmp::Ordering;
use std::ops::Add;
use std::path;

#[derive(PartialEq)]
pub enum TreeEntryKind {
    Blob,
    Tree,
}

impl TreeEntryKind {
    fn url_prefix(&self) -> &str {
        match self {
            TreeEntryKind::Blob => "/blob",
            TreeEntryKind::Tree => "/tree",
        }
    }
}

impl Eq for TreeEntryKind {}

impl Ord for TreeEntryKind {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Blob, Self::Blob) => Ordering::Equal,
            (Self::Tree, Self::Tree) => Ordering::Equal,
            (Self::Tree, Self::Blob) => Ordering::Less,
            (Self::Blob, Self::Tree) => Ordering::Greater,
        }
    }
}

impl PartialOrd for TreeEntryKind {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub struct TreeEntry<'a> {
    pub name: String,
    pub kind: TreeEntryKind,
    pub url: Option<String>,
    blob: Option<git2::Blob<'a>>,
    tree: Option<git2::Tree<'a>>,
    last_commit: Option<git2::Commit<'a>>,
}

impl TreeEntry<'_> {
    pub fn from_blob<'a>(
        name: String,
        last_commit: Option<git2::Commit<'a>>,
        blob: git2::Blob<'a>,
    ) -> TreeEntry<'a> {
        TreeEntry {
            name,
            kind: TreeEntryKind::Blob,
            blob: Some(blob),
            tree: None,
            url: None,
            last_commit,
        }
    }

    pub fn from_tree<'a>(
        name: String,
        last_commit: Option<git2::Commit<'a>>,
        tree: git2::Tree<'a>,
    ) -> TreeEntry<'a> {
        TreeEntry {
            name,
            kind: TreeEntryKind::Tree,
            tree: Some(tree),
            blob: None,
            url: None,
            last_commit,
        }
    }

    pub fn content(&self) -> Result<&str, Missing> {
        match self.kind {
            TreeEntryKind::Blob => {
                let content = self.blob.as_ref().unwrap().content();
                match std::str::from_utf8(content) {
                    Ok(string) => Ok(string),
                    _ => Err(Missing::Nowhere),
                }
            }
            _ => Err(Missing::Nowhere),
        }
    }

    pub fn set_url(&mut self, url: String, refname: &str, subpath: Option<&path::PathBuf>) {
        let prefix = url.add(&self.kind.url_prefix()).add("/").add(refname);
        let path = match subpath {
            Some(subpath) => {
                let substr = subpath.to_str().unwrap();
                if substr.len() > 0 {
                    format!("/{}/", substr)
                } else {
                    "/".to_string()
                }
            }
            None => "/".to_string(),
        };
        let url: String = prefix.add(&path).add(&self.name);
        self.url = Some(url.to_owned());
    }

    pub fn last_summary(&self) -> Option<&str> {
        if let Some(commit) = &self.last_commit {
            commit.summary()
        } else {
            None
        }
    }

    pub fn last_update(&self) -> Option<DateTime<Utc>> {
        if let Some(commit) = &self.last_commit {
            Some(Utc.timestamp(commit.time().seconds(), 0))
        } else {
            None
        }
    }

    pub fn last_id(&self) -> Option<git2::Oid> {
        if let Some(commit) = &self.last_commit {
            Some(commit.id())
        } else {
            None
        }
    }
}

pub struct Tree<'a, 'b, 'c> {
    pub repo_url: String,
    pub subtree: bool,
    // tree: git2::Tree<'a>,
    repo: &'c Repository,
    pub entries: Vec<TreeEntry<'b>>,
    refname: String,
}

impl Tree<'_, '_, '_> {
    pub fn new<'a, 'b>(
        refname: &str,
        subpath: Option<&path::PathBuf>,
        tree: git2::Tree<'a>,
        repo: &'b Repository,
    ) -> Result<Tree<'a, 'b, 'b>, Missing> {
        let mut entries: Vec<TreeEntry> = vec![];
        let mut log = repo.log(Some(refname))?;
        log.reverse();
        for item in tree.iter() {
            let name = item.name().unwrap().to_owned();
            let mut tree_entry = match item.to_object(&repo.git2) {
                Ok(object) => {
                    let mut last_commit: Option<git2::Commit> = None;
                    for commit in log.iter() {
                        let commit: git2::Commit = commit.clone();
                        let mut file_path = match subpath {
                            Some(subpath) => path::PathBuf::from(subpath),
                            None => path::PathBuf::new(),
                        };
                        file_path.push(&name);
                        match commit.tree().unwrap().get_path(&file_path) {
                            Ok(blob) => {
                                if object.id() == blob.id() {
                                    last_commit = Some(commit);
                                    break;
                                }
                            }
                            _ => continue,
                        }
                    }
                    match object.kind() {
                        Some(git2::ObjectType::Tree) => {
                            let tree = object.into_tree().unwrap();
                            TreeEntry::from_tree(name, last_commit, tree)
                        }
                        Some(git2::ObjectType::Blob) => {
                            let blob = object.into_blob().unwrap();
                            TreeEntry::from_blob(name, last_commit, blob)
                        }
                        _ => continue,
                    }
                }
                _ => continue,
            };
            tree_entry.set_url(repo.url(), refname, subpath);
            entries.push(tree_entry);
        }
        let repo_url = repo.url();
        entries.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        entries.sort_by(|a, b| a.kind.cmp(&b.kind));
        Ok(Tree {
            repo,
            // tree,
            entries,
            repo_url,
            subtree: match subpath {
                Some(_) => true,
                None => false,
            },
            refname: refname.to_owned(),
        })
    }

    pub fn get_blob(&self, target: Option<&path::PathBuf>) -> Result<Vec<u8>, Missing> {
        if let Some(target_path) = target {
            if let Ok(path) = self.tree.get_path(target_path) {
                if let Ok(object) = path.to_object(&self.repo.git2) {
                    if let Ok(blob) = object.into_blob() {
                        return Ok(blob.content().to_vec());
                    }
                }
            }
        }
        Err(Missing::Nowhere)
    }

    pub fn url_for(&self, path: Option<&str>) -> Result<String, Missing> {
        let repo_url = self.repo.url();
        match path {
            Some(path) => Ok(repo_url
                .add("/tree/")
                .add(&self.refname)
                .add("/")
                .add(path)
                .to_owned()),
            None => Err(Missing::Nowhere),
        }
    }
}
