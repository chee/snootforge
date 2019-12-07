use crate::missing::Missing;
use crate::repository::Repository;
use std::{fs, path};

#[derive(Debug)]
pub struct User {
    pub name: String,
    pub repos: Vec<Repository>,
}

impl User {
    pub fn from_path(path: &path::PathBuf) -> Result<User, Missing> {
        if !path.is_dir() {
            return Err(Missing::Nowhere);
        }
        let dir = fs::read_dir(path);
        if dir.is_err() {
            return Err(Missing::Nowhere);
        }
        let name = path.file_name();
        if name.is_none() {
            return Err(Missing::Nowhere);
        }
        let name = name.unwrap().to_str();
        if name.is_none() {
            return Err(Missing::Nowhere);
        }
        let name = name.unwrap().to_owned();
        let dir = dir.unwrap();
        let mut repos: Vec<Repository> = vec![];
        for bare in dir {
            let bare: fs::DirEntry = bare.expect("item wasn't anything");
            let path = bare.path();
            let repo = Repository::open(&name, &path);
            if let Err(error) = repo {
                eprintln!("not including {:?} because {:?}", bare, error);
                continue;
            }
            repos.push(repo.unwrap());
        }
        Ok(User { name, repos })
    }

    pub fn url(&self) -> String {
        format!("/{}", self.name)
    }
}
