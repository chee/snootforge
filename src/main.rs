#![feature(proc_macro_hygiene)]

extern crate futures;
extern crate git2;
extern crate hyper;
extern crate hyperlocal;
extern crate maud;
extern crate mime;
extern crate comrak;

use crate::futures::Future;
use hyper::{header, Body, Request, Response, StatusCode};
use hyper::service::service_fn;
use std::{fs, io, path, env};
use maud::{html, Markup};

mod markup;
mod page;
mod user;
mod repository;

fn respond (markup: Result<Markup, StatusCode>) -> Response<Body> {
    match markup {
        Ok(markup) => {
            let body = markup::render(
                html! {
                    (markup::head("snootforge"))
                        main {
                            (markup)
                        }
                }
            );

            Response::builder()
                .header(header::CONTENT_LENGTH, body.len() as u64)
                .header(header::CONTENT_TYPE, "text/html")
                .body(Body::from(body))
                .expect("Failed to construct the response")
        },
        Err(status) => {
            Response::builder()
                .status(status)
                .body(Body::from("sorry"))
                .expect("failed")
        }
    }
}

fn get_git_root () -> path::PathBuf {
    let args = get_args();
    let mut pathbuf = path::PathBuf::new();
    pathbuf.push(args.get(1).expect("Usage: yeet <root_path>"));
    pathbuf
}

fn guess_mime (file_path: &path::PathBuf) -> mime::Mime {
    let extension = file_path.extension();
    match extension {
        Some(extension) =>
            match extension.to_str() {
                Some("png") => mime::IMAGE_PNG,
                Some("jpg") => mime::IMAGE_JPEG,
                Some("jpeg") => mime::IMAGE_JPEG,
                Some("gif") => mime::IMAGE_GIF,
                Some("svg") => mime::IMAGE_SVG,
                Some("css") => mime::TEXT_CSS,
                Some("json") => mime::APPLICATION_JSON,
                Some("js") => mime::APPLICATION_JAVASCRIPT,
                Some(&_) => mime::TEXT_PLAIN,
                None => mime::TEXT_PLAIN
            },
        None => mime::TEXT_PLAIN
    }
}

fn make_file_response (file_path: &path::PathBuf, body: Vec<u8>) -> Response<Body> {
    let file_mime = guess_mime(file_path).to_string();

    Response::builder()
        .header(header::CONTENT_LENGTH, body.len() as u64)
        .header(header::CONTENT_TYPE, file_mime)
        .body(Body::from(body))
        .expect("tried to make a static file and didn't")
}

fn get_static_file (file_path: &path::PathBuf) -> Option<Response<Body>> {
    match fs::read(file_path) {
        Ok(file) => Some(make_file_response(file_path, file)),
        Err(_) => None
    }
}

fn route(request: Request<Body>) -> impl futures::Future<Item = Response<Body>, Error = io::Error> + Send {
    let uri_path = path::PathBuf::from(request.uri().path());
    let mut parts = uri_path.components();
    parts.next();
    let user_name = parts.next();
    let project_name = parts.next();
    let page_name = parts.next();
    let ref_name = parts.next();
    let response = match (user_name, project_name, page_name, ref_name) {
        (None, None, None, None) => {
           respond(page::root())
        }
        (Some(first), None, None, None) => {
            let first = first.as_os_str().to_str().unwrap();
            // might be a static file
            let mut static_path = path::PathBuf::from("static/");
            static_path.push(first);
            let static_response = get_static_file(&static_path);
            match static_response {
                Some(response) => response,
                None => {
                    respond(page::user(first))
                }
            }
        }
        (Some(user_name), Some(project_name), None, None) => {
            let user_name = user_name.as_os_str().to_str().unwrap();
            let project_name = project_name.as_os_str().to_str().unwrap();
            respond(page::project(user_name, project_name))
        }
        (Some(user_name), Some(project_name), Some(page_name), None) => {
            let user_name = user_name.as_os_str().to_str().unwrap();
            let project_name = project_name.as_os_str().to_str().unwrap();
            let page_name = page_name.as_os_str().to_str().unwrap();
            respond(Ok(html!(h1 {(user_name) "/" (project_name) "/" (page_name)})))
        }
        (Some(user_name), Some(project_name), Some(page_name), Some(ref_name)) => {
            let user_name = user_name.as_os_str().to_str().unwrap();
            let project_name = project_name.as_os_str().to_str().unwrap();
            let page_name = page_name.as_os_str().to_str().unwrap();
            let ref_name = ref_name.as_os_str().to_str().unwrap();
            respond(Ok(html!(
                h1 {
                    (user_name) "/" (project_name) "/" (page_name) "/" (ref_name)
                }))
            )
        }
        _ => respond(Ok(html!(h1{"404"})))
    };
    futures::future::ok(response)
}

fn get_args () -> Vec<String> {
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
