#![feature(proc_macro_hygiene)]

extern crate chrono_humanize;
extern crate comrak;
extern crate futures;
extern crate git2;
extern crate hyper;
extern crate hyperlocal;
extern crate maud;
extern crate mime;

use crate::futures::Future;
use hyper::service::service_fn;
use hyper::{header, Body, Request, Response, StatusCode};
use maud::{html, Markup};
use std::{env, fs, io, path};

mod blob;
mod markup;
mod page;
mod repository;
mod tree;
mod user;

fn respond(markup: Result<Markup, StatusCode>) -> Response<Body> {
    match markup {
        Ok(markup) => {
            let body = markup::render(html! {
                (markup::head("snootforge"))
                    main {
                        (markup)
                    }
            });

            Response::builder()
                .header(header::CONTENT_LENGTH, body.len() as u64)
                .header(header::CONTENT_TYPE, "text/html")
                .body(Body::from(body))
                .expect("Failed to construct the response")
        }
        Err(status) => Response::builder()
            .status(status)
            .body(Body::from("sorry"))
            .expect("failed"),
    }
}

fn get_git_root() -> path::PathBuf {
    let args = get_args();
    let mut pathbuf = path::PathBuf::new();
    pathbuf.push(args.get(1).expect("Usage: yeet <root_path>"));
    pathbuf
}

fn guess_mime(file_path: &path::PathBuf) -> mime::Mime {
    let extension = file_path.extension();
    match extension {
        Some(extension) => match extension.to_str() {
            Some("png") => mime::IMAGE_PNG,
            Some("jpg") => mime::IMAGE_JPEG,
            Some("jpeg") => mime::IMAGE_JPEG,
            Some("gif") => mime::IMAGE_GIF,
            Some("svg") => mime::IMAGE_SVG,
            Some("css") => mime::TEXT_CSS,
            Some("json") => mime::APPLICATION_JSON,
            Some("js") => mime::APPLICATION_JAVASCRIPT,
            Some(&_) => mime::TEXT_PLAIN,
            None => mime::TEXT_PLAIN,
        },
        None => mime::TEXT_PLAIN,
    }
}

fn make_file_response(file_path: &path::PathBuf, body: Vec<u8>) -> Response<Body> {
    let file_mime = guess_mime(file_path).to_string();

    Response::builder()
        .header(header::CONTENT_LENGTH, body.len() as u64)
        .header(header::CONTENT_TYPE, file_mime)
        .body(Body::from(body))
        .expect("tried to make a static file and didn't")
}

fn get_static_file(file_path: &path::PathBuf) -> Option<Response<Body>> {
    match fs::read(file_path) {
        Ok(file) => Some(make_file_response(file_path, file)),
        Err(_) => None,
    }
}

fn route(
    request: Request<Body>,
) -> impl futures::Future<Item = Response<Body>, Error = io::Error> + Send {
    let uri_path = path::PathBuf::from(request.uri().path());
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
            let mut static_path = path::PathBuf::from("static/");
            static_path.push(first);
            let static_response = get_static_file(&static_path);
            match static_response {
                Some(response) => response,
                None => respond(page::user(first)),
            }
        }
        2 => {
            // this will have a user and a project
            let user_name = uri_parts.get(0).unwrap();
            let project_name = uri_parts.get(1).unwrap();
            respond(page::project(user_name, project_name))
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
            match *page_name {
                "tree" => respond(page::tree(user_name, project_name, target, rest)),
                "log" => respond(page::log(user_name, project_name, target, rest)),
                "blob" => respond(page::blob(user_name, project_name, target, rest)),
                "commit" => respond(page::commit(user_name, project_name, target, rest)),
                "branches" => respond(page::branches(user_name, project_name, target, rest)),
                _ => respond(Err(StatusCode::NOT_FOUND)),
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
