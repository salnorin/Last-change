//! Append-only, signed, Merkle-committed local log stored in SQLite.
//! Every entry is a verified `Envelope`; duplicates are ignored.

use lc_core::{Envelope, Payload};
use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};

#[derive(Debug, thiserror::Error)]
pub enum LedgerError {
    #[error("sqlite: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("core: {0}")]
    Core(#[from] lc_core::CoreError),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("blocked author or content")]
    Blocked,
}

pub struct Ledger {
    db: Connection,
    pub min_stamp_bits: u32,
}

pub struct Entry {
    pub seq: i64,
    pub id: String,
    pub env: Envelope,
    pub payload: Payload,
}

impl Ledger {
    pub fn open(path: &std::path::Path, min_stamp_bits: u32) -> Result<Self, LedgerError> {
        Self::init(Connection::open(path)?, min_stamp_bits)
    }
    pub fn memory(min_stamp_bits: u32) -> Result<Self, LedgerError> {
        Self::init(Connection::open_in_memory()?, min_stamp_bits)
    }
    fn init(db: Connection, min_stamp_bits: u32) -> Result<Self, LedgerError> {
        db.execute_batch(
            "PRAGMA journal_mode=WAL;
             CREATE TABLE IF NOT EXISTS entries(seq INTEGER PRIMARY KEY AUTOINCREMENT, id TEXT UNIQUE NOT NULL,
               author TEXT NOT NULL, kind TEXT NOT NULL, env TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS blocklist(key TEXT PRIMARY KEY);",
        )?;
        Ok(Self { db, min_stamp_bits })
    }

    /// Verify and append. Returns Ok(true) if new, Ok(false) if already known.
    pub fn append(&self, env: &Envelope) -> Result<bool, LedgerError> {
        let p = env.verify(self.min_stamp_bits)?;
        let id = env.id();
        if self.is_blocked(&env.author)? || self.is_blocked(&env.author_pseudonym())? || self.is_blocked(&id)? {
            return Err(LedgerError::Blocked);
        }
        let kind = match p { Payload::Listing(_) => "listing", Payload::DealReceipt(_) => "receipt",
            Payload::Rating(_) => "rating", Payload::Report(_) => "report" };
        let n = self.db.execute("INSERT OR IGNORE INTO entries(id,author,kind,env) VALUES(?1,?2,?3,?4)",
            params![id, env.author, kind, serde_json::to_string(env)?])?;
        Ok(n == 1)
    }

    pub fn get(&self, id: &str) -> Result<Option<Envelope>, LedgerError> {
        let s: Option<String> = self.db.query_row("SELECT env FROM entries WHERE id=?1", [id], |r| r.get(0)).optional()?;
        Ok(match s { Some(s) => Some(serde_json::from_str(&s)?), None => None })
    }

    pub fn all(&self) -> Result<Vec<Entry>, LedgerError> {
        let mut st = self.db.prepare("SELECT seq,id,env FROM entries ORDER BY seq")?;
        let rows = st.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?)))?;
        let mut out = vec![];
        for row in rows {
            let (seq, id, s) = row?;
            let env: Envelope = serde_json::from_str(&s)?;
            let payload = env.payload()?;
            out.push(Entry { seq, id, env, payload });
        }
        Ok(out)
    }

    pub fn len(&self) -> Result<i64, LedgerError> {
        Ok(self.db.query_row("SELECT COUNT(*) FROM entries", [], |r| r.get(0))?)
    }
    pub fn is_empty(&self) -> Result<bool, LedgerError> { Ok(self.len()? == 0) }

    /// Merkle root over entry ids in insertion order (sha256, duplicate last on odd levels).
    pub fn merkle_root(&self) -> Result<String, LedgerError> {
        let mut level: Vec<[u8; 32]> = self.all()?.iter().map(|e| Sha256::digest(e.id.as_bytes()).into()).collect();
        if level.is_empty() { return Ok(hex::encode([0u8; 32])); }
        while level.len() > 1 {
            level = level.chunks(2).map(|c| {
                let mut h = Sha256::new();
                h.update(c[0]);
                h.update(c.get(1).unwrap_or(&c[0]));
                h.finalize().into()
            }).collect();
        }
        Ok(hex::encode(level[0]))
    }

    pub fn block(&self, key: &str) -> Result<(), LedgerError> {
        self.db.execute("INSERT OR IGNORE INTO blocklist(key) VALUES(?1)", [key])?;
        Ok(())
    }
    pub fn is_blocked(&self, key: &str) -> Result<bool, LedgerError> {
        Ok(self.db.query_row("SELECT COUNT(*) FROM blocklist WHERE key=?1", [key], |r| r.get::<_, i64>(0))? > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lc_core::*;
    fn l(t: &str) -> Payload {
        Payload::Listing(Listing { kind: ListingKind::Request, title: t.into(), desc: "".into(), price_hint: "".into(),
            category: "misc".into(), expiry: 0, image_hashes: vec![], created: 0 })
    }
    #[test]
    fn append_dedupe_root() {
        let led = Ledger::memory(4).unwrap();
        let id = Identity::generate();
        let a = id.seal(&l("a"), 4);
        let r0 = led.merkle_root().unwrap();
        assert!(led.append(&a).unwrap());
        assert!(!led.append(&a).unwrap());
        let r1 = led.merkle_root().unwrap();
        led.append(&id.seal(&l("b"), 4)).unwrap();
        assert_eq!(led.len().unwrap(), 2);
        assert_ne!(r0, r1);
        assert_ne!(r1, led.merkle_root().unwrap());
    }
    #[test]
    fn blocklist_and_persist() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("l.db");
        let id = Identity::generate();
        {
            let led = Ledger::open(&p, 4).unwrap();
            led.append(&id.seal(&l("x"), 4)).unwrap();
            led.block(&id.pseudonym()).unwrap();
            assert!(matches!(led.append(&id.seal(&l("y"), 4)), Err(LedgerError::Blocked)));
        }
        assert_eq!(Ledger::open(&p, 4).unwrap().len().unwrap(), 1);
    }
    #[test]
    fn rejects_weak_stamp() {
        let led = Ledger::memory(30).unwrap();
        assert!(led.append(&Identity::generate().seal(&l("a"), 0)).is_err());
    }
}
