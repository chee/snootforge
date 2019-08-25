use crate::highlight;
use crate::page::Page;
use crate::repository::Repository;
use crate::tree::{Tree, TreeEntry, TreeEntryKind};
use crate::user::User;
use chrono::prelude::*;
use chrono_humanize::HumanTime;
use comrak::nodes::{AstNode, NodeHtmlBlock, NodeValue};
use comrak::{format_html, parse_document, ComrakOptions};
use maud::{html, Markup, DOCTYPE};
use std::ops::Add;
use typed_arena::Arena;

pub fn render(markup: Markup) -> String {
    format!("{}", markup.into_string())
}

pub fn head(title: &str) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en-ca";
        meta charset="utf-8";
        link rel="stylesheet" href="/normalize.css";
        link rel="stylesheet" href="/styles.css";
        link rel="stylesheet" href="/blob-theme.css";
        link rel="stylesheet" href="/custom.css";
        title {(title)}
        header.main-header {
            a.main-header__anchor href="/" {
                .main-header__logo {}
                .main-header__name {(title)}
            }
        }
    }
}

fn time(time: &DateTime<Utc>, class: &str) -> Markup {
    let datetime = time.format("%Y-%m-%dT%H:%MZ");
    let friendly_time = time.format("%c");
    let relative_time = HumanTime::from(*time);
    html! {
        time class=(class) datetime=(datetime) title=(friendly_time) {
            (relative_time)
        }
    }
}

pub fn repo_summary(repo: &Repository, page: &Page) -> Markup {
    html! {
        .repo-summary {
            .repo-summary__name-desc {
                h2.repo-summary-name {
                    @match page {
                        Page::Root => {
                            a.repo-summary__anchor.repo-summary-name__user href=(repo.user_url()) {
                                (repo.user_name)
                            }
                            span.repo-summary-name__slash {("/")}
                        },
                        _ => {}
                    }
                    a.repo-summary__anchor.repo-summary-name__repo href=(repo.url()) {
                        (repo.name)
                    }
                }
                 @if let Some(description) = &repo.description {
                    p.repo-summary-description {
                        (description)
                    }
                }
            }
            .repo-summary__last-update {
                @if let Ok(last_update) = &repo.last_update() {
                    "Updated "
                    (time(last_update, "repo-summary-date"))
                }
            }
        }
    }
}

pub fn user_header(user: &User, _page: &Page) -> Markup {
    html!(
        h1.user-name {
            a.user-name__anchor href=(user.url()) {
                (user.name)
            }
        }
    )
}

pub fn user_repos(user: &User, page: &Page) -> Markup {
    html! {
        section.repo-summary-collection {
            @for repo in &user.repos {
                (repo_summary(&repo, page))
            }
        }
    }
}

pub fn project_nav(repo: &Repository, page: &Page) -> Markup {
    let links = [
        ("tree", "/", Page::Tree),
        ("log", "/log", Page::Log),
        ("refs", "/refs", Page::Refs),
    ];
    let item_base_class = "project-nav-item".to_owned();
    let item_active_class = item_base_class
        .clone()
        .add(" ")
        .add(&item_base_class)
        .add("--active");

    html! {
        nav.project-nav {
          ul.project-nav-list {
                @for link in &links {
                    li class=(if link.2 == *page {&item_active_class} else {&item_base_class}) {
                        a.project-nav-item__anchor href=(repo.url().add(link.1)) {
                            (link.0)
                        }
                    }
                }
            }
        }
    }
}

pub fn project_header(repo: &Repository, page: &Page) -> Markup {
    html!(
        h1.project-name {
            a.project-name__user href=(repo.user_url()) {
                (repo.user_name)
            }
            "/"
            a.project-name__repo href=(repo.url()) {
                (repo.name)
            }
        }
        (project_nav(repo, page))
    )
}

pub fn blob_header(directory: &str, directory_url: &str, file_name: &str) -> Markup {
    let open_folder_icon = maud::PreEscaped("&#x1f4c2;");
    html! {
        header.blob-header {
            .tree-entry.tree-entry--parent {
                .tree-entry-icon aria-hidden=(true) {
                    (open_folder_icon)
                }
                h3.tree-entry-name.tree-entry-name--parent {
                    a.tree-entry-name__anchor href=(directory_url) {
                        ("..")
                    }
                }
            }
            h2.blob-header__heading {
                @if directory.len() > 0 {
                    span.blob-header__breadcrumb {
                        (directory)
                    }
                    "/"
                }
                span.blob-header__filename {
                    (file_name)
                }
            }
        }
    }
}

