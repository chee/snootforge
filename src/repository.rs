use crate::user::User;

use chrono::prelude::*;
use std::{fmt, fs, io, path, str};

pub struct Repository {
    git2: git2::Repository,
    pub path: String,
    pub user_name: String,
    pub name: String,
    pub description: Option<String>,
}

impl fmt::Debug for Repository {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Repository {{name: {}, description: {}}}",
            self.name,
            self.description.as_ref().unwrap()
        )
    }
}

impl Repository {
    pub fn open(user_name: &str, repo_path: &path::PathBuf) -> Result<Repository, io::Error> {
        let repo = git2::Repository::open_bare(&repo_path);
        if let Err(_error) = repo {
            // yeet
            return Err(io::Error::last_os_error());
        }
        let repo = repo.unwrap();
        let name = repo_path.file_stem().unwrap().to_str().unwrap_or("no name").to_string();
        let description_path = repo_path.join("description");
        let description = match fs::read_to_string(description_path) {
            Ok(string) => Some(string),
            Err(_) => None,
        };
        Ok(Repository {
            user_name: user_name.to_owned(),
            git2: repo,
            name,
            description,
            path: repo_path.to_str().unwrap().to_owned()
        })
    }

    pub fn open_user_project (user_name: &str, project_name: &str) -> Result<Repository, io::Error> {
        let mut repo_path = super::get_git_root();
        repo_path.push(user_name);
        repo_path.push(format!("{}.git", project_name));
        Repository::open(user_name, &repo_path)
    }

    pub fn head(&self) -> Option<git2::Reference> {
        if let Ok(head) = self.git2.head() {
            Some(head)
        } else {
            None
        }
    }

    pub fn last_commit(&self) -> Option<git2::Commit> {
        self.head().and_then(|head| {
            if let Ok(commit) = head.peel_to_commit() {
                Some(commit)
            } else {
                None
            }
        })
    }

    pub fn last_update(&self) -> Option<DateTime<Utc>> {
        self.last_commit().and_then(|commit| {
            let seconds_since_epoch = commit.time().seconds();
            Some(Utc.timestamp(seconds_since_epoch, 0))
        })
    }

    pub fn user_url (&self) -> String {
        format!("/{}", self.user_name)
    }

    pub fn url (&self) -> String {
        format!("/{}/{}", self.user_name, self.name)
    }

    pub fn tree (&self) -> Result<git2::Tree, git2::Error> {
        self.head().unwrap().peel_to_tree()
    }

    pub fn find_file (&self, file: &str) -> Option<String> {
        if let Ok(tree) = self.tree() {
            if let Ok(entry) = tree.get_path(path::Path::new(file)) {
                if let Ok(object) = entry.to_object(&self.git2) {
                    if let Ok(blob) = object.peel_to_blob() {
                        if let Ok(string) = str::from_utf8(blob.content()) {
                            return Some(string.to_owned())
                        }
                    }
                }
            }
        }
        None
    }

    pub fn readme (&self) -> Option<String> {
        let readme_names = ["readme.md", "README.md", "readme", "README"];
        for readme_name in &readme_names {
            if let Some(readme) = self.find_file(readme_name) {
                return Some(readme)
            }
        }
        None
    }
}
