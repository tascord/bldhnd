use std::{
    env,
    fs::create_dir_all,
    ops::Deref,
    path::{Path, PathBuf},
    sync::{Arc, LazyLock},
};

use crate::{config, events::EventTarget};
use futures_signals::signal::Mutable;

static LIBRARY: LazyLock<Arc<Library>> = LazyLock::new(|| Arc::new(Library::new()));

pub fn library() -> Arc<Library> {
    LIBRARY.clone()
}

pub fn working() -> PathBuf {
    let p = Path::new(&env::home_dir().expect("No home dir")).join(".cache/").join("bldhnd");
    create_dir_all(&p).expect("Failed to create working dir");
    p
}

#[derive(Debug)]
pub enum LibraryEvent {
    ScanStarted,
    ScanCompleted,
    FoundEntry { volume_idx: usize, path: String },
}

#[derive(Debug)]
pub enum File {
    Movie { title: String, size_gb: f32 },
    Music { title: String, size_gb: f32 },
    Series { title: String, size_gb: f32 },
}

#[derive(Debug, Clone)]
pub struct VolumeStats {
    pub path: PathBuf,
    pub size_bytes: u64,
    pub file_count: u64,
}

impl VolumeStats {
    fn new(path: PathBuf) -> Self {
        Self { path, size_bytes: 0, file_count: 0 }
    }

    fn add_file(&mut self, size: u64) {
        self.size_bytes += size;
        self.file_count += 1;
    }

    pub fn size_gb(&self) -> f32 {
        self.size_bytes as f32 / (1024.0 * 1024.0 * 1024.0)
    }
}

impl File {
    pub fn title(&self) -> String {
        match self {
            File::Movie { title, .. } => title,
            File::Music { title, .. } => title,
            File::Series { title, .. } => title,
        }
        .to_string()
    }

    pub fn ty(&self) -> String {
        match self {
            File::Movie { .. } => "Movie",
            File::Music { .. } => "Music",
            File::Series { .. } => "Series",
        }
        .to_string()
    }

    pub fn size_gb(&self) -> f32 {
        match self {
            File::Movie { size_gb, .. } => *size_gb,
            File::Music { size_gb, .. } => *size_gb,
            File::Series { size_gb, .. } => *size_gb,
        }
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct Library {
    ev: EventTarget<LibraryEvent>,
    files: Mutable<Vec<Arc<File>>>,
    scanning: Mutable<bool>,
    volume_stats: Mutable<Vec<VolumeStats>>,
}

impl Deref for Library {
    type Target = EventTarget<LibraryEvent>;

    fn deref(&self) -> &Self::Target {
        &self.ev
    }
}

#[allow(clippy::new_without_default)]
impl Library {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            scanning: Mutable::new(false),
            ev: EventTarget::new(),
            files: Mutable::new(Vec::new()),
            volume_stats: Mutable::new(Vec::new()),
        }
    }

    pub fn volume_stats(&self) -> Vec<VolumeStats> {
        self.volume_stats.lock_ref().clone()
    }

    pub fn can_download_to_volume(volume_idx: usize, size_bytes: u64) -> bool {
        let binding = config();
        let c = binding.lock_ref();
        let stats = library().volume_stats();

        let Some(volume) = c.volumes.get(volume_idx) else {
            return false;
        };

        let Some(max_gb) = volume.max_size_gb else {
            return true;
        };

        let max_bytes = (max_gb * 1024.0 * 1024.0 * 1024.0) as u64;
        let Some(current) = stats.get(volume_idx) else {
            return true;
        };

        current.size_bytes + size_bytes <= max_bytes
    }

    #[allow(dead_code)]
    pub fn scan(&self) {
        if self.scanning.get() {
            return;
        }
        self.scanning.set(true);
        self.ev.emit(LibraryEvent::ScanStarted);

        let lib = library();
        let ev = self.ev.clone();

        std::thread::spawn(move || {
            let c = config().get_cloned();

            let mut all_stats = Vec::new();

            for (idx, v) in c.volumes.iter().enumerate() {
                let mut stats = VolumeStats::new(PathBuf::from(&v.path));
                let path = Path::new(&v.path);

                if path.is_dir() {
                    let entries = walkdir::WalkDir::new(path).into_iter().filter_map(|e| e.ok());
                        for entry in entries {
                            if entry.file_type().is_file() {
                                if let Ok(metadata) = entry.metadata() {
                                    stats.add_file(metadata.len());
                                    ev.emit(LibraryEvent::FoundEntry {
                                        volume_idx: idx,
                                        path: entry.path().display().to_string(),
                                    });
                                }
                            }
                        }
                }

                all_stats.push(stats);
            }

            {
                lib.volume_stats.set(all_stats);
                lib.scanning.set(false);
            }

            ev.emit(LibraryEvent::ScanCompleted);
        });
    }
}