#![feature(proc_macro_hygiene)]
extern crate chrono_humanize;
extern crate futures;
extern crate git2;
extern crate hyper;
extern crate hyperlocal;
#[macro_use]
extern crate lazy_static;
extern crate comrak;
extern crate maud;
extern crate mime;
extern crate syntect;
extern crate typed_arena;

use crate::futures::Future;
use hyper::service::service_fn;
use hyper::{header, Body, Request, Response, StatusCode};
use maud::{html, Markup};
use missing::Missing;
use std::{env, fs, io, path};

mod highlight;
mod markup;
mod missing;
mod page;
mod repository;
mod tree;
mod user;

pub enum ContentType {
    // content_type, data
    Binary(String, Vec<u8>),
    // markup, optional title
    Markup(Markup, Option<String>),
    PlainText(String),
}

fn respond(content: Result<ContentType, Missing>) -> Response<Body> {
    match content {
        Ok(ContentType::Markup(markup, title)) => {
            let body = markup::render(html! {
                (markup::head(title))
                main {
                    (markup)
                }
                (markup::foot())
            });
            Response::builder()
                .header(header::CONTENT_LENGTH, body.len() as u64)
                .header(header::CONTENT_TYPE, "text/html")
                .body(Body::from(body))
                .expect("Failed to construct the response")
        }
        Ok(ContentType::Binary(content_type, body)) => {
            let response = Response::builder()
                .header(header::CONTENT_LENGTH, body.len() as u64)
                .header(header::CONTENT_TYPE, content_type)
                .body(Body::from(body))
                .expect("Failed to construct the response");
            response
        }
        Ok(ContentType::PlainText(body)) => {
            let response = Response::builder()
                .header(header::CONTENT_LENGTH, body.len() as u64)
                .header(header::CONTENT_TYPE, "text/plain")
                .body(Body::from(body))
                .expect("Failed to construct the response");
            response
        }
        Err(missing) => {
            let response = match missing {
                // Missing::Sometime => Response::builder()
                //     .status(StatusCode::NOT_IMPLEMENTED)
                //     .body(Body::from("sorry"))
                //     .expect("failed"),
                Missing::Nowhere => Response::builder()
                    .status(StatusCode::PAYMENT_REQUIRED)
                    .body(Body::from("sorry"))
                    .expect("failed"),
                Missing::Elsewhere(location) => Response::builder()
                    .header(header::LOCATION, location)
                    .status(StatusCode::FOUND)
                    .body(Body::from("sorry"))
                    .expect("failed to redirect"),
            };
            response
        }
    }
}
fn get_git_root() -> path::PathBuf {
    let args = get_args();
    let mut pathbuf = path::PathBuf::new();
    pathbuf.push(args.get(1).expect("Usage: yeet <root_path>"));
    pathbuf
}

pub fn guess_mime(file_name: &str) -> mime::Mime {
    let file_path = path::PathBuf::from(file_name);
    match file_path.extension() {
        Some(extension) => match extension.to_str() {
            Some("png") => mime::IMAGE_PNG,
            Some("jpg") => mime::IMAGE_JPEG,
            Some("jpeg") => mime::IMAGE_JPEG,
            Some("gif") => mime::IMAGE_GIF,
            Some("svg") => mime::IMAGE_SVG,
            Some("css") => mime::TEXT_CSS,
            Some("json") => mime::APPLICATION_JSON,
            Some("js") => mime::APPLICATION_JAVASCRIPT,
            _ => mime::TEXT_PLAIN,
        },
        None => mime::TEXT_PLAIN,
    }
}

fn make_file_response(file_name: &str, body: Vec<u8>) -> Response<Body> {
    let file_mime = guess_mime(file_name).to_string();

    respond(Ok(ContentType::Binary(file_mime, body)))
}

fn check_static_exists(file_name: &str) -> Option<path::PathBuf> {
    let file_path = path::PathBuf::from(format!("static/{}", file_name));
    if file_path.exists() {
        Some(file_path)
    } else {
        None
    }
}

fn read_or_bytes(file_name: &str, bytes: Vec<u8>) -> Vec<u8> {
    if let Some(file_path) = check_static_exists(file_name) {
        match fs::read(file_path) {
            Ok(file) => file,
            _ => bytes,
        }
    } else {
        bytes
    }
}

