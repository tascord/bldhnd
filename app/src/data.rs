use {
    bh_server::{RouterClient, mb::ty::MinifiedRelease, wikidata::ty::WikiDataItem},
    chrono::NaiveDate,
    dashmap::DashMap,
    std::{
        env,
        hash::Hash,
        sync::{Arc, LazyLock},
        time::{Duration, Instant},
    },
};

// =========================================================
// Search Result
// =========================================================

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub name: String,
    pub release: NaiveDate,
    pub ty: String,
    pub size_gb: f32,
}

impl SearchResult {
    pub fn gb_fmt(&self) -> String { format!("{:.1}Gb", self.size_gb) }

    pub fn rel_fmt(&self) -> String { self.release.format("%b %G").to_string() }

    pub fn ty_fmt(&self) -> String {
        match self.ty.as_str() {
            "Music" => "♫ Music",
            oth => oth,
        }
        .to_string()
    }
}

// =========================================================
// Simple TTL Cache
// =========================================================

#[derive(Clone)]
struct CacheEntry<V> {
    value: V,
    expires_at: Instant,
}

#[derive(Clone)]
pub struct Cache<K: Eq + Hash, V> {
    inner: DashMap<K, CacheEntry<V>>,
    ttl: Duration,
}

impl<K, V> Cache<K, V>
where
    K: std::hash::Hash + Eq + Clone,
    V: Clone,
{
    pub fn new(ttl: Duration) -> Self { Self { inner: DashMap::new(), ttl } }

    pub fn get(&self, key: &K) -> Option<V> {
        let entry = self.inner.get(key)?;
        if Instant::now() > entry.expires_at {
            drop(entry);
            self.inner.remove(key);
            return None;
        }
        Some(entry.value.clone())
    }

    pub fn insert(&self, key: K, value: V) {
        self.inner.insert(key, CacheEntry { value, expires_at: Instant::now() + self.ttl });
    }
}

static DATA_CLIENT: LazyLock<Arc<RouterClient>> = LazyLock::new(|| {
    bh_server::Router::client(
        env::var("BLDHND_SERVER_URL").unwrap_or("https://bldhnd.fargone.sh".to_string()),
        Default::default(),
    )
    .into()
});

pub fn data() -> Arc<RouterClient> { DATA_CLIENT.clone() }

impl From<MinifiedRelease> for SearchResult {
    fn from(value: MinifiedRelease) -> Self {
        // Build display name as "Title — Artist"
        let name = if value.primary_artist.is_empty() {
            value.title.clone()
        } else {
            format!("{} — {}", value.title, value.primary_artist)
        };

        // Parse release date (try common granularities), fallback to 1970-01-01
        let release = value
            .release_date
            .as_deref()
            .and_then(|s| {
                chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
                    .or_else(|_| chrono::NaiveDate::parse_from_str(s, "%Y-%m"))
                    .or_else(|_| chrono::NaiveDate::parse_from_str(s, "%Y"))
                    .ok()
            })
            .unwrap_or_else(|| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap());

        // Approximate size from track count (assume ~10MB per track)
        let size_gb = (value.total_tracks as f32) * 0.01;

        SearchResult { name, release, ty: "Music".to_string(), size_gb }
    }
}

impl From<WikiDataItem> for SearchResult {
    fn from(value: WikiDataItem) -> Self {
        let name = value.title.clone();

        let release = value
            .release_date
            .as_deref()
            .and_then(|s| {
                chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
                    .or_else(|_| chrono::NaiveDate::parse_from_str(s, "%Y-%m"))
                    .or_else(|_| chrono::NaiveDate::parse_from_str(s, "%Y"))
                    .ok()
            })
            .unwrap_or_else(|| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap());

        let ty = match value.media_type.as_str() {
            "film" => "Movie".to_string(),
            "tv" => "Series".to_string(),
            other => other.to_string(),
        };

        // WikiData items have no size info in the KB; leave as 0.0
        SearchResult { name, release, ty, size_gb: 0.0 }
    }
}
