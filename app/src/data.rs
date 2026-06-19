use {
    crate::config,
    async_trait::async_trait,
    chrono::NaiveDate,
    dashmap::DashMap,
    reqwest::Client,
    std::{
        fmt::Display,
        hash::Hash,
        sync::{Arc, LazyLock, RwLock},
        time::{Duration, Instant},
    },
    zeroize::Zeroizing,
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

// =========================================================
// Trait
// =========================================================

#[async_trait]
pub trait KnowledgeBase: Send + Sync {
    async fn login(&self) -> anyhow::Result<()>;
    async fn search(&self, q: &str) -> anyhow::Result<Vec<SearchResult>>;
}

// =========================================================
// MusicBrainz
// =========================================================

pub struct MusicBrainz {
    key: Zeroizing<String>,
    http: Client,
    cache: Cache<String, Vec<SearchResult>>,
}

impl MusicBrainz {
    pub fn new() -> Self {
        Self {
            key: Zeroizing::new(config().read().unwrap().key_mb.clone()),
            http: Client::new(),
            cache: Cache::new(Duration::from_secs(60 * 10)),
        }
    }
}

#[async_trait]
impl KnowledgeBase for MusicBrainz {
    async fn login(&self) -> anyhow::Result<()> { Ok(()) }

    async fn search(&self, q: &str) -> anyhow::Result<Vec<SearchResult>> {
        let query = q.to_string();

        if let Some(cached) = self.cache.get(&query) {
            return Ok(cached);
        }

        let url = format!("https://musicbrainz.org/ws/2/release/?query={}&fmt=json", urlencoding::encode(&query));

        let resp = self
            .http
            .get(url)
            .header("User-Agent", "kb-client/1.0 (contact@example.com)")
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        let mut results = Vec::new();

        if let Some(releases) = resp["releases"].as_array() {
            for r in releases {
                results.push(SearchResult {
                    name: r["title"].as_str().unwrap_or("").to_string(),
                    release: NaiveDate::from_ymd_opt(2000, 1, 1).unwrap(),
                    ty: "Music".into(),
                    size_gb: 0.0,
                });
            }
        }

        self.cache.insert(query, results.clone());
        Ok(results)
    }
}

// =========================================================
// TMDB
// =========================================================

pub struct TMDB {
    key: Zeroizing<String>,
    http: Client,
    cache: Cache<String, Vec<SearchResult>>,
    token: RwLock<Option<String>>,
}

impl TMDB {
    pub fn new() -> Self {
        Self {
            key: Zeroizing::new(config().read().unwrap().key_tm.clone()),
            http: Client::new(),
            cache: Cache::new(Duration::from_secs(60 * 10)),
            token: RwLock::new(None),
        }
    }
}

#[async_trait]
impl KnowledgeBase for TMDB {
    async fn login(&self) -> anyhow::Result<()> {
        let url = "https://api.themoviedb.org/3/authentication";

        let resp = self.http.get(url).bearer_auth(self.key.as_str()).send().await?;

        if resp.status().is_success() {
            *self.token.write().unwrap() = Some(self.key.to_string());
        }

        Ok(())
    }

    async fn search(&self, q: &str) -> anyhow::Result<Vec<SearchResult>> {
        let query = q.to_string();

        if let Some(cached) = self.cache.get(&query) {
            return Ok(cached);
        }

        let url = format!("https://api.themoviedb.org/3/search/multi?query={}", urlencoding::encode(&query));

        let resp = self.http.get(url).bearer_auth(self.key.as_str()).send().await?.json::<serde_json::Value>().await?;

        let mut results = Vec::new();

        if let Some(items) = resp["results"].as_array() {
            for i in items {
                results.push(SearchResult {
                    name: i["title"].as_str().or_else(|| i["name"].as_str()).unwrap_or("").to_string(),
                    release: NaiveDate::from_ymd_opt(2000, 1, 1).unwrap(),
                    ty: format!("Music | {}", i["media_type"].as_str().unwrap_or("unknown")),
                    size_gb: 0.0,
                });
            }
        }

        self.cache.insert(query, results.clone());
        Ok(results)
    }
}

// =========================================================
// TVDB
// =========================================================

pub struct TVDB {
    key: Zeroizing<String>,
    http: Client,
    cache: Cache<String, Vec<SearchResult>>,
    token: RwLock<Option<String>>,
}

impl TVDB {
    pub fn new() -> Self {
        Self {
            key: Zeroizing::new(config().read().unwrap().key_tv.clone()),
            http: Client::new(),
            cache: Cache::new(Duration::from_secs(60 * 10)),
            token: RwLock::new(None),
        }
    }
}

#[async_trait]
impl KnowledgeBase for TVDB {
    async fn login(&self) -> anyhow::Result<()> {
        let url = "https://api4.thetvdb.com/v4/login";

        let resp = self
            .http
            .post(url)
            .json(&serde_json::json!({
                "apikey": self.key.as_str()
            }))
            .send()
            .await?;

        if resp.status().is_success() {
            let json: serde_json::Value = resp.json().await?;
            if let Some(token) = json["data"]["token"].as_str() {
                *self.token.write().unwrap() = Some(token.to_string());
            }
        }

        Ok(())
    }

    async fn search(&self, q: &str) -> anyhow::Result<Vec<SearchResult>> {
        let query = q.to_string();

        if let Some(cached) = self.cache.get(&query) {
            return Ok(cached);
        }

        let token = self.token.read().unwrap().clone();

        let mut req = self.http.get("https://api4.thetvdb.com/v4/search").query(&[("query", &query)]);

        if let Some(t) = token {
            req = req.bearer_auth(t);
        }

        let resp = req.send().await?.json::<serde_json::Value>().await?;

        let mut results = Vec::new();

        if let Some(items) = resp["data"].as_array() {
            for i in items {
                results.push(SearchResult {
                    name: i["name"].as_str().unwrap_or("").to_string(),
                    release: NaiveDate::from_ymd_opt(2000, 1, 1).unwrap(),
                    ty: "TV".into(),
                    size_gb: 0.0,
                });
            }
        }

        self.cache.insert(query, results.clone());
        Ok(results)
    }
}

static MB_CLIENT: LazyLock<Arc<MusicBrainz>> = LazyLock::new(|| Arc::new(MusicBrainz::new()));
static TM_CLIENT: LazyLock<Arc<TMDB>> = LazyLock::new(|| Arc::new(TMDB::new()));
static TV_CLIENT: LazyLock<Arc<TVDB>> = LazyLock::new(|| Arc::new(TVDB::new()));

pub fn mb() -> Arc<MusicBrainz> { MB_CLIENT.clone() }

pub fn tm() -> Arc<TMDB> { TM_CLIENT.clone() }

pub fn tv() -> Arc<TVDB> { TV_CLIENT.clone() }
