use tiny_http::{Server, Response, Request};
use std::fs::{File, read};
use std::io::Read;

fn get_path(url : &str) -> &str
{
    if (url == "/")
    {
        return "static/index.html";
    }
    else {
        let chars = url.chars();
        chars.next();
        return chars.as_str();
    }
}

fn main()
{
    let server : Server = Server::http("0.0.0.0:8000").unwrap();

    while (true)
    {
        let opReq = server.recv();
        if (opReq.is_err())
        {
            continue;
        }

        let request : Request = opReq.unwrap();

        let url : &str = request.url();

        let path : &str = get_path(url);
        println!("Request: {}", path);

        let response = read(&path);
        if (response.is_err())
        {
            request.respond(Response::from_string("404"));
        }
        else
        {
            request.respond(Response::from_data(response.unwrap()));
        }
    }
}