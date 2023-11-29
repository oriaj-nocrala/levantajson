pub(crate) mod assets;

use actix_cors::Cors;
use actix_web::Responder;
use actix_web::http::header;
use actix_web::middleware::DefaultHeaders;
use actix_web::{web, App, HttpServer, HttpResponse};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::sync::Mutex;
use std::collections::HashMap;
use std::path::Path;
use assets::index::get_index_json;

#[derive(Debug, Deserialize, Clone, Serialize)]
struct Config {
    host: String,
    port: u16,
    json_directories: Vec<String>,
}

fn process_entry(entry: fs::DirEntry, directorio: &str, endpoint_to_file: &mut HashMap<String, String>) {
    let path = entry.path();
    println!("Procesando archivo: {:?}", path.file_name()); 

    if path.is_file() && path.extension() == Some(std::ffi::OsStr::new("json")) {
        let directorio = directorio.trim_start_matches("./");
        if path.file_name().unwrap_or_default().to_str() == Some("index.json") {
            let content = fs::read_to_string(&path).expect("Error al intentar leer el archivo");
            endpoint_to_file.insert(directorio.to_string(), content);
        } else {
            let endpoint = format!("{}/{}", directorio, path.file_stem().unwrap().to_str().unwrap());
            let content = fs::read_to_string(&path).expect("Error al intentar leer el fichero");
            endpoint_to_file.insert(endpoint, content);
        }
    }
}

fn process_directory(directorio: &str, endpoint_to_file: &mut HashMap<String, String>) {
    let entries = fs::read_dir(directorio).expect("Error leyendo directorio");
    for entry in entries {
        let entry = entry.expect("Error leyendo archivo");
        process_entry(entry, directorio, endpoint_to_file);
    }
}

async fn leer_directorio(config: Config) -> HashMap<String, String> {
    let mut endpoint_to_file = HashMap::new();
    for directorio in &config.json_directories {
        if !Path::new(directorio).exists() {
            fs::create_dir(directorio).expect("No se pudo crear el directorio");
            let json = get_index_json();
            let path = format!("{}/index.json", directorio);
            fs::write(path, json).expect("Imposible escribir archivo");
        }
        process_directory(directorio, &mut endpoint_to_file);
    }
    endpoint_to_file
}

async fn read_config(filename: &str) -> Config {
    let content = fs::read_to_string(filename).expect("Imposible leer archivo");
    serde_json::from_str(&content).expect("El JSON no es válido")
}

async fn index(data: web::Data<Mutex<HashMap<String, String>>>, path: web::Path<String>) -> impl Responder {
    let map = data.lock().unwrap();
    let path = path.into_inner();
    if let Some(response) = map.get(&path) {
        println!("Ruta solicitada: {}", &path);
        HttpResponse::Ok().body(response.clone())

    } else {
        println!("La ruta no fue encontrada: {}", &path);
        HttpResponse::NotFound().finish()
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let config;
    if args.len() != 2 {
        //leer el archivo de configuracion config.json si existe:
        if !Path::new("config.json").exists() {
            config = Config {
                host: "localhost".to_string(),
                port: 3000,
                json_directories: vec!["./json".to_string()],
            };
            let json = serde_json::to_string_pretty(&config).unwrap();
            fs::write("config.json", json).expect("Imposible escribir archivo");
        } else {
            config = read_config("config.json").await;
        }
    } else {
        config = read_config(&args[1]).await;
    }
    let endpoint_to_file = leer_directorio(config.clone()).await;
    println!("Endpoints: {:?}", endpoint_to_file.keys());

    let data = web::Data::new(Mutex::new(endpoint_to_file));

    let ip_addr = config.host.clone();
    let port = config.port.clone();
    match HttpServer::new(move || {
        let cors = Cors::default()
        .allow_any_origin()
        .allow_any_method()
        .allow_any_header()
        .max_age(3600);
        App::new()
        .wrap(DefaultHeaders::new().add((header::CONTENT_TYPE, "application/json")))
        .wrap(cors)
            .app_data(data.clone())
            .service(web::resource("/{path:.*}").to(index))
    })
    .bind((config.host, config.port)) {
        Ok(server) => {
            println!("Servidor ejecutándose en http://{}:{}", ip_addr, port);
            println!("Servidor versión pico.zorra.seaman.brigidamente.0.1.1");
           

            server
        }
        Err(error) => {
            println!("Error iniciando el servidor: {}", error);
            return Ok(());
        }
    }
    .run()
    .await
}
