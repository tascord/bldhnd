use {
    redb::{Database, ReadableDatabase, TableDefinition},
    serde::{Deserialize, Serialize},
    std::path::PathBuf,
};

pub static USERS_DB: std::sync::LazyLock<std::sync::Arc<Database>> = std::sync::LazyLock::new(|| {
    let dir = users_dir();
    if !dir.exists() {
        std::fs::create_dir_all(&dir).expect("Failed to create users dir");
    }
    let db_path = dir.join("users.db");
    std::sync::Arc::new(Database::create(&db_path).expect("Failed to open users db"))
});

pub fn users_dir() -> PathBuf {
    let p = if let Ok(b) = std::env::var("BLDHND_DIR") {
        PathBuf::from(b).join("users")
    } else if let Ok(x) = std::env::var("XDG_DATA_HOME") {
        PathBuf::from(x).join("bldhnd").join("users")
    } else {
        std::env::home_dir().expect("No home dir").join(".local/share/").join("bldhnd").join("users")
    };
    if !p.exists() {
        std::fs::create_dir_all(&p).expect("Failed to create users dir");
    }
    p
}

const USERS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("users");
const SESSIONS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("sessions");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: u64,
    pub username: String,
    pub password_hash: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub user_id: u64,
    pub token: String,
    pub created_at: i64,
    pub expires_at: i64,
}

pub struct UserManager;

impl UserManager {
    pub fn new() -> Self { UserManager }

    pub fn create_user(&self, username: &str, password: &str) -> anyhow::Result<u64> {
        if username.is_empty() || password.is_empty() {
            return Err(anyhow::anyhow!("Username and password required"));
        }

        if self.get_by_username(username)?.is_some() {
            return Err(anyhow::anyhow!("User already exists"));
        }

        let id = rand_id();
        let password_hash = hash_password(password);

        let db = &*USERS_DB;
        let mut tx = db.begin_write()?;
        {
            let mut table = tx.open_table(USERS_TABLE)?;
            let user =
                User { id, username: username.to_string(), password_hash, created_at: chrono::Utc::now().timestamp() };
            let json = serde_json::to_string(&user)?;
            table.insert(username.as_bytes(), json.as_str())?;
        }
        tx.commit()?;
        Ok(id)
    }

    pub fn get_by_username(&self, username: &str) -> anyhow::Result<Option<User>> {
        let db = &*USERS_DB;
        let tx = db.begin_read()?;
        let table = tx.open_table(USERS_TABLE)?;
        if let Ok(Some(value)) = table.get(username.as_bytes()) {
            let user: User = serde_json::from_str(value.value())?;
            Ok(Some(user))
        } else {
            Ok(None)
        }
    }

    pub fn verify_password(&self, username: &str, password: &str) -> anyhow::Result<Option<User>> {
        if let Some(user) = self.get_by_username(username)? {
            if verify_hash(password, &user.password_hash) {
                return Ok(Some(user));
            }
        }
        Ok(None)
    }

    pub fn create_session(&self, user_id: u64) -> anyhow::Result<String> {
        let token = generate_token();
        let session = Session {
            user_id,
            token: token.clone(),
            created_at: chrono::Utc::now().timestamp(),
            expires_at: chrono::Utc::now().timestamp() + 86400 * 7,
        };
        let db = &*USERS_DB;
        let mut tx = db.begin_write()?;
        {
            let mut table = tx.open_table(SESSIONS_TABLE)?;
            let json = serde_json::to_string(&session)?;
            table.insert(token.as_bytes(), json.as_str())?;
        }
        tx.commit()?;
        Ok(token)
    }

    pub fn get_session(&self, token: &str) -> anyhow::Result<Option<Session>> {
        let db = &*USERS_DB;
        let tx = db.begin_read()?;
        let table = tx.open_table(SESSIONS_TABLE)?;
        if let Ok(Some(value)) = table.get(token.as_bytes()) {
            let session: Session = serde_json::from_str(value.value())?;
            if session.expires_at > chrono::Utc::now().timestamp() {
                return Ok(Some(session));
            }
        }
        Ok(None)
    }

    pub fn delete_session(&self, token: &str) -> anyhow::Result<()> {
        let db = &*USERS_DB;
        let mut tx = db.begin_write()?;
        {
            let mut table = tx.open_table(SESSIONS_TABLE)?;
            table.remove(token.as_bytes())?;
        }
        tx.commit()?;
        Ok(())
    }
}

impl Default for UserManager {
    fn default() -> Self { Self::new() }
}

fn hash_password(password: &str) -> String {
    use std::{
        collections::hash_map::DefaultHasher,
        hash::{Hash, Hasher},
    };
    let mut hasher = DefaultHasher::new();
    password.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

fn verify_hash(password: &str, hash: &str) -> bool { hash_password(password) == hash }

fn generate_token() -> String {
    use std::io::Read;
    let mut file = std::fs::File::open("/dev/urandom").unwrap();
    let mut buf = [0u8; 32];
    file.read_exact(&mut buf).unwrap();
    buf.iter().map(|b| format!("{:02x}", b)).collect()
}

fn rand_id() -> u64 {
    use std::io::Read;
    let mut file = std::fs::File::open("/dev/urandom").unwrap();
    let mut buf = [0u8; 8];
    file.read_exact(&mut buf).unwrap();
    u64::from_le_bytes(buf)
}
