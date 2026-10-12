//! Local block explorer: lists blocks and shows circuit-pow details for each header.
use anyhow::Result;
use axum::{
    extract::{Path, State},
    response::Html,
    routing::get,
    Json, Router,
};
use lc_chain::LocalChain;
use serde_json::{json, Value};
use std::path::{Path as FsPath, PathBuf};
use tower_http::services::ServeDir;

#[derive(Clone)]
struct St {
    dir: PathBuf,
}

fn chain(st: &St) -> Result<LocalChain, String> {
    LocalChain::load_or_init(&st.dir).map_err(|e| e.to_string())
}

async fn page() -> Html<&'static str> {
    Html(include_str!("../../../web/explorer.html"))
}

async fn blocks(State(st): State<St>) -> Result<Json<Value>, (axum::http::StatusCode, String)> {
    let c = chain(&st).map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let list: Vec<Value> = c
        .blocks
        .iter()
        .enumerate()
        .map(|(i, b)| {
            json!({
                "height": i,
                "hash": b.header.hash(),
                "time": b.header.time,
                "bits": b.header.bits,
                "value": b.coinbase.value,
            })
        })
        .collect();
    Ok(Json(json!({
        "genesis_hash": c.genesis_hash,
        "tip_hash": c.tip_hash,
        "height": c.height,
        "blocks": list,
    })))
}

async fn block(
    State(st): State<St>,
    Path(hash): Path<String>,
) -> Result<Json<Value>, (axum::http::StatusCode, String)> {
    let c = chain(&st).map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let b = c
        .block_by_hash(&hash)
        .ok_or((axum::http::StatusCode::NOT_FOUND, "unknown block".into()))?;
    let cv = b.header.circuit_view();
    Ok(Json(json!({
        "hash": b.header.hash(),
        "header": &b.header,
        "coinbase": &b.coinbase,
        "circuit": cv,
    })))
}

pub async fn serve(dir: &FsPath, http: &str) -> Result<()> {
    let st = St {
        dir: dir.to_path_buf(),
    };
    let app = Router::new()
        .route("/", get(page))
        .route("/blocks", get(blocks))
        .route("/api/blocks", get(blocks))
        .route("/block/:hash", get(block))
        .route("/api/block/:hash", get(block))
        .nest_service("/css", ServeDir::new("web/css"))
        .with_state(st);
    let tcp = tokio::net::TcpListener::bind(http).await?;
    println!("explorer http://{}", tcp.local_addr()?);
    axum::serve(tcp, app).await?;
    Ok(())
}
