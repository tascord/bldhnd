use {
    crate::mb::ty::MinifiedRelease,
    milrouter::{
        hyper::{server::conn::http1, service::service_fn},
        hyper_util::rt::TokioIo,
        *,
    },
    reqwest::header::HeaderMap,
    serde::Serialize,
    std::{
        env,
        fs::{self, File},
        path::{Path, PathBuf},
    },
    tokio::{net::TcpListener, spawn},
    tracing::{info, warn},
    tracing_subscriber::{Layer, fmt, layer::SubscriberExt, util::SubscriberInitExt},
};

mod mb;
mod tm;
mod tv;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    setup_logging()?;

    info!("Started bldhnd server");

    let stale = fs::read_dir(working())?.collect::<Result<Vec<_>, _>>()?;
    if !stale.is_empty() {
        warn!(
            "Stale files in cache ({}): {}",
            working().display(),
            stale.iter().map(|s| s.file_name().display().to_string()).collect::<Vec<_>>().join(", ")
        );
    }

    spawn(async {
        let c = mb::client();
        let _ = c.fetch().await.inspect_err(|e| warn!("{e:?}"));
    });

    serve(Router::route).await
}

fn setup_logging() -> anyhow::Result<()> {
    // rotate previous LATEST -> timestamped file
    let logs_dir = logs();
    if !logs_dir.exists() {
        fs::create_dir_all(&logs_dir)?;
    }

    let latest = logs_dir.join("LATEST");
    if latest.exists() {
        let meta = fs::metadata(&latest)?;
        // prefer creation time, fall back to modification time
        let sys_time = meta.created().or_else(|_| meta.modified()).unwrap_or(std::time::SystemTime::now());

        let dt: chrono::DateTime<chrono::Local> = sys_time.into();
        let name = format!("{}.log", dt.format("%Y%m%dT%H%M%S%z"));
        let target = logs_dir.join(name);
        // ignore error if rename fails for some reason
        let _ = fs::rename(&latest, &target);
    }

    // create new LATEST log file
    let file = File::create(&latest)?;

    let stdout_layer = fmt::layer()
        .with_file(true)
        .with_level(true)
        .with_target(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .with_ansi(true)
        .pretty()
        .with_filter(tracing_subscriber::filter::LevelFilter::INFO);

    let file_layer = fmt::layer()
        .with_file(true)
        .with_level(true)
        .with_target(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .with_ansi(false)
        .with_writer(move || file.try_clone().expect("failed to clone log file"))
        .compact()
        .with_filter(tracing_subscriber::filter::LevelFilter::INFO);

    tracing_subscriber::registry().with(stdout_layer).with(file_layer).init();

    Ok(())
}

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
    async fn search(&self, q: &str, p: usize) -> anyhow::Result<Vec<Self::Output>>;
}

async fn auth(_: HeaderMap) -> anyhow::Result<()> { Ok(()) }

#[derive(Router)]
pub enum Router {
    Music(EndpointMusic),
}

#[endpoint(auth = auth)]
async fn music(q: (String, usize)) -> anyhow::Result<Vec<MinifiedRelease>> { mb::client().search(&q.0, q.1).await }

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
