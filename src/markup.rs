use maud::{DOCTYPE, html, Markup};
use crate::repository::Repository;
use crate::page::Page;
use crate::user::User;
use std::ops::Add;
use comrak::{markdown_to_html, ComrakOptions};

pub fn render (markup: Markup) -> String {
    format!("{}", markup.into_string())
}

pub fn head (title: &str) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en-ca";
        meta charset="utf-8";
        link rel="stylesheet" href="/normalize.css";
        link rel="stylesheet" href="/styles.css";
        title {(title)}
        header.main-header {
            .main-header__logo {}
            .main-header__name {(title)}
        }
    }
}

pub fn repo_summary (repo: &Repository, page: &Page) -> Markup {
    html!{
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
                @if let Some(last_update) = &repo.last_update() {
                    "Updated "
                    time.repo-summary-date datetime=(last_update.format("%Y-%m-%dT%H:%MZ")) {
                        (last_update.format("%c"))
                    }
                }
            }
        }
    }
}

pub fn user_repos (user: &User, page: &Page) -> Markup {
    html! {
        section.repo-summary-collection {
            @for repo in &user.repos {
                (repo_summary(&repo, page))
            }
        }
    }
}

pub fn project_nav (repo: &Repository, page: &Page) -> Markup {
    let links = [
        ("readme", "/readme")
    ];
    html! {
        nav.project-nav {
            ul.project-nav-list {
                li.project-nav-item {
                    @for link in &links {
                        a.project-nav href=(repo.url().add(link.1)) {
                           (link.0)
                        }
                    }
                }
            }
        }
    }
}

pub fn readme (readme: &Option<String>, page: &Page) -> Markup {
    match readme {
        Some(readme) => {
            let comrak_options = ComrakOptions {
                ext_autolink: true,
                ext_superscript: true,
                ext_header_ids: Some("".to_owned()),
                ext_footnotes: true,
                width: 80,
                ..ComrakOptions::default()
            };
            let markdown = markdown_to_html(readme, &comrak_options);
            let rendered_readme = maud::PreEscaped(markdown);
            html!(article.readme{
                (rendered_readme)
            })
        }
        None => {
            html!{h4{"sorry! no readme"}}
        }
    }
}
