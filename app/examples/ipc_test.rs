use bldhnd::ipsea::{Client, Request, Response};

fn main() {
    let c = Client::connect();

    match c.get_config() {
        Ok(cfg) => println!("GetConfig OK: volumes={} dl_dir={:?}", cfg.volumes.len(), cfg.download_dir),
        Err(e) => println!("GetConfig FAIL: {e:#}"),
    }

    for mt in ["Music", "Movie"] {
        match c.search("radiohead", mt) {
            Ok(hits) => {
                println!("{mt}: {} hits", hits.len());
                for h in hits.iter().take(3) {
                    println!("  - [{}] {} ({:?}) {:?}", h.backend, h.title, h.artist, h.year);
                }
                if hits.is_empty() {
                    println!("  (empty result set)");
                }
            }
            Err(e) => println!("{mt} FAIL: {e:#}"),
        }
    }

    let _ = (Request::ListDownloads {}, Response::CancelDownload);
}
