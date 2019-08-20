use hyper::StatusCode;
use std::{path, fs};
use crate::repository::Repository;

#[derive(Debug)]
pub struct User {
    pub name: String,
    pub repos: Vec<Repository>
}

impl User {
    pub fn from_path(path: &path::PathBuf) -> Result<User, StatusCode>{
        if !path.is_dir() {
            return Err(StatusCode::NOT_FOUND)
        }
        let dir = fs::read_dir(path);
        if dir.is_err() {
            return Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
        let name = path.file_name();
        if name.is_none() {
            return Err(StatusCode::NOT_FOUND)
        }
        let name = name.unwrap().to_str();
        if name.is_none() {
            return Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
        let name = name.unwrap().to_owned();
        let dir = dir.unwrap();
        let mut repos: Vec<Repository> = vec![];
        for bare in dir {
            let bare: fs::DirEntry = bare.expect("item wasn't anything");
            let path = bare.path();
            let repo = Repository::open(&name, &path);
            if let Err(error) = repo {
                println!("not including {:?} because {:?}", bare, error);
                continue;
            }
            repos.push(repo.unwrap());
        }
        Ok(User {
            name,
            repos
        })
    }

    pub fn url (&self) -> String {
        format!("/{}", self.name)
    }
}