pub fn blob(token: &str, blob: String, _page: &Page) -> Markup {
    let blob = highlight::highlight(token, &blob);
    let lines = blob.lines();
    html! {
        pre.blob-content {
            @for line in lines {
                code.blob-content__line {
                    (maud::PreEscaped(line))
                    ("\n")
                }
            }
        }
    }
}

fn iter_nodes<'a, F>(node: &'a AstNode<'a>, f: &F)
where
    F: Fn(&'a AstNode<'a>),
{
    f(node);
    for c in node.children() {
        iter_nodes(c, f);
    }
}

pub fn readme(readme: &str, _page: &Page) -> Markup {
    let comrak_options = ComrakOptions {
        ext_autolink: true,
        ext_superscript: true,
        ext_header_ids: Some("".to_owned()),
        ext_footnotes: true,
        width: 80,
        smart: true,
        unsafe_: true,
        ..ComrakOptions::default()
    };
    let arena = Arena::new();
    let document = parse_document(&arena, readme, &comrak_options);

    iter_nodes(document, &|node| {
        let ref mut value = node.data.borrow_mut().value;
        let new_value = match value {
            &mut NodeValue::CodeBlock(ref code_block) => {
                let token = std::str::from_utf8(&code_block.info).unwrap_or("text");
                if token == "text" {
                    value.to_owned()
                } else {
                    let string = std::str::from_utf8(&code_block.literal)
                        .unwrap_or("codeblock was not valid utf-8");
                    let highlighted = format!(
                        "<pre lang={}><code>{}</pre></code>",
                        token,
                        highlight::highlight(token, string)
                    );
                    NodeValue::HtmlBlock(NodeHtmlBlock {
                        literal: highlighted.into_bytes(),
                        block_type: 0,
                    })
                }
            }
            _ => value.to_owned(),
        };

        *value = new_value;
    });
    let mut readme = vec![];
    format_html(document, &comrak_options, &mut readme).unwrap();
    let readme = std::str::from_utf8(&readme);

    // i'm not sanitising this, so feel free to bring your own scripts, styles
    // and tracking pixels
    html!(article.readme {
        (maud::PreEscaped(readme.unwrap_or("Readme was not valid utf-8")))
    })
}

fn tree_entry(entry: &TreeEntry, repo_url: &str) -> Markup {
    let closed_folder_icon = maud::PreEscaped("&#x1f4c1;");
    let file_icon = maud::PreEscaped("&#x1f4c4;");

    let entry_icon = match entry.kind {
        TreeEntryKind::Blob => file_icon,
        TreeEntryKind::Tree => closed_folder_icon,
    };

    let entry_href = match (&entry.kind, &entry.url) {
        (TreeEntryKind::Blob, Some(url)) => format!("{}", url),
        (TreeEntryKind::Tree, Some(url)) => format!("{}", url),
        _ => "".to_string(),
    };

    let name_base_class = "tree-entry-name".to_owned();
    let name_tree_class = name_base_class
        .clone()
        .add(" ")
        .add(&name_base_class)
        .add("--tree");

    let name_blob_class = name_base_class
        .clone()
        .add(" ")
        .add(&name_base_class)
        .add("--blob");

    let name_class = match entry.kind {
        TreeEntryKind::Blob => name_blob_class,
        TreeEntryKind::Tree => name_tree_class,
    };

    let last_summary = entry.last_summary().unwrap_or("");
    let last_update = entry.last_update().unwrap_or(Utc::now());

    let commit_href = match entry.last_id() {
        Some(id) => format!("{}/commit/{}", repo_url, id),
        _ => "".to_owned(),
    };

    html! {
        li.tree-entry {
            .tree-entry-icon aria-hidden=(true) {
                (entry_icon)
            }
            h3 class=(name_class) {
                a.tree-entry-name__anchor href=(entry_href) {
                    (entry.name)
                }
            }
            a.tree-entry-summary href=(commit_href) {
                (last_summary)
            }
            (time(&last_update, "tree-entry-date"))
        }
    }
}

