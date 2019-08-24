use crate::markup;
use crate::repository::Repository;
use crate::user::User;

use hyper::StatusCode;
use maud::{html, Markup};
use std::{fs, path};

#[derive(PartialEq, Debug)]
pub enum Page {
    Root,
    User,
    Tree,
    Log,
    Blob,
    Commit,
    Refs,
}

pub fn root() -> Result<Markup, StatusCode> {
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

fn get_user_root(name: &str) -> path::PathBuf {
    let mut user_root = super::get_git_root();
    user_root.push(name);
    user_root
}

pub fn user(name: &str) -> Result<Markup, StatusCode> {
    let user = User::from_path(&get_user_root(name));
    if user.is_err() {
        return Err(user.unwrap_err());
    }
    let user = user.unwrap();
    Ok(html! {
        (markup::user_header(&user, &Page::User))
        (markup::user_repos(&user, &Page::User))
    })
}

pub fn get_repo(name: &str, project_name: &str) -> Result<Repository, StatusCode> {
    let repo = Repository::open_user_project(&name, &project_name);
    match repo {
        Ok(repo) => Ok(repo),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

fn get_path(rest: Option<&[&str]>) -> Option<path::PathBuf> {
    match rest {
        Some(rest) => Some(path::PathBuf::from(rest.join("/"))),
        _ => None,
    }
}

pub fn tree(
    name: &str,
    project_name: &str,
    target: Option<&str>,
    rest: Option<&[&str]>,
) -> Result<Markup, StatusCode> {
    let repo = get_repo(name, project_name)?;
    let subpath = get_path(rest);
    let tree = repo.tree(target, subpath.as_ref())?;
    let readme = repo.readme(&tree);

    Ok(html! {
       (markup::project_header(&repo, &Page::Tree))
       (markup::tree(&tree, &Page::Tree))
       @if let Ok(readme) = readme {
           (markup::readme(readme, &Page::Tree))
       }
    })
}

pub fn project(name: &str, project_name: &str) -> Result<Markup, StatusCode> {
    tree(name, project_name, None, None)
}

fn get_log<'a>(
    repo: &'a Repository,
    reff: Option<&str>,
) -> Result<Vec<git2::Commit<'a>>, StatusCode> {
    let log = repo.log(reff);
    match log {
        Ok(log) => Ok(log),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

pub fn log(
    name: &str,
    project_name: &str,
    target: Option<&str>,
    _rest: Option<&[&str]>,
) -> Result<Markup, StatusCode> {
    let repo = get_repo(name, project_name)?;
    let log = get_log(&repo, target)?;
    Ok(html! {
        (markup::project_header(&repo, &Page::Log))
        (markup::log(log, &Page::Log))
    })
}

fn _get_commit<'a>(_repo: &'a Repository, _reff: &str) -> Result<git2::Commit<'a>, StatusCode> {
    Err(StatusCode::NOT_IMPLEMENTED)
}

pub fn commit(
    _name: &str,
    _project_name: &str,
    _target: Option<&str>,
    _rest: Option<&[&str]>,
) -> Result<Markup, StatusCode> {
    Err(StatusCode::NOT_IMPLEMENTED)
}

pub fn blob(
    name: &str,
    project_name: &str,
    target: Option<&str>,
    rest: Option<&[&str]>,
) -> Result<Markup, StatusCode> {
    let repo = get_repo(name, project_name)?;
    let subpath = get_path(rest);
    let tree = repo.tree(target, None)?;
    let blob = tree.get_blob(subpath.as_ref())?;
    let subpath = subpath.unwrap();
    let directory = subpath.parent().unwrap().to_str();
    let directory_url = tree.url_for(directory);
    let file_name = rest.unwrap().last();
    Ok(html! {
        (markup::project_header(&repo, &Page::Log))
        article.blob {
            (markup::blob_header(directory.unwrap(), &directory_url.unwrap(), file_name.unwrap()))
            (markup::blob(blob, &Page::Blob))
        }
    })
}

pub fn branches(
    _name: &str,
    _project_name: &str,
    _target: Option<&str>,
    _rest: Option<&[&str]>,
) -> Result<Markup, StatusCode> {
    Err(StatusCode::NOT_IMPLEMENTED)
}
