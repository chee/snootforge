use crate::markup;
use crate::missing::Missing;
use crate::repository::Repository;
use crate::user::User;

use maud::{html, Markup};
use std::cmp::Ordering;
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

fn sort_repos(repos: &mut Vec<Repository>) {
    repos.sort_by(|a, b| {
        let a = a.last_update();
        let b = b.last_update();
        match (a, b) {
            (Ok(a), Ok(b)) => b.cmp(&a),
            (Err(_), Ok(_)) => Ordering::Greater,
            (Ok(_), Err(_)) => Ordering::Less,
            (Err(_), Err(_)) => Ordering::Equal,
        }
    })
}

pub fn root() -> Result<Markup, Missing> {
    let git_root = super::get_git_root();
    if git_root.is_dir() {
        let user_dirs = fs::read_dir(git_root).expect("git root read didn't work");
        let users = user_dirs.map(|user_dir| User::from_path(&user_dir.unwrap().path()));
        let mut all_repos = vec![];
        for user in users {
            if let Ok(mut user) = user {
                all_repos.append(&mut user.repos)
            }
        }
        sort_repos(&mut all_repos);
        Ok(html!(
            h1.visuallyhidden {
                "snoot forge repository list"
            }
            (markup::user_repos(&all_repos, &Page::Root))
        ))
    } else {
        Err(Missing::Nowhere)
    }
}

fn get_user_root(name: &str) -> path::PathBuf {
    let mut user_root = super::get_git_root();
    user_root.push(name);
    user_root
}

pub fn user(name: &str) -> Result<Markup, Missing> {
    let user = User::from_path(&get_user_root(name));
    let mut user = match user {
        Ok(user) => user,
        Err(error) => return Err(error),
    };
    sort_repos(&mut user.repos);
    Ok(html! {
        (markup::user_header(&user, &Page::User))
        (markup::user_repos(&user.repos, &Page::User))
    })
}

pub fn get_repo(name: &str, project_name: &str) -> Result<Repository, Missing> {
    let repo = Repository::open_user_project(&name, &project_name);
    match repo {
        Ok(repo) => Ok(repo),
        Err(_) => Err(Missing::Nowhere),
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
) -> Result<Markup, Missing> {
    let repo = get_repo(name, project_name)?;
    let subpath = get_path(rest);
    let tree = match repo.tree(target, subpath.as_ref()) {
        Ok(tree) => tree,
        Err(Missing::Elsewhere(page)) => {
            return Err(Missing::Elsewhere(format!(
                "/{}/{}/{}/{}/{}",
                name,
                project_name,
                page,
                target.unwrap_or(""),
                rest.unwrap_or(&[]).join("/")
            )))
        }
        _ => return Err(Missing::Nowhere),
    };

    let readme = repo.readme(&tree);

    Ok(html! {
       (markup::project_header(&repo, &Page::Tree))
       (markup::tree(&tree, &Page::Tree))
       @if let Ok(readme) = readme {
           (markup::readme(readme, &Page::Tree))
       }
    })
}

pub fn project(name: &str, project_name: &str) -> Result<Markup, Missing> {
    tree(name, project_name, None, None)
}

fn get_log<'a>(repo: &'a Repository, reff: Option<&str>) -> Result<Vec<git2::Commit<'a>>, Missing> {
    let log = repo.log(reff);
    match log {
        Ok(log) => Ok(log),
        Err(_) => Err(Missing::Nowhere),
    }
}

pub fn log(
    name: &str,
    project_name: &str,
    target: Option<&str>,
    _rest: Option<&[&str]>,
) -> Result<Markup, Missing> {
    let repo = get_repo(name, project_name)?;
    let log = get_log(&repo, target)?;
    Ok(html! {
        (markup::project_header(&repo, &Page::Log))
        (markup::log(log, repo.url(), &Page::Log))
    })
}

pub fn commit(
    name: &str,
    project_name: &str,
    target: Option<&str>,
    _rest: Option<&[&str]>,
) -> Result<Markup, Missing> {
    let repo = get_repo(name, project_name)?;
    let log = get_log(&repo, None)?;
    let target = match target {
        Some(t) => t,
        _ => return Err(Missing::Nowhere),
    };
    let mut this_commit: Option<&git2::Commit> = None;
    let mut prev_commit: Option<&git2::Commit> = None;

    for (index, c) in log.iter().enumerate() {
        if format!("{}", c.id()) == target {
            this_commit = Some(c);
            // this isn't really right at all
            prev_commit = log.get(index + 1).map(|t| t);
            break;
        }
    }

    let commit_markup = match (this_commit, prev_commit) {
        (Some(this), Some(prev)) => {
            let this_tree = this.tree().map(|t| Some(t)).unwrap_or(None);
            let that_tree = prev.tree().map(|t| Some(t)).unwrap_or(None);
            let diff = match repo.git2.diff_tree_to_tree(
                that_tree.as_ref(),
                this_tree.as_ref(),
                Some(&mut git2::DiffOptions::default()),
            ) {
                Ok(diff) => Some(diff),
                _ => None,
            };
            markup::commit(this, diff)
        }
        (Some(this), None) => {
            let this_tree = this.tree().map(|t| Some(t)).unwrap_or(None);
            let diff = match repo.git2.diff_tree_to_tree(
                None,
                this_tree.as_ref(),
                Some(&mut git2::DiffOptions::default()),
            ) {
                Ok(diff) => Some(diff),
                _ => None,
            };
            markup::commit(this, diff)
        }
        _ => return Err(Missing::Nowhere),
    };

    Ok(html! {
        (markup::project_header(&repo, &Page::Commit))
        (commit_markup)
    })
}

pub fn blob(
    name: &str,
    project_name: &str,
    target: Option<&str>,
    rest: Option<&[&str]>,
) -> Result<Markup, Missing> {
    let repo = get_repo(name, project_name)?;
    let subpath = get_path(rest);
    let tree = repo.tree(target, None)?;
    let blob = match tree.get_blob(subpath.as_ref()) {
        Ok(blob) => blob,
        _ => {
            return Err(Missing::Elsewhere(format!(
                "/{}/{}/tree/{}/{}",
                name,
                project_name,
                target.unwrap_or(""),
                rest.unwrap_or(&[]).join("/")
            )))
        }
    };
    let subpath = subpath.unwrap();
    let file_extension = subpath
        .extension()
        .unwrap_or(subpath.file_name().unwrap_or_default())
        .to_str();
    let directory = subpath.parent().unwrap().to_str();
    let directory_url = tree.url_for(directory);
    let file_name = rest.unwrap_or(&[]).last();
    Ok(html! {
        (markup::project_header(&repo, &Page::Blob))
        article.blob {
            (markup::blob_header(directory.unwrap(), &directory_url.unwrap(), file_name.unwrap()))
            (markup::blob(file_extension.unwrap(), blob, &Page::Blob))
        }
    })
}

pub fn refs(
    name: &str,
    project_name: &str,
    _target: Option<&str>,
    _rest: Option<&[&str]>,
) -> Result<Markup, Missing> {
    let repo = get_repo(name, project_name)?;
    let refs = repo.refs()?;
    Ok(html! {
        (markup::project_header(&repo, &Page::Refs))
        (markup::refs(refs, repo.url(), &Page::Refs))
    })
}
