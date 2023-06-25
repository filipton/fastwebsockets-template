use fastwebsockets::upgrade::upgrade;
use fastwebsockets::{FragmentCollector, OpCode};
use hyper::server::conn::Http;
use hyper::service::service_fn;
use hyper::upgrade::Upgraded;
use hyper::{Body, Request, Response};
use std::net::SocketAddr;
use tokio::net::TcpListener;

async fn handle_ws(
    mut ws: FragmentCollector<Upgraded>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    loop {
        let frame = ws.read_frame().await?;
        match frame.opcode {
            OpCode::Close => {
                //println!("Closing connection...");
                break;
            }
            OpCode::Text | OpCode::Binary => {
                ws.write_frame(frame).await?;
            }
            _ => {}
        }
    }

    Ok(())
}

async fn request_handler(
    mut req: Request<Body>,
) -> Result<Response<Body>, Box<dyn std::error::Error + Send + Sync>> {
    let uri = req.uri().to_string();
    let uri = uri.as_str();

    match uri {
        "/ws" => {
            let (response, fut) = upgrade(&mut req)?;

            tokio::spawn(async move {
                let ws = fastwebsockets::FragmentCollector::new(fut.await.unwrap());
                handle_ws(ws).await.unwrap();
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
    let addr = SocketAddr::from(([0, 0, 0, 0], 1234));
    let listener = TcpListener::bind(addr).await?;

    loop {
        let (stream, _) = listener.accept().await?;

        tokio::task::spawn(async move {
            if let Err(err) = Http::new()
                .serve_connection(stream, service_fn(move |req| request_handler(req)))
                .with_upgrades()
                .await
            {
                println!("Error serving connection: {:?}", err);
            }
        });
    }
}