pub fn tree(tree: &Tree, _page: &Page) -> Markup {
    let open_folder_icon = maud::PreEscaped("&#x1f4c2;");
    html! {
        ul.tree {
            @if tree.subtree {
                li.tree-entry.tree-entry--parent {
                    .tree-entry-icon aria-hidden=(true) {
                        (open_folder_icon)
                    }
                    h3.tree-entry-name.tree-entry-name--parent {
                        a.tree-entry-name__anchor href=("..") {
                            ("..")
                        }
                    }
                }
            }
            @for entry in &tree.entries {
                (tree_entry(&entry, &tree.repo_url))
            }
        }
    }
}

fn log_commit(commit: git2::Commit, repo_url: &str) -> Markup {
    let summary = commit.summary().unwrap_or("yeet");
    let committer = commit.committer();
    let author = commit.author();
    let id = commit.id();
    let commit_url = format!("{}/commit/{}", repo_url, id);
    let commit_short_id = commit.as_object().short_id().unwrap_or_default();
    let short_id = commit_short_id.as_str().unwrap_or_default();
    let committer_name = committer.name().unwrap_or("secret person");
    let committer_email = committer.email().unwrap_or("secret@person.club");
    let author_name = author.name().unwrap_or("secret person");
    let author_email = author.email().unwrap_or("secret@person.club");
    let author_matches_committer = author_name == committer_name && author_email == committer_email;
    let date = Utc.timestamp(commit.time().seconds(), 0);

    fn mailto(email: &str) -> String {
        format!("mailto:{}", email)
    }

    html! {
        li.log-commit {
            h3.log-commit__summary {
                a.log-commit__summary-anchor href=(commit_url) {
                    (summary)
                }
            }
            (time(&date, "log-commit__date"))
            a.commit-id.log-commit__id href=(commit_url) {
                (short_id)
            }
            span.log-commit__people {
                span.log-commit__by {
                    a.log-commit__person.log-commit__committer href=(mailto(committer_email)) {
                        (committer_name)
                    }
                }
                @if !author_matches_committer {
                    " & "
                        a.log-commit__person.log-commit-author href=(mailto(author_email)) {
                            (author_name)
                        }
                }
            }
        }
    }
}

pub fn log(log: Vec<git2::Commit>, repo_url: String, _page: &Page) -> Markup {
    html! {
        ol.log {
            @for commit in log {
                (log_commit(commit, &repo_url))
            }
        }
    }
}

fn refname(refname: &str, repo_url: &str) -> Markup {
    html! {
        li.refs-name {
            a.refs-name__anchor href=(format!("{}/tree/{}", repo_url, refname)) {
                (refname)
            }
        }
    }
}

pub fn refs(refs: (Vec<String>, Vec<String>), repo_url: String, _page: &Page) -> Markup {
    html! {
        section.refs {
            section.refs__tags {
                h1.refs-heading {
                    "tags"
                }
                ul.refs-list {
                    @for tag in refs.0 {
                        (refname(&tag, &repo_url))
                    }
                }
            }
            section.refs__branches {
                h1.refs-heading {
                    "branches"
                }
                ul.refs-list {
                    @for branch in refs.1 {
                        (refname(&branch, &repo_url))
                    }
                }
            }
        }
    }
}

pub fn diff_file(file: &str) -> Markup {
    let string = file.to_owned();
    let diff = highlight::highlight("diff", &string);
    html! {
        pre.commit-diff {
            (maud::PreEscaped(diff))
        }
    }
}

pub fn commit<'a>(commit: &git2::Commit, diff: Option<git2::Diff>) -> Markup {
    let mut files: Vec<String> = vec![];
    let mut current_diff: String = "".to_string();
    let mut last_file_id: git2::Oid = git2::Oid::zero();
    if let Some(diff) = &diff {
        diff.print(git2::DiffFormat::Patch, |delta, _hunk, line| {
            let new_file_id = delta.new_file().id();
            if new_file_id != last_file_id {
                files.push(current_diff.clone());
                current_diff.clear();
            }
            current_diff = format!("{}{}", current_diff, line.origin().to_string());
            current_diff = format!(
                "{}{}",
                current_diff,
                std::str::from_utf8(line.content()).unwrap_or("")
            );
            last_file_id = new_file_id;
            true
        })
        .unwrap_or_default();
    }

    html! {
        section.commit {
            pre.commit-message {
                (commit.message().unwrap_or(""))
            }
            .commit-id {
                (commit.id())
            }
            @if files.len() > 1 {
                @for file in &files[1..] {
                    (diff_file(&file))
                }
            }
            (diff_file(&current_diff))
        }
    }
}