fn get_static_file(file_name: &str) -> Option<Response<Body>> {
    let file = match file_name {
        "normalize.css" => Some(read_or_bytes(
            file_name,
            include_bytes!("../static/normalize.css").to_vec(),
        )),
        "styles.css" => Some(read_or_bytes(
            file_name,
            include_bytes!("../static/styles.css").to_vec(),
        )),
        "logo.svg" => Some(read_or_bytes(
            file_name,
            include_bytes!("../static/logo.svg").to_vec(),
        )),
        "favicon.ico" => Some(read_or_bytes(
            file_name,
            include_bytes!("../static/favicon.ico").to_vec(),
        )),
        "javascript.js" => Some(read_or_bytes(
            file_name,
            include_bytes!("../static/javascript.js").to_vec(),
        )),
        "custom.css" | "custom.js" => Some(read_or_bytes(file_name, vec![])),
        _ => None,
    };

    if let Some(file) = file {
        Some(make_file_response(file_name, file))
    } else {
        None
    }
}

fn route(
    request: Request<Body>,
) -> impl futures::Future<Item = Response<Body>, Error = io::Error> + Send {
    let uri_path = path::PathBuf::from(request.uri().path());
    println!("{}", request.uri());
    let uri_parts: Vec<&str> = uri_path
        .components()
        .map(|component: path::Component| component.as_os_str().to_str().unwrap())
        .collect();
    let uri_parts = &uri_parts[1..];
    let response = match uri_parts.len() {
        0 => respond(page::root()),
        1 => {
            // might be a static path, or a username
            let first = uri_parts.get(0).unwrap();
            let static_response = get_static_file(first);
            match static_response {
                Some(response) => response,
                None => respond(page::user(first)),
            }
        }
        2 => {
            // this will have a user and a project
            let user_name = uri_parts.get(0).unwrap();
            let project_name = uri_parts.get(1).unwrap();
            if project_name.ends_with(".git") {
                let uri_path = request.uri().path();
                let dotgitless = &uri_path[0..uri_path.len() - 4];
                respond(Err(Missing::Elsewhere(dotgitless.to_string())))
            } else {
                respond(page::project(user_name, project_name))
            }
        }
        _ => {
            // this will have a user, a project, a page and maybe more (ref,
            // path, etc)
            let user_name = uri_parts.get(0).unwrap();
            let project_name = uri_parts.get(1).unwrap();
            let page_name = uri_parts.get(2).unwrap();
            // TODO change to as_deref when https://github.com/rust-lang/rust/issues/50264
            let target = uri_parts.get(3).map(|t| &**t);
            let rest = match uri_parts.len() {
                3 => None,
                _ => Some(&uri_parts[4..]),
            };

            let project_name = if project_name.ends_with(".git") {
                &project_name[..project_name.len() - 4]
            } else {
                *project_name
            };

            match *page_name {
                "tree" => respond(page::tree(user_name, project_name, target, rest)),
                "log" => respond(page::log(user_name, project_name, target, rest)),
                "blob" => respond(page::blob(user_name, project_name, target, rest)),
                "commit" => respond(page::commit(user_name, project_name, target, rest)),
                "refs" => respond(page::refs(user_name, project_name, target, rest)),
                "raw" => respond(page::raw(user_name, project_name, target, rest)),
                "info" => respond(page::info(user_name, project_name, target, rest)),
                "HEAD" => respond(page::head(user_name, project_name, target, rest)),
                "objects" => respond(page::objects(user_name, project_name, target, rest)),
                _ => respond(Err(Missing::Nowhere)),
            }
        }
    };
    futures::future::ok(response)
}

fn get_args() -> Vec<String> {
    let args: Vec<String> = env::args().collect();
    args
}

fn main() -> Result<(), io::Error> {
    let args = get_args();

    let should_sock = args.get(2).is_none();

    if should_sock {
        let sock_path = "./sock";
        fs::remove_file(sock_path).unwrap_or_default();
        let server = hyperlocal::server::Server::bind(sock_path, || service_fn(route))?;
        server.run()?;
    } else {
        let addr = ([127, 0, 0, 1], 3000).into();
        let server = hyper::Server::bind(&addr).serve(|| service_fn(route));
        hyper::rt::run(server.map_err(|_| {}));
    }

    Ok(())
}
