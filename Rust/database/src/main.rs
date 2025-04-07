mod btree;
mod cli;
mod storage;
mod web;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Add command line argument parsing
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "--cli" {
        // Run in CLI mode if requested
        cli::start_cli()
    } else {
        // Otherwise start the web server
        web::start_server().await
    }
}