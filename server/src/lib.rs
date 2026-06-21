use {
    crate::mb::ty::MinifiedRelease,
    milrouter::{
        hyper::{server::conn::http1, service::service_fn},
        hyper_util::rt::TokioIo,
        *,
    },
    serde::Serialize,
    std::{
        env,
        fs::{self},
        path::{Path, PathBuf},
    },
    tokio::{net::TcpListener, spawn},
    tracing::{info, warn},
};

pub mod mb;
pub mod tm;
pub mod tv;

pub fn working() -> PathBuf {
    let p = Path::new(&env::home_dir().expect("No home dir")).join(".bldhnd/").join("cache/");
    if !p.exists() {
        fs::create_dir_all(&p).expect("Failed to create 'working' folder.");
    }

    p
}

pub fn logs() -> PathBuf {
    let p = Path::new(&env::home_dir().expect("No home dir")).join(".bldhnd/").join("logs/");
    if !p.exists() {
        fs::create_dir_all(&p).expect("Failed to create 'working' folder.");
    }

    p
}

pub fn db() -> PathBuf {
    let p = Path::new(&env::home_dir().expect("No home dir")).join(".bldhnd/").join("dbs/");
    if !p.exists() {
        fs::create_dir_all(&p).expect("Failed to create 'db' folder.");
    }

    p
}

#[allow(async_fn_in_trait)]
pub trait KnowledgeBase {
    type Output: Serialize;
    async fn fetch(&self) -> anyhow::Result<()>;
    fn search(&self, q: &str, p: usize) -> anyhow::Result<Vec<Self::Output>>;
    fn stats(&self) -> anyhow::Result<usize>;
}

pub async fn serve<RouteFut>(route: fn(hyper::Request<hyper::body::Incoming>) -> RouteFut) -> anyhow::Result<()>
where
    RouteFut: Future<Output = std::result::Result<hyper::Response<http_body_util::Full<bytes::Bytes>>, std::convert::Infallible>>
        + Sync
        + Send
        + 'static,
{
    let addr: std::net::SocketAddr = format!("127.0.0.1:{}", env::var("PORT").unwrap_or("40000".to_string())).parse()?;
    let listener = TcpListener::bind(addr).await?;

    info!("Listening on http://{}", addr);

    while let Ok((stream, _)) = listener.accept().await {
        let service = service_fn(route);
        let io = TokioIo::new(stream);

        spawn(async move {
            if let Err(err) = http1::Builder::new().serve_connection(io, service).await {
                warn!("Error serving connection: {:?}", err);
            }
        });
    }

    Ok(())
}
