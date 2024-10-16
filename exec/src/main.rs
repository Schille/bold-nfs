use std::fs;

use bold::ServerBuilder;
use clap::Parser;
use memoryfs::create_memory_fs;
use tracing::Level;

mod memoryfs;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Path to a memory fs YAML file
    fakefs: Option<String>,
    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,
}

fn main() {
    let cli = Cli::parse();

    if cli.debug {
        tracing_subscriber::fmt()
            .with_max_level(Level::DEBUG)
            .init();
    } else {
        tracing_subscriber::fmt().with_max_level(Level::INFO).init();
    }

    let fakefs = cli.fakefs.unwrap_or("bold-demo/memoryfs.yaml".to_string());

    println!("Loading YAML: {:?}", fakefs);
    let contents = fs::read_to_string(fakefs).expect("Should have been able to read the file");
    let root_dir: memoryfs::Directory = serde_yaml::from_str(&contents).unwrap();

    let root = create_memory_fs(root_dir);

    let server = ServerBuilder::new(root).bind("127.0.0.1:11112").build();
    server.start();
}
