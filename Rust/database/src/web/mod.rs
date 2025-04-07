// web.rs
use actix_web::{web, App, HttpResponse, HttpServer, Responder, middleware};
use actix_cors::Cors;
use actix_files as fs;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::collections::HashMap;
use std::io;

use crate::btree::{BTree, Record};
use crate::storage::{load_records, save_records};

// Structure to hold our database connections
struct AppState {
    databases: Mutex<HashMap<String, BTree>>,
}

#[derive(Serialize)]
struct ApiResponse {
    success: bool,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Vec<RecordDto>>,
}

#[derive(Serialize, Deserialize)]
struct RecordDto {
    key: i32,
    value: String,
}

#[derive(Deserialize)]
struct ConnectRequest {
    db_name: String,
}

#[derive(Deserialize)]
struct InsertRequest {
    key: i32,
    value: String,
}

#[derive(Deserialize)]
struct KeyRequest {
    key: i32,
}

// Helper function to convert between domain Record and DTO
impl From<Record> for RecordDto {
    fn from(record: Record) -> Self {
        RecordDto {
            key: record.key,
            value: record.value,
        }
    }
}

// Serve static files (HTML, CSS, JS)
async fn index() -> impl Responder {
    fs::NamedFile::open_async("./static/index.html").await
}

// API endpoint to connect to a database
async fn connect_database(
    data: web::Data<AppState>,
    req: web::Json<ConnectRequest>,
) -> impl Responder {
    let db_name = &req.db_name;
    let file_path = format!("{}.db", db_name);
    
    let mut databases = data.databases.lock().unwrap();
    
    // Check if we're already connected to this DB
    if databases.contains_key(db_name) {
        return HttpResponse::Ok().json(ApiResponse {
            success: true,
            message: format!("Already connected to database: {}", db_name),
            data: None,
        });
    }
    
    // Try to load records from the database file
    match load_records(&file_path) {
        Ok(records) => {
            let mut tree = BTree::new();
            
            // Insert all records into the tree
            for record in records {
                tree.insert(record.key, record.value);
            }
            
            // Store the tree in our app state
            databases.insert(db_name.clone(), tree);
            
            HttpResponse::Ok().json(ApiResponse {
                success: true,
                message: format!("Connected to database: {}", db_name),
                data: None,
            })
        }
        Err(error) => {
            if error.kind() == io::ErrorKind::NotFound {
                // If file doesn't exist, create a new empty database
                let tree = BTree::new();
                databases.insert(db_name.clone(), tree);
                
                HttpResponse::Ok().json(ApiResponse {
                    success: true,
                    message: format!("Created new database: {}", db_name),
                    data: None,
                })
            } else {
                // If there was another error, return it
                HttpResponse::InternalServerError().json(ApiResponse {
                    success: false,
                    message: format!("Failed to connect to database: {}", error),
                    data: None,
                })
            }
        }
    }
}

// API endpoint to get all records
async fn get_all_records(
    data: web::Data<AppState>,
    path: web::Path<String>,
) -> impl Responder {
    let db_name = path.into_inner();
    let databases = data.databases.lock().unwrap();
    
    if let Some(tree) = databases.get(&db_name) {
        let records = tree.get_all_records();
        let records_dto: Vec<RecordDto> = records.into_iter().map(|r| r.into()).collect();
        
        HttpResponse::Ok().json(ApiResponse {
            success: true,
            message: format!("Retrieved {} records", records_dto.len()),
            data: Some(records_dto),
        })
    } else {
        HttpResponse::NotFound().json(ApiResponse {
            success: false,
            message: format!("Database '{}' not found", db_name),
            data: None,
        })
    }
}

// API endpoint to find a record by key
async fn find_record(
    data: web::Data<AppState>,
    path: web::Path<(String, i32)>,
) -> impl Responder {
    let (db_name, key) = path.into_inner();
    let databases = data.databases.lock().unwrap();
    
    if let Some(tree) = databases.get(&db_name) {
        if let Some(value) = tree.search(key) {
            let record = RecordDto { key, value };
            HttpResponse::Ok().json(ApiResponse {
                success: true,
                message: format!("Found record with key {}", key),
                data: Some(vec![record]),
            })
        } else {
            HttpResponse::NotFound().json(ApiResponse {
                success: false,
                message: format!("Record with key {} not found", key),
                data: None,
            })
        }
    } else {
        HttpResponse::NotFound().json(ApiResponse {
            success: false,
            message: format!("Database '{}' not found", db_name),
            data: None,
        })
    }
}

