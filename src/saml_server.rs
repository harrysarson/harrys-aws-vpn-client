use std::collections::HashMap;
use std::io::{BufRead, Read, Write};
use std::mem;

fn handle_connection(mut stream: std::net::TcpStream) -> String {
    let mut reader = std::io::BufReader::new(&mut stream);


    let mut header = Vec::new();
    let mut content_len = None::<usize>;

    loop {
        let mut buf = String::new();
        reader.read_line(&mut buf).unwrap();
        if buf.trim().is_empty() {
            break;
        }
        if let Some(len_s) = buf.to_ascii_lowercase().strip_prefix("content-length:") {
            content_len = Some(len_s.trim().parse().unwrap());
        }
        header.push(buf);
    }

    println!("header\n{:?}", &header);

    let mut body = vec![0; content_len.unwrap()];
    reader.read_exact(&mut body).unwrap();

    let status_line = "HTTP/1.1 200 OK";
    let contents = "Got SAMLResponse field, it is now safe to close this window";
    let length = contents.len();

    let response = format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");

    stream.write_all(response.as_bytes()).unwrap();

    let mut form: HashMap<String, String> = form_urlencoded::parse(&body).into_owned().collect();

    tracing::debug!("Got form with keys {:?}", form.keys().collect::<Vec<_>>());

    mem::take(form.get_mut("SAMLResponse").unwrap())
}

pub(crate) fn run_server_for_saml(port: u16) -> Saml {
    tracing::info!("Starting SAML server at 0.0.0.0:{port}...");

    let listener = std::net::TcpListener::bind(("0.0.0.0", port)).unwrap();

    let (socket, _) = listener.accept().unwrap();
    let saml_response = handle_connection(socket);

    Saml {
        data: saml_response,
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Saml {
    pub(crate) data: String,
}
