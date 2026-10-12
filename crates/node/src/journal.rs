//! PAPER trade journal. Local only: a SQLite file in the wallet dir.
//! Never on the ledger or the network; it records notes the user types in.
//! Positions use FIFO lots; realized P&L = proceeds - FIFO cost - fees.

use rusqlite::{params, Connection};
use std::collections::{BTreeMap, VecDeque};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Buy,
    Sell,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Trade {
    pub id: i64,
    pub time: u64,
    pub side: Side,
    pub asset: String,
    pub qty: f64,
    pub price: f64,
    pub fee: f64,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Position {
    pub qty: f64,
    pub avg_cost: f64,
    pub realized: f64,
}

pub struct Journal {
    db: Connection,
}

impl Journal {
    pub fn open(dir: &Path) -> rusqlite::Result<Self> {
        Self::init(Connection::open(dir.join("paper_journal.db"))?)
    }
    #[cfg(test)]
    pub fn memory() -> rusqlite::Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }
    fn init(db: Connection) -> rusqlite::Result<Self> {
        db.execute_batch(
            "CREATE TABLE IF NOT EXISTS paper_trades(id INTEGER PRIMARY KEY AUTOINCREMENT, time INTEGER NOT NULL,
               side TEXT NOT NULL, asset TEXT NOT NULL, qty REAL NOT NULL, price REAL NOT NULL,
               fee REAL NOT NULL DEFAULT 0, note TEXT NOT NULL DEFAULT '');",
        )?;
        Ok(Self { db })
    }
    #[allow(clippy::too_many_arguments)]
    pub fn add(
        &self,
        time: u64,
        side: Side,
        asset: &str,
        qty: f64,
        price: f64,
        fee: f64,
        note: &str,
    ) -> Result<i64, String> {
        if !(qty > 0.0 && price >= 0.0 && fee >= 0.0) || asset.trim().is_empty() {
            return Err("qty must be > 0, price and fee >= 0, asset required".into());
        }
        let asset = asset.trim().to_uppercase();
        if side == Side::Sell {
            let held = self
                .positions()
                .map_err(|e| e.to_string())?
                .get(&asset)
                .map(|p| p.qty)
                .unwrap_or(0.0);
            if qty > held + 1e-12 {
                return Err(format!(
                    "cannot sell {qty} {asset}: paper position is {held}"
                ));
            }
        }
        self.db
            .execute(
                "INSERT INTO paper_trades(time,side,asset,qty,price,fee,note) VALUES(?1,?2,?3,?4,?5,?6,?7)",
                params![time as i64, if side == Side::Buy { "buy" } else { "sell" }, asset, qty, price, fee, note],
            )
            .map_err(|e| e.to_string())?;
        Ok(self.db.last_insert_rowid())
    }
    pub fn trades(&self) -> rusqlite::Result<Vec<Trade>> {
        let mut st = self.db.prepare(
            "SELECT id,time,side,asset,qty,price,fee,note FROM paper_trades ORDER BY id",
        )?;
        let rows = st.query_map([], |r| {
            Ok(Trade {
                id: r.get(0)?,
                time: r.get::<_, i64>(1)? as u64,
                side: if r.get::<_, String>(2)? == "buy" {
                    Side::Buy
                } else {
                    Side::Sell
                },
                asset: r.get(3)?,
                qty: r.get(4)?,
                price: r.get(5)?,
                fee: r.get(6)?,
                note: r.get(7)?,
            })
        })?;
        rows.collect()
    }
    /// FIFO positions per asset.
    pub fn positions(&self) -> rusqlite::Result<BTreeMap<String, Position>> {
        Ok(fifo(&self.trades()?))
    }
    pub fn csv(&self) -> rusqlite::Result<String> {
        let mut out = String::from("id,time,side,asset,qty,price,fee,note\n");
        for t in self.trades()? {
            let note = t.note.replace('"', "\"\"");
            out.push_str(&format!(
                "{},{},{},{},{},{},{},\"{}\"\n",
                t.id,
                t.time,
                if t.side == Side::Buy { "buy" } else { "sell" },
                t.asset,
                t.qty,
                t.price,
                t.fee,
                note
            ));
        }
        Ok(out)
    }
}