// API endpoint to insert a record
async fn insert_record(
    data: web::Data<AppState>,
    path: web::Path<String>,
    req: web::Json<InsertRequest>,
) -> impl Responder {
    let db_name = path.into_inner();
    let mut databases = data.databases.lock().unwrap();
    
    if let Some(tree) = databases.get_mut(&db_name) {
        let file_path = format!("{}.db", db_name);
        
        // Check if key already exists
        let updating = tree.search(req.key).is_some();
        
        // Insert the record
        tree.insert(req.key, req.value.clone());
        
        // Save changes to disk
        match save_records(&file_path, &tree.get_all_records()) {
            Ok(_) => {
                let message = if updating {
                    format!("Updated record with key {}", req.key)
                } else {
                    format!("Inserted new record with key {}", req.key)
                };
                
                HttpResponse::Ok().json(ApiResponse {
                    success: true,
                    message,
                    data: None,
                })
            }
            Err(error) => {
                HttpResponse::InternalServerError().json(ApiResponse {
                    success: false,
                    message: format!("Failed to save changes: {}", error),
                    data: None,
                })
            }
        }
    } else {
        HttpResponse::NotFound().json(ApiResponse {
            success: false,
            message: format!("Database '{}' not found", db_name),
            data: None,
        })
    }
}

// API endpoint to delete a record
async fn delete_record(
    data: web::Data<AppState>,
    path: web::Path<(String, i32)>,
) -> impl Responder {
    let (db_name, key) = path.into_inner();
    let mut databases = data.databases.lock().unwrap();
    
    if let Some(tree) = databases.get_mut(&db_name) {
        let file_path = format!("{}.db", db_name);
        
        // Try to delete the record
        let deleted = tree.delete(key);
        
        if deleted {
            // Save changes to disk
            match save_records(&file_path, &tree.get_all_records()) {
                Ok(_) => {
                    HttpResponse::Ok().json(ApiResponse {
                        success: true,
                        message: format!("Deleted record with key {}", key),
                        data: None,
                    })
                }
                Err(error) => {
                    HttpResponse::InternalServerError().json(ApiResponse {
                        success: false,
                        message: format!("Failed to save changes: {}", error),
                        data: None,
                    })
                }
            }
        } else {
            HttpResponse::NotFound().json(ApiResponse {
                success: false,
                message: format!("Record with key {} not found", key),
                data: None,
            })
        }
    } else {
        HttpResponse::NotFound().json(ApiResponse {
            success: false,
            message: format!("Database '{}' not found", db_name),
            data: None,
        })
    }
}

// Main function to start the web server
pub async fn start_server() -> io::Result<()> {
    println!("Starting B-Tree database web server...");
    println!("Open your browser and navigate to: http://localhost:8080");
    
    // Create the app state with an empty map of databases
    let app_state = web::Data::new(AppState {
        databases: Mutex::new(HashMap::new()),
    });
    
    // Start the HTTP server
    HttpServer::new(move || {
        // Configure CORS to allow frontend access
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header();
        
        App::new()
            .wrap(middleware::Logger::default())
            .wrap(cors)
            .app_data(app_state.clone())
            // API routes
            .service(
                web::scope("/api")
                    .route("/connect", web::post().to(connect_database))
                    .route("/db/{db_name}/records", web::get().to(get_all_records))
                    .route("/db/{db_name}/records/{key}", web::get().to(find_record))
                    .route("/db/{db_name}/records", web::post().to(insert_record))
                    .route("/db/{db_name}/records/{key}", web::delete().to(delete_record))
            )
            // Static files
            .service(fs::Files::new("/static", "./static").show_files_listing())
            .default_service(web::get().to(index))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}