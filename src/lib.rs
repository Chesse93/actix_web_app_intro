#[macro_use]
extern crate actix_web;

use actix_web::{
    error::{Error, InternalError, JsonPayloadError},
    middleware, web::{self, Data}, App, HttpRequest, HttpResponse, HttpServer, Result};
use serde::{Deserialize, Serialize};
use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

static SERVER_COUNTER: AtomicUsize = AtomicUsize::new(0);
const LOG_FORMAT: &'static str = r#""%r" %s %b "%{User-Agent}i" %D"#;

struct AppState {
    server_id: usize,
    request_count: Cell<usize>,
    messages: Arc<Mutex<Vec<String>>>,
}
pub struct MessageApp {
    port: u16,
}

#[derive(Deserialize)]
struct PostInput {
    message: String,
}

#[derive(Serialize)]
struct PostError {
    server_id: usize,
    request_count: usize,
    error: String,
}

#[derive(Serialize)]
struct PostResponse {
    server_id: usize,
    request_count: usize,
    message: String,
}

#[derive(Serialize)]
struct IndexResponse {
    server_id: usize,
    request_count: usize,
    messages: Vec<String>,
}

#[derive(Serialize)]
struct LookupResponse {
    server_id: usize,
    request_count: usize,
    result: Option<String>,
}

#[get("/")]
async fn index(state: web::Data<AppState>) -> Result<web::Json<IndexResponse>> {
    let request_count = state.request_count.get() + 1;
    state.request_count.set(request_count);
    let ms = state.messages.lock().unwrap();
        
    Ok(web::Json(IndexResponse {
        server_id: state.server_id,
        request_count,
        messages: ms.clone(),
    }))
}

#[get("/lookup/{index}")]
async fn lookup(state: web::Data<AppState>, idx: web::Path<usize>) -> Result<web::Json<LookupResponse>> {
    let request_count = state.request_count.get() + 1;
    state.request_count.set(request_count);
    let ms = state.messages.lock().unwrap();
    let result = ms.get(idx.into_inner()).cloned();

    Ok(web::Json(LookupResponse {
        server_id: state.server_id,
        request_count,
        result
    }))
}

async fn post(msg: web::Json<PostInput>, state: web::Data<AppState>) -> Result<web::Json<PostResponse>> {
    let request_count = state.request_count.get() + 1;
    state.request_count.set(request_count);
    let mut ms = state.messages.lock().unwrap();
    ms.push(msg.message.clone());

    Ok(web::Json(PostResponse {
        server_id: state.server_id,
        request_count,
        message: msg.message.clone(),
    }))
}

fn post_error(err: JsonPayloadError, req: &HttpRequest) -> Error {
    let state = req.app_data::<web::Data<AppState>>().unwrap();
    let request_count = state.request_count.get() + 1;
    let post_error = PostError {
        server_id: state.server_id,
        request_count,
        error: format!("{}", err),
    };
    InternalError::from_response(err, HttpResponse::BadRequest().json(post_error)).into()
}

#[post("/clear")]
async fn clear(state: web::Data<AppState>) -> Result<web::Json<IndexResponse>> {
    let request_count = state.request_count.get() + 1;
    state.request_count.set(request_count);
    let mut ms = state.messages.lock().unwrap();
    ms.clear();

    Ok(web::Json(IndexResponse {
        server_id: state.server_id,
        request_count,
        messages: vec![],
    }))
}

impl MessageApp {
    pub fn new(port: u16) -> Self {
        MessageApp { port }
    }

    #[actix_web::main]
    pub async fn run(&self) -> std::io::Result<()> {
        let messages = Arc::new(Mutex::new(vec![]));
        println!("Starting http server 127.0.0.1:{}", self.port);
        HttpServer::new(move || {
            // App is the abstraction actix-web defines for representing a collection of routes and their handlers
            App::new()
                .app_data(Data::new(AppState {
                    server_id: SERVER_COUNTER.fetch_add(1, Ordering::SeqCst),
                    request_count: Cell::new(0),
                    messages: messages.clone(),
                }))
                // Wraps the app with Logger middleware which is provided by actix so that we can see information about requests
                .wrap(middleware::Logger::new(LOG_FORMAT))
                // Sepcifies that we want to add index service to our app
                .service(index)
                .service(
                    web::resource("/send")
                    .app_data(
                        web::JsonConfig::default()
                        .limit(4096)
                        .error_handler(post_error),
                    )
                    .route(web::post().to(post)),
                )
                .service(clear)
                .service(lookup)
        })
        .bind(("127.0.0.1", self.port))?
        .workers(8)
        .run()
        .await
    }
}