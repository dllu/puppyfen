#[macro_use]
extern crate lazy_static;
extern crate regex;
extern crate resvg;
extern crate sha1;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::net::TcpListener;
use std::net::TcpStream;
use std::path::Path;

mod fen2svg;

fn main() {
    let _resvg = resvg::init();
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        handle_connection(stream);
    }
}

fn handle_connection(mut stream: TcpStream) {
    lazy_static! {
        static ref FEN_SVG_RE: regex::Regex =
            regex::Regex::new(r"^GET (/(r|n|b|q|k|p|R|N|B|Q|K|P|[1-8])+){8}\.svg HTTP/\d")
                .unwrap();
        static ref FEN_PNG_RE: regex::Regex =
            regex::Regex::new(r"^GET (/(r|n|b|q|k|p|R|N|B|Q|K|P|[1-8])+){8}\.png HTTP/\d")
                .unwrap();
    }
    let mut buffer = [0; 512];
    stream.read(&mut buffer).unwrap();
    let req = String::from_utf8_lossy(&buffer[..]);

    //println!("Request: {}", req);
    let status_200 = "HTTP/1.1 200 OK\r\n";
    let status_404 = "HTTP/1.1 404 NOT FOUND\r\n\r\ninvalid fen";

    if FEN_SVG_RE.is_match(&req) {
        let fen = req[5..].split(".svg HTTP/").next().unwrap().to_string();
        let svg = fen2svg::fen2svg(fen);

        stream.write(status_200.as_bytes()).unwrap();
        let content_length = format!("Content-Length: {}\r\n", svg.len());
        stream.write(content_length.as_bytes()).unwrap();
        let content_type = "Content-Type: image/svg+xml\r\n";
        stream.write(content_type.as_bytes()).unwrap();
        stream.write("\r\n".as_bytes()).unwrap();
        stream.write(svg.as_bytes()).unwrap();
    } else if FEN_PNG_RE.is_match(&req) {
        let fen = req[5..].split(".png HTTP/").next().unwrap().to_string();
        let filename: String = format!("{}.png", sha1::Sha1::from(&fen).digest());
        let path = Path::new(&filename);
        if !path.exists() {
            let svg = fen2svg::fen2svg(fen);
            let backend = resvg::default_backend();
            let opt = resvg::Options::default();
            let usvg_opt = resvg::usvg::Options::default();
            let rtree = resvg::usvg::Tree::from_str(svg.as_str(), &usvg_opt).unwrap();
            let img = backend.render_to_image(&rtree, &opt).unwrap();
            img.save(path);
        }
        let mut png_file = File::open(path).unwrap();
        let mut png_buffer = Vec::new();
        if let Err(result) = png_file.read_to_end(&mut png_buffer) {
            println!("couldn't write png: {}", result);
            stream.write(status_404.as_bytes()).unwrap();
        } else {
            stream.write(status_200.as_bytes()).unwrap();
            let content_length = format!("Content-Length: {}\r\n", png_buffer.len());
            stream.write(content_length.as_bytes()).unwrap();
            let content_type = "Content-Type: image/png\r\n";
            stream.write(content_type.as_bytes()).unwrap();
            stream.write("\r\n".as_bytes()).unwrap();
            stream.write(png_buffer.as_slice()).unwrap();
        }
    } else {
        stream.write(status_404.as_bytes()).unwrap();
    }

    stream.flush().unwrap();
}
