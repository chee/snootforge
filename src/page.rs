use crate::user::User;
use crate::markup;
use crate::repository::Repository;

use hyper::{StatusCode};
use std::{fs, path};
use maud::{html, Markup};

pub enum Page {
    Root,
    User,
    Readme,
    Log,
    Commit,
    Tree
}

pub fn root () -> Result<Markup, StatusCode> {
    let git_root = super::get_git_root();
    if git_root.is_dir() {
        let user_dirs = fs::read_dir(git_root).expect("git root read didn't work");
        let users = user_dirs.map(|user_dir| User::from_path(&user_dir.unwrap().path()));
        Ok(html!(
            h1.visuallyhidden {
                "snoot forge repository list"
            }
            @for user in users {
                 (markup::user_repos(&user.unwrap(), &Page::Root))
            }
        ))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

fn get_user_root (name: &str) -> path::PathBuf {
    let mut user_root = super::get_git_root();
    user_root.push(name);
    user_root
}

pub fn user (name: &str) -> Result<Markup, StatusCode> {
    let user = User::from_path(&get_user_root(name));
    if user.is_err() {
        return Err(user.unwrap_err());
    }
    let user = user.unwrap();
    let markup = markup::user_repos(&user, &Page::User);
    Ok(html!{
        h1.page-title {
            (name) "'s repos"
        }
        (markup)
    })
}

pub fn project (name: &str, project_name: &str) -> Result<Markup, StatusCode> {
    let repo = Repository::open_user_project(&name, &project_name);
    match repo {
        Ok(repo) => {
            Ok(html!{
                (markup::project_nav(&repo, &Page::Readme))
                (markup::readme(&repo.readme(), &Page::Readme))
            })
        },
        Err(_) => {
            Err(StatusCode::NOT_FOUND)
        }
    }
}
