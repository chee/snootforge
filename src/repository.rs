use crate::missing::Missing;
use crate::tree::{Tree, TreeEntry, TreeEntryKind};
use chrono::prelude::*;
use std::{fmt, fs, io, path, str};

pub struct Repository {
    // TODO make this unnesc
    pub git2: git2::Repository,
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
        let name = repo_path
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap_or("no name")
            .to_string();
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
            path: repo_path.to_str().unwrap().to_owned(),
        })
    }

    pub fn open_user_project(user_name: &str, project_name: &str) -> Result<Repository, Missing> {
        let mut repo_path = super::get_git_root();
        repo_path.push(user_name);
        repo_path.push(format!("{}.git", project_name));
        match Repository::open(user_name, &repo_path) {
            Ok(repo) => Ok(repo),
            Err(_) => Err(Missing::Nowhere),
        }
    }

    pub fn head(&self) -> Result<git2::Reference, Missing> {
        match self.git2.head() {
            Ok(head) => Ok(head),
            Err(_) => Err(Missing::Nowhere),
        }
    }

    pub fn last_commit(&self) -> Result<git2::Commit, Missing> {
        match self.head()?.peel_to_commit() {
            Ok(commit) => Ok(commit),
            Err(_) => Err(Missing::Nowhere),
        }
    }

    pub fn last_update(&self) -> Result<DateTime<Utc>, Missing> {
        let commit = self.last_commit()?;
        let seconds_since_epoch = commit.time().seconds();
        Ok(Utc.timestamp(seconds_since_epoch, 0))
    }

    pub fn user_url(&self) -> String {
        format!("/{}", self.user_name)
    }

    pub fn url(&self) -> String {
        format!("/{}/{}", self.user_name, self.name)
    }

    pub fn log(&self, refname: Option<&str>) -> Result<Vec<git2::Commit>, Missing> {
        let refname = self.get_refname(refname)?;
        let reff = self.get_ref(&refname)?;
        let mut walk = match self.git2.revwalk() {
            Ok(walk) => walk,
            Err(_) => return Err(Missing::Nowhere),
        };
        let head_commit = match reff.peel_to_commit() {
            Ok(commit) => commit,
            Err(_) => return Err(Missing::Nowhere),
        };
        walk.push(head_commit.id()).unwrap_or_default();
        walk.set_sorting(git2::Sort::TOPOLOGICAL);
        let mut commits = vec![];
        for commit in walk {
            let commit = match commit {
                Ok(commit) => commit,
                Err(_) => return Err(Missing::Nowhere),
            };
            let commit = match self.git2.find_commit(commit) {
                Ok(commit) => commit,
                Err(_) => return Err(Missing::Nowhere),
            };
            commits.push(commit);
        }
        Ok(commits)
    }

    fn get_refname(&self, refname: Option<&str>) -> Result<String, Missing> {
        let head = self.head()?;
        let shorthead = head.shorthand().unwrap_or_default();
        Ok(refname.unwrap_or(shorthead).to_owned())
    }

    fn get_ref(&self, refname: &str) -> Result<git2::Reference, Missing> {
        match self.git2.resolve_reference_from_short_name(refname) {
            Ok(reff) => Ok(reff),
            Err(_) => return Err(Missing::Nowhere),
        }
    }

    pub fn tree(
        &self,
        refname: Option<&str>,
        subpath: Option<&path::PathBuf>,
    ) -> Result<crate::tree::Tree, Missing> {
        let refname = self.get_refname(refname)?;
        let reff = self.get_ref(&refname)?;
        let tree = match reff.peel_to_tree() {
            Ok(tree) => tree,
            Err(_) => return Err(Missing::Nowhere),
        };

        let tree = match subpath {
            Some(subpath) => {
                if subpath == &path::PathBuf::new() {
                    tree
                } else {
                    match tree.get_path(subpath) {
                        Ok(entry) => match entry.to_object(&self.git2) {
                            Ok(object) => match object.kind() {
                                Some(git2::ObjectType::Tree) => object.into_tree().unwrap(),
                                Some(git2::ObjectType::Blob) => {
                                    return Err(Missing::Elsewhere("blob".to_string()))
                                }
                                _ => return Err(Missing::Nowhere),
                            },
                            Err(_) => return Err(Missing::Nowhere),
                        },
                        Err(_) => return Err(Missing::Nowhere),
                    }
                }
            }
            None => tree,
        };

        Tree::new(&refname, subpath, tree, &self)
    }

    pub fn readme<'a>(&'a self, tree: &'a Tree) -> Result<&'a str, Missing> {
        let readme_names = [
            "readme.md".to_string(),
            "README.md".to_string(),
            "readme".to_string(),
            "README".to_string(),
        ];
        for entry in tree.entries.iter() {
            let entry: &TreeEntry = entry;
            let name = &entry.name;
            if readme_names.contains(&name) && entry.kind == TreeEntryKind::Blob {
                return entry.content();
            }
        }
        Err(Missing::Nowhere)
    }
}
