use crate::markup;
use crate::missing::Missing;
use crate::repository::Repository;
use crate::user::User;
use crate::ContentType;

use maud::html;
use std::cmp::Ordering;
use std::{fs, path};

// TODO get rid of this i think
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

pub fn root() -> Result<ContentType, Missing> {
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
        Ok(ContentType::Markup(
            html!(
                h1.visuallyhidden {
                    "snoot forge repository list"
                }
                (markup::user_repos(&all_repos, &Page::Root))
            ),
            None,
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

pub fn user(name: &str) -> Result<ContentType, Missing> {
    let user = User::from_path(&get_user_root(name));
    let mut user = match user {
        Ok(user) => user,
        Err(error) => return Err(error),
    };
    sort_repos(&mut user.repos);
    Ok(ContentType::Markup(
        html! {
            (markup::user_header(&user, &Page::User))
            (markup::user_repos(&user.repos, &Page::User))
        },
        Some(user.name),
    ))
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
) -> Result<ContentType, Missing> {
    let repo = get_repo(name, project_name)?;
    let subpath = get_path(rest);
    let tree = match repo.tree(target, subpath.as_ref()) {
        Ok(tree) => tree,
        Err(Missing::Elsewhere(page)) => {
            return Err(Missing::Elsewhere(format!(
                "{}/{}/{}/{}",
                repo.url(),
                page,
                target.unwrap_or(""),
                rest.unwrap_or(&[]).join("/")
            )))
        }
        _ => return Err(Missing::Nowhere),
    };

    let readme = repo.readme(&tree);

    let title_prefix = match rest {
        Some(rest) => format!("{} -", rest.join("/")),
        None => "".to_string(),
    };

    let title = format!(
        "{} {}/{}@{}",
        title_prefix,
        name,
        project_name,
        target.unwrap_or(repo.head()?.name().unwrap_or("")),
    );

    Ok(ContentType::Markup(
        html! {
            (markup::project_header(&repo, &Page::Tree))
                (markup::tree(&tree, &Page::Tree))
                @if let Ok(readme) = readme {
                    (markup::readme(readme, &Page::Tree))
                }
        },
        Some(title),
    ))
}

pub fn project(name: &str, project_name: &str) -> Result<ContentType, Missing> {
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
) -> Result<ContentType, Missing> {
    let repo = get_repo(name, project_name)?;
    let log = get_log(&repo, target)?;
    let title = format!(
        "Log - {}/{} @ {}",
        name,
        project_name,
        target.unwrap_or(repo.head()?.name().unwrap_or(""))
    );
    Ok(ContentType::Markup(
        html! {
            (markup::project_header(&repo, &Page::Log))
            (markup::log(log, repo.url(), &Page::Log))
        },
        Some(title),
    ))
}

pub fn commit(
    name: &str,
    project_name: &str,
    target: Option<&str>,
    _rest: Option<&[&str]>,
) -> Result<ContentType, Missing> {
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

    let title = match this_commit {
        Some(commit) => format!(
            "{} - {}/{}@{}",
            commit.summary().unwrap_or("Commit"),
            name,
            project_name,
            target
        ),
        None => format!("Commit - {}/{}@{}", name, project_name, target),
    };

    Ok(ContentType::Markup(
        html! {
            (markup::project_header(&repo, &Page::Commit))
            (commit_markup)
        },
        Some(title),
    ))
}

fn get_blob(
    repo: &Repository,
    target: Option<&str>,
    rest: Option<&[&str]>,
) -> Result<Vec<u8>, Missing> {
    let subpath = get_path(rest);
    let tree = repo.tree(target, None)?;
    let blob = tree.get_blob(subpath.as_ref())?;
    Ok(blob)
}

pub fn blob(
    name: &str,
    project_name: &str,
    target: Option<&str>,
    rest: Option<&[&str]>,
) -> Result<ContentType, Missing> {
    let repo = get_repo(name, project_name)?;
    let tree = repo.tree(target, None)?;
    let tree_redirect = Err(Missing::Elsewhere(format!(
        "{}/tree/{}/{}",
        repo.url(),
        target.unwrap_or(""),
        rest.unwrap_or(&[]).join("/")
    )));
    let raw_url = format!(
        "{}/raw/{}/{}",
        repo.url(),
        target.unwrap_or(""),
        rest.unwrap_or(&[]).join("/")
    );
    let raw_redirect = Err(Missing::Elsewhere(raw_url.to_string()));
    let blob = match get_blob(&repo, target, rest) {
        Ok(blob) => match std::str::from_utf8(&blob) {
            Ok(blob) => blob.to_owned(),
            _ => return raw_redirect,
        },
        _ => return tree_redirect,
    };
    let subpath = get_path(rest).unwrap();
    let file_extension = subpath
        .extension()
        .unwrap_or(subpath.file_name().unwrap_or_default())
        .to_str();
    let directory = subpath.parent().unwrap().to_str();
    let directory_url = tree.url_for(directory);
    let file_name = rest.unwrap_or(&[]).last();
    let title = format!(
        "{} ({}) - {}/{}@{}",
        file_name.unwrap(),
        directory.unwrap(),
        name,
        project_name,
        target.unwrap_or(repo.head()?.name().unwrap_or(""))
    );
    Ok(ContentType::Markup(
        html! {
            (markup::project_header(&repo, &Page::Blob))
                article.blob {
                    (markup::blob_header(directory.unwrap(), &raw_url, &directory_url.unwrap(), file_name.unwrap()))
                    (markup::blob(file_extension.unwrap(), blob, &Page::Blob))
                }
        },
        Some(title),
    ))
}

pub fn refs(
    name: &str,
    project_name: &str,
    _target: Option<&str>,
    _rest: Option<&[&str]>,
) -> Result<ContentType, Missing> {
    let repo = get_repo(name, project_name)?;
    let refs = repo.refs()?;
    Ok(ContentType::Markup(
        html! {
            (markup::project_header(&repo, &Page::Refs))
            (markup::refs(refs, repo.url(), &Page::Refs))
        },
        Some(format!("Refs - {}/{}", name, project_name)),
    ))
}

pub fn raw(
    name: &str,
    project_name: &str,
    target: Option<&str>,
    rest: Option<&[&str]>,
) -> Result<ContentType, Missing> {
    let repo = get_repo(name, project_name)?;
    let blob = get_blob(&repo, target, rest)?.to_owned();
    let missing = Err(Missing::Nowhere);
    let file = match rest {
        Some(rest) => match rest.last() {
            Some(file) => file,
            None => return missing,
        },
        None => return missing,
    };
    let mime_type: mime::Mime = super::guess_mime(file);
    let mime = format!("{}", mime_type);
    Ok(ContentType::Binary(mime, blob))
}

// TODO the below should not be part of page

pub fn head(
    name: &str,
    project_name: &str,
    _target: Option<&str>,
    _rest: Option<&[&str]>,
) -> Result<ContentType, Missing> {
    let repo = get_repo(name, project_name)?;
    let head: git2::Reference = repo.head()?;
    if let Some(head) = head.name() {
        Ok(ContentType::PlainText(format!("ref: {}{}", head, "\n\n")))
    } else {
        Err(Missing::Nowhere)
    }
}

pub fn info(
    name: &str,
    project_name: &str,
    target: Option<&str>,
    _rest: Option<&[&str]>,
) -> Result<ContentType, Missing> {
    let target = target.unwrap_or("");
    if target != "refs" {
        return Err(Missing::Nowhere);
    }
    let repo = get_repo(name, project_name)?;
    let references = repo.references()?;
    let mut lines = vec![];
    for reference in references {
        if let Ok(reference) = reference {
            let id = reference.target().unwrap_or(git2::Oid::zero());

            lines.push(format!("{}\t{}", id, reference.name().unwrap_or("")))
        }
    }
    Ok(ContentType::PlainText(format!(
        "{}{}",
        lines.join("\n"),
        "\n"
    )))
}

pub fn pack_info(
    name: &str,
    project_name: &str,
    _target: Option<&str>,
    _rest: Option<&[&str]>,
) -> Result<ContentType, Missing> {
    let mut packs_path = get_user_root(name);
    packs_path.push(format!("{}.git", project_name));
    packs_path.push("objects");
    packs_path.push("packs");

    let packs = fs::read_dir(packs_path).expect("i couldn't find the objects/packs directory :(");
    let mut pack_list = String::new();
    for pack in packs {
        let pack: fs::DirEntry = pack.unwrap();
        let name = pack.file_name();
        let name = name.to_str().unwrap_or_default();
        if name.ends_with(".pack") {
            pack_list += &format!("P {}\n", name);
        }
    }
    Ok(ContentType::PlainText(format!("{}\n", pack_list)))
}

pub fn objects(
    name: &str,
    project_name: &str,
    target: Option<&str>,
    rest: Option<&[&str]>,
) -> Result<ContentType, Missing> {
    let user_root = get_user_root(name);
    let mut object_path = std::path::PathBuf::from(user_root);
    let folder = target.unwrap();
    let file = rest.unwrap_or(&[""])[0];
    if folder == "info" && file == "packs" {
        return pack_info(name, project_name, target, rest);
    }
    object_path.push(format!("{}.git", project_name));
    object_path.push("objects");
    object_path.push(folder);
    object_path.push(file);

    if let Ok(content) = std::fs::read(&object_path) {
        return Ok(ContentType::Binary("application/zlib".to_string(), content));
    }

    Err(Missing::Nowhere)
}
