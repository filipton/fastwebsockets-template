use crate::structs::WsMessage;
use anyhow::Result;
use fastwebsockets::upgrade::upgrade;
use fastwebsockets::{FragmentCollector, OpCode, WebSocketError};
use hyper::server::conn::Http;
use hyper::service::service_fn;
use hyper::upgrade::Upgraded;
use hyper::{Body, Request, Response};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use structs::{SharedState, State};
use tokio::net::TcpListener;
use tokio::sync::RwLock;

mod structs;

async fn handle_ws(
    mut ws: FragmentCollector<Upgraded>,
    client_addr: SocketAddr,
    state: &SharedState,
) -> Result<(), WebSocketError> {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    {
        let mut state = state.write().await;
        state.clients.insert(client_addr, tx);
    }

    loop {
        tokio::select! {
            frame = ws.read_frame() => {
                let frame = frame?;

                match frame.opcode {
                    OpCode::Close => {
                        //println!("Closing connection...");
                        break;
                    }
                    OpCode::Text => {
                        let text = String::from_utf8(frame.payload.to_vec()).unwrap();
                        state.read().await.broadcast(&client_addr, WsMessage::Text(text)).await;
                        //ws.write_frame(frame).await?;
                    }
                    OpCode::Binary => {
                        state.read().await.broadcast(&client_addr, WsMessage::Binary(frame.payload.to_vec())).await;
                        //ws.write_frame(frame).await?;
                    }
                    _ => {}
                }
            },
            frame = rx.recv() => {
                if let Some(frame) = frame {
                    ws.write_frame(frame.to_frame()).await?;
                } else {
                    break;
                }
            }
        }
    }

    Ok(())
}

async fn request_handler(
    mut req: Request<Body>,
    client_addr: SocketAddr,
    state: SharedState,
) -> Result<Response<Body>> {
    let uri = req.uri().path();

    match uri {
        "/ws" => {
            let (response, fut) = upgrade(&mut req)?;

            tokio::spawn(async move {
                let ws = fastwebsockets::FragmentCollector::new(fut.await.unwrap());
                handle_ws(ws, client_addr, &state).await.unwrap();

                {
                    let mut state = state.write().await;
                    state.clients.remove(&client_addr);
                }
            });

            return Ok(response);
        }
        _ => {
            let resp = Response::builder().status(404).body("Not found".into())?;
            return Ok(resp);
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    let listener = TcpListener::bind(addr).await?;

    println!("Listening on {}", addr);

    let state = Arc::new(RwLock::new(State {
        clients: HashMap::new(),
    }));

    loop {
        let (stream, client_addr) = listener.accept().await?;
        let state = state.clone();

        tokio::task::spawn(async move {
            if let Err(err) = Http::new()
                .serve_connection(
                    stream,
                    service_fn(move |req| request_handler(req, client_addr, state.clone())),
                )
                .with_upgrades()
                .await
            {
                println!("Error serving connection: {:?}", err);
            }
        });
    }
}