/// FIFO lot accounting. Buy fees are added to lot cost; sell fees reduce realized P&L.
pub fn fifo(trades: &[Trade]) -> BTreeMap<String, Position> {
    let mut lots: BTreeMap<String, VecDeque<(f64, f64)>> = BTreeMap::new(); // (qty, unit cost)
    let mut realized: BTreeMap<String, f64> = BTreeMap::new();
    for t in trades {
        let q = lots.entry(t.asset.clone()).or_default();
        match t.side {
            Side::Buy => q.push_back((t.qty, (t.qty * t.price + t.fee) / t.qty)),
            Side::Sell => {
                let mut left = t.qty;
                let mut cost = 0.0;
                while left > 1e-12 {
                    let Some(front) = q.front_mut() else { break };
                    let take = left.min(front.0);
                    cost += take * front.1;
                    front.0 -= take;
                    left -= take;
                    if front.0 <= 1e-12 {
                        q.pop_front();
                    }
                }
                *realized.entry(t.asset.clone()).or_default() += t.qty * t.price - t.fee - cost;
            }
        }
    }
    lots.into_iter()
        .map(|(a, q)| {
            let qty: f64 = q.iter().map(|l| l.0).sum();
            let cost: f64 = q.iter().map(|l| l.0 * l.1).sum();
            let avg = if qty > 1e-12 { cost / qty } else { 0.0 };
            let r = *realized.get(&a).unwrap_or(&0.0);
            (
                a,
                Position {
                    qty,
                    avg_cost: avg,
                    realized: r,
                },
            )
        })
        .collect()
}

/// Unrealized P&L at a manual current price.
pub fn unrealized(p: &Position, current_price: f64) -> f64 {
    p.qty * (current_price - p.avg_cost)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn fifo_realized_and_avg_cost() {
        let j = Journal::memory().unwrap();
        j.add(1, Side::Buy, "paxg", 1.0, 100.0, 0.0, "").unwrap();
        j.add(2, Side::Buy, "PAXG", 1.0, 200.0, 0.0, "").unwrap();
        j.add(3, Side::Sell, "PAXG", 1.5, 300.0, 0.0, "").unwrap();
        let p = &j.positions().unwrap()["PAXG"];
        // FIFO: sold 1@100 + 0.5@200 = 200 cost; proceeds 450 -> realized 250
        assert!(close(p.realized, 250.0));
        assert!(close(p.qty, 0.5));
        assert!(close(p.avg_cost, 200.0));
        assert!(close(unrealized(p, 250.0), 25.0));
    }
    #[test]
    fn fees_counted() {
        let j = Journal::memory().unwrap();
        j.add(1, Side::Buy, "XAU", 2.0, 10.0, 2.0, "").unwrap(); // unit cost 11
        j.add(2, Side::Sell, "XAU", 2.0, 12.0, 1.0, "").unwrap(); // 24 - 1 - 22 = 1
        let p = &j.positions().unwrap()["XAU"];
        assert!(close(p.realized, 1.0));
        assert!(close(p.qty, 0.0));
    }
    #[test]
    fn cannot_oversell_and_csv() {
        let j = Journal::memory().unwrap();
        j.add(1, Side::Buy, "ETH", 1.0, 5.0, 0.0, "say \"hi\"")
            .unwrap();
        assert!(j.add(2, Side::Sell, "ETH", 2.0, 5.0, 0.0, "").is_err());
        assert!(j.add(2, Side::Buy, "ETH", 0.0, 5.0, 0.0, "").is_err());
        let csv = j.csv().unwrap();
        assert!(csv.starts_with("id,time,side"));
        assert!(csv.contains("\"say \"\"hi\"\"\""));
    }
}
