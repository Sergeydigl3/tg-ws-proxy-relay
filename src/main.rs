use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, Query},
    response::IntoResponse,
    routing::get,
    Router,
};
use clap::{Parser, ValueEnum};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::Deserialize;
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[cfg(unix)]
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    server::conn::auto::Builder,
};
#[cfg(unix)]
use tokio::net::UnixListener;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Listen type: tcp or unix
    #[arg(short, long, default_value = "tcp")]
    listen_type: ListenType,

    /// Listen address (e.g. 127.0.0.1:8080 for tcp, or /tmp/worker.sock for unix)
    #[arg(short = 'a', long, default_value = "127.0.0.1:8080")]
    listen_addr: String,
}

#[derive(Clone, Debug, ValueEnum)]
enum ListenType {
    Tcp,
    Unix,
}

#[derive(Deserialize)]
struct WsParams {
    dst: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let app = Router::new().route("/apiws", get(ws_handler));

    match args.listen_type {
        ListenType::Tcp => {
            let addr: SocketAddr = args.listen_addr.parse().expect("Invalid TCP address");
            let listener = TcpListener::bind(&addr).await.expect("Failed to bind TCP listener");
            println!("Listening on tcp://{}", addr);
            axum::serve(listener, app.into_make_service()).await.unwrap();
        }
        ListenType::Unix => {
            #[cfg(unix)]
            {
                let path = PathBuf::from(&args.listen_addr);
                // remove existing socket if it exists
                let _ = std::fs::remove_file(&path);
                let uds = UnixListener::bind(&path).expect("Failed to bind Unix listener");
                println!("Listening on unix://{}", path.display());
                
                loop {
                    let (socket, _) = uds.accept().await.unwrap();
                    let io = TokioIo::new(socket);
                    let app = app.clone();

                    tokio::task::spawn(async move {
                        let service = hyper_util::service::TowerToHyperService::new(app);
                        if let Err(err) = Builder::new(TokioExecutor::new())
                            .serve_connection_with_upgrades(io, service)
                            .await
                        {
                            eprintln!("Error serving connection: {}", err);
                        }
                    });
                }
            }
            #[cfg(not(unix))]
            {
                panic!("Unix sockets are not supported on this platform");
            }
        }
    }
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<WsParams>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, params.dst))
}

async fn handle_socket(socket: WebSocket, dst: String) {
    let target = format!("{}:443", dst);
    
    let tcp_stream = match TcpStream::connect(&target).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to connect to {}: {}", target, e);
            return;
        }
    };

    let (mut client_ws_sender, mut client_ws_receiver) = socket.split();
    let (mut tcp_reader, mut tcp_writer) = tcp_stream.into_split();

    // Task to read from WebSocket and write to TCP
    let mut ws_to_tcp = tokio::spawn(async move {
        while let Some(msg) = client_ws_receiver.next().await {
            match msg {
                Ok(Message::Binary(bytes)) => {
                    if tcp_writer.write_all(&bytes).await.is_err() {
                        break;
                    }
                }
                Ok(Message::Text(text)) => {
                    if tcp_writer.write_all(text.as_bytes()).await.is_err() {
                        break;
                    }
                }
                Ok(Message::Close(_)) => {
                    break;
                }
                _ => {}
            }
        }
        let _ = tcp_writer.shutdown().await;
    });

    // Task to read from TCP and write to WebSocket
    let mut tcp_to_ws = tokio::spawn(async move {
        let mut buf = vec![0u8; 8192];
        loop {
            match tcp_reader.read(&mut buf).await {
                Ok(0) => {
                    // EOF
                    break;
                }
                Ok(n) => {
                    if client_ws_sender
                        .send(Message::Binary(buf[..n].to_vec()))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                Err(_) => {
                    break;
                }
            }
        }
        let _ = client_ws_sender.close().await;
    });

    tokio::select! {
        _ = (&mut ws_to_tcp) => {
            tcp_to_ws.abort();
        }
        _ = (&mut tcp_to_ws) => {
            ws_to_tcp.abort();
        }
    }
}
