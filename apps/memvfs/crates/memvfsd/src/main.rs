use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

use clap::{Parser, Subcommand};
use memvfs_core::protocol::{handle_request, Request, Response};
use memvfs_core::{FileSystem, FileSystemConfig};

#[derive(Debug, Parser)]
#[command(name = "memvfsd", about = "In-memory VFS daemon")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Start(StartArgs),
    Status(StatusArgs),
}

#[derive(Debug, Parser)]
struct StartArgs {
    #[arg(long, default_value = "127.0.0.1:7878")]
    addr: String,

    #[arg(long, default_value_t = 4096)]
    block_size: usize,

    #[arg(long, default_value_t = 64 * 1024 * 1024)]
    capacity: usize,

    #[arg(long, default_value_t = 16_384)]
    inode_capacity: usize,
}

#[derive(Debug, Parser)]
struct StatusArgs {
    #[arg(long, default_value = "127.0.0.1:7878")]
    addr: String,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("memvfsd: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    match Cli::parse().command.unwrap_or(Commands::Start(StartArgs {
        addr: "127.0.0.1:7878".to_string(),
        block_size: 4096,
        capacity: 64 * 1024 * 1024,
        inode_capacity: 16_384,
    })) {
        Commands::Start(args) => start(args),
        Commands::Status(args) => status(args),
    }
}

fn start(args: StartArgs) -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(&args.addr)?;
    let mut fs = FileSystem::new(FileSystemConfig {
        block_size: args.block_size,
        data_capacity: args.capacity,
        inode_capacity: args.inode_capacity,
    })?;

    println!(
        "memvfsd listening on {} (block_size={}, capacity={}, inode_capacity={})",
        args.addr, args.block_size, args.capacity, args.inode_capacity
    );

    for stream in listener.incoming() {
        let mut stream = stream?;
        let request: Request = read_frame(&mut stream)?;
        let should_shutdown = matches!(request, Request::Shutdown);
        let response = handle_request(&mut fs, request);
        write_frame(&mut stream, &response)?;
        if should_shutdown {
            break;
        }
    }
    Ok(())
}

fn status(args: StatusArgs) -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TcpStream::connect(args.addr)?;
    write_frame(&mut stream, &Request::Ping)?;
    let response: Response = read_frame(&mut stream)?;
    match response {
        Response::Pong => {
            println!("memvfsd is running");
            Ok(())
        }
        Response::Error(error) => Err(format!("{}: {}", error.code, error.message).into()),
        other => Err(format!("unexpected response: {other:?}").into()),
    }
}

fn read_frame<T: serde::de::DeserializeOwned>(
    reader: &mut impl Read,
) -> Result<T, Box<dyn std::error::Error>> {
    let mut len = [0; 4];
    reader.read_exact(&mut len)?;
    let len = u32::from_be_bytes(len) as usize;
    let mut bytes = vec![0; len];
    reader.read_exact(&mut bytes)?;
    Ok(serde_json::from_slice(&bytes)?)
}

fn write_frame<T: serde::Serialize>(
    writer: &mut impl Write,
    value: &T,
) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = serde_json::to_vec(value)?;
    if bytes.len() > u32::MAX as usize {
        return Err("frame is too large".into());
    }
    writer.write_all(&(bytes.len() as u32).to_be_bytes())?;
    writer.write_all(&bytes)?;
    writer.flush()?;
    Ok(())
}
