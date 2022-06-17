#[macro_use]
extern crate actix_web;

use actix_web::{middleware, web, App, HttpRequest, HttpServer, Result};
use serde::Serialize;

pub struct MessageApp {
    port: u16,
}

impl MessageApp {
    pub fn new(port: u16) -> Self {
        MessageApp { port }
    }

    pub fn run(&self) -> std::io::Result<()> {
        println!("Starting http server 127.0.0.1:{}", self.port);
        HttpServer::new(move || {
            // App is the abstraction actix-web defines for representing a collection of routes and their handlers
            App::new()
                // Wraps the app with Logger middleware which is provided by actix so that we can see information about requests
                .wrap(middleware::Logger::default())
                // Sepcifies that we want to add index service to our app
                .service(index)
        })
        /* 
            bind returns a Result, by putting the ? after the call, we are saying that if the returned Result is the Err variant, then just return early with that value
            .bind(("127.0.0.1", self.port));
            if result.is_err() {
                return Err(result.err().unwrap());
            }
            result.unwrap().workers(8).run()
        */ 
        .bind(("127.0.0.1", self.port))?
        .workers(8)
        .run()
    }
}

#[derive(Serialize)]
struct IndexResponse {
    message: String,
}

#[get("/")]
fn index(req: HttpRequest) -> Result<web::Json<IndexResponse>> {
    let hello = req
        .headers()
        .get("hello")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_else(|| "world");
        
    Ok(web::Json(IndexResponse {
        message: hello.to_owned(),
    }))
}