use {
    crate::ipsea::{Client, SearchHit as IpseaSearchHit},
    chrono::NaiveDate,
    dashmap::DashMap,
    std::{
        hash::Hash,
        time::{Duration, Instant},
    },
};

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

pub fn data() -> Client { Client::connect() }

impl From<IpseaSearchHit> for SearchResult {
    fn from(value: IpseaSearchHit) -> Self {
        let name =
            if let Some(artist) = &value.artist { format!("{} - {}", value.title, artist) } else { value.title.clone() };

        let release = value
            .year
            .map(|y| NaiveDate::from_ymd_opt(y as i32, 1, 1).unwrap())
            .unwrap_or_else(|| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap());

        let size_gb = (value.size as f32) / 1_000_000_000.0;

        SearchResult { name, release, ty: "Music".to_string(), size_gb }
    }
}
