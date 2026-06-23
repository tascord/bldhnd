use std::{
    env,
    fs::create_dir_all,
    ops::Deref,
    path::{Path, PathBuf},
    sync::{
        Arc, LazyLock, RwLock,
        atomic::{AtomicBool, Ordering::SeqCst},
    },
};

use crate::{config, events::EventTarget};

static LIBRARY: LazyLock<Arc<RwLock<Library>>> = LazyLock::new(|| Arc::new(RwLock::new(Library::new())));

pub fn library() -> Arc<RwLock<Library>> {
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
    ScanCommpleted,
    FoundEntry,
}

#[derive(Debug)]
pub enum File {
    Movie { title: String, size_gb: f32 },
    Music { title: String, size_gb: f32 },
    Series { title: String, size_gb: f32 },
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
    files: RwLock<Vec<Arc<File>>>,
    scanning: AtomicBool,
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
        Self { scanning: AtomicBool::new(false), ev: EventTarget::new(), files: Default::default() }
    }

    #[allow(dead_code)]
    fn scan() {
        let lock = library();
        let lock = lock.write().unwrap();

        if lock.scanning.load(SeqCst) {
            return;
        };
        lock.emit(LibraryEvent::ScanStarted);

        std::thread::spawn(|| {
            let c = config();
            let c = c.read().unwrap().clone();

            for _v in c.volumes {
                // Scan volume
            }
        });
    }
}
