use std::io;
use std::io::{Read, Write};
use std::fs;
use std::fs::{DirBuilder, File};
use std::path::Path;
use std::error::Error;
use super::generation::PageGenerator;
use super::config::Config;
use hyper::server::{Request, Response, Server};
use hyper::status::StatusCode;
use hyper::uri::RequestUri;
use hyper::method::Method;
use pulldown_cmark::{Options, OPTION_ENABLE_TABLES, OPTION_ENABLE_FOOTNOTES};

const DEFAULT_CONFIG_FILE: &'static str = "\
source: pages
output: _site
port: 4000
markdown_options:
  - tables
  - footnotes\n
";

const BAD_REQUEST: &'static str = "\
<h1>400 Bad Request</h1>
";

const NOT_FOUND: &'static str = "\
<h1>404 Not Found</h1>
";

pub fn new_project(parent_dir: &str) -> Result<(), io::Error> {
    try!(DirBuilder::new().recursive(true).create(parent_dir));

    try!(DirBuilder::new().recursive(false).create(format!("{}/pages", parent_dir)));

    let mut config_file = try!(File::create(format!("{}/_config.yml", parent_dir)));

    try!(config_file.write_all(DEFAULT_CONFIG_FILE.as_bytes()));

    Ok(())
}

pub fn build_project(config: &Config) -> Result<(), io::Error> {
    let pages_path = &*config.source_dir;
    let output_dir = &*config.output_dir;
    let mut markdown_options = Options::empty();

    if config.markdown_options.contains(&"footnotes".to_string()) {
        markdown_options.insert(OPTION_ENABLE_FOOTNOTES);
    }

    if config.markdown_options.contains(&"tables".to_string()) {
        markdown_options.insert(OPTION_ENABLE_TABLES);
    }

    let mut page_generator = PageGenerator::new();

    let directory_iterator = try!(Path::new(pages_path).read_dir());

    if !Path::new(output_dir).exists() {
        try!(DirBuilder::new().create(output_dir));
    }

    for entry in directory_iterator {
        let file = try!(entry);
        let file_type = try!(file.file_type());

        let file_name = file.file_name().into_string().expect("File Name");

        let source_file = format!("{}/{}", pages_path, file_name);

        let file_stem = file.path().file_stem().expect("File Stem").to_string_lossy().into_owned();

        let destination_file = format!("{}/{}.html", output_dir, file_stem);

        if file_type.is_file() && file_name.contains(".md") {
            try!(page_generator.set_input_file(source_file.as_str())
                     .set_output_file(destination_file.as_str())
                     .set_wrap(true)
                     .set_parse_options(markdown_options.clone())
                     .generate());
        }
    }

    Ok(())
}

pub fn clean_project(config: &Config) -> Result<(), io::Error> {
    try!(fs::remove_dir_all(&*config.output_dir));

    Ok(())
}

pub fn serve(config: &Config) -> Result<(), io::Error> {
    try!(build_project(&config));

    let server_addr = format!("127.0.0.1:{}", &*config.port);
    let server = match Server::http(server_addr.as_str()) {
        Ok(server) => server,
        Err(what) => panic!("{}", Error::description(&what))
    };

    let serve_dir = config.output_dir.clone();
    match server.handle(move |request: Request, response: Response| {
        match handle_static_file(&serve_dir, request, response) {
            Ok(_) => {},
            Err(what) => panic!("{}", Error::description(&what))
        }
    }) {
        Ok(_) => {},
        Err(what) => panic!("{}", Error::description(&what))
    }

    Ok(())
}

fn handle_static_file(page_dir: &str, request: Request, mut response: Response) -> Result<(), io::Error> {
    let path = match request.uri {
        RequestUri::AbsolutePath(ref uri) if request.method == Method::Get => uri,
        _ => {
            *response.status_mut() = StatusCode::BadRequest;
            let body = BAD_REQUEST.as_bytes();
            try!(response.send(body));
            return Ok(())
        }
    };

    let file_path = Path::new(page_dir).join(&path[1..]);

    if file_path.exists() && file_path.is_file() {
        let mut file = try!(File::open(file_path));
        let mut file_contents = String::new();

        try!(file.read_to_string(&mut file_contents));

        *response.status_mut() = StatusCode::Ok;
        try!(response.send(&file_contents.into_bytes()));
        return Ok(())
    } else {
        *response.status_mut() = StatusCode::NotFound;
        let body = NOT_FOUND.as_bytes();
        try!(response.send(body));
        return Ok(())
    }

    Ok(())
}
