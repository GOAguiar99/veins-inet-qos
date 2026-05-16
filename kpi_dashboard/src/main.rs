use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use anyhow::{Context, Result};
use axum::extract::{Query, State};
use axum::response::Html;
use axum::routing::get;
use axum::{Json, Router};
use clap::Parser;
use kpi_dashboard::{
    build_dashboard_response, default_results_dir, load_startup_dataset, rebuild_raw_dataset,
    DashboardDataset, DashboardResponse, RebuildStatus, INDEX_HTML,
};
use serde::Deserialize;

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Fast local KPI tables for Veins QoS OMNeT++ results"
)]
struct Cli {
    #[arg(long)]
    results: Option<PathBuf>,

    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    #[arg(long, default_value_t = 8050)]
    port: u16,

    #[arg(long, default_value = "plain_netload_high")]
    baseline: String,

    #[arg(long)]
    rebuild: bool,

    #[arg(long)]
    threads: Option<usize>,
}

#[derive(Clone)]
struct AppState {
    dataset: Arc<RwLock<DashboardDataset>>,
    rebuild_status: Arc<RwLock<RebuildStatus>>,
    baseline_default: String,
    results_dir: PathBuf,
    threads: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct DashboardQuery {
    baseline: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();
    let results_dir = args.results.unwrap_or_else(default_results_dir).resolve();

    let startup = load_startup_dataset(&results_dir, args.rebuild, args.threads)
        .with_context(|| format!("failed to load KPI dataset from {}", results_dir.display()))?;

    let state = AppState {
        dataset: Arc::new(RwLock::new(startup.dataset)),
        rebuild_status: Arc::new(RwLock::new(RebuildStatus::idle())),
        baseline_default: args.baseline,
        results_dir: results_dir.clone(),
        threads: args.threads,
    };

    if startup.spawn_rebuild {
        spawn_rebuild(state.clone());
    }

    let app = Router::new()
        .route("/", get(index))
        .route("/api/dashboard", get(api_dashboard))
        .with_state(Arc::new(state));

    let addr: SocketAddr = format!("{}:{}", args.host, args.port)
        .parse()
        .context("invalid host/port")?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("Veins QoS KPI dashboard: http://{}", addr);
    axum::serve(listener, app).await?;
    Ok(())
}

async fn index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

async fn api_dashboard(
    State(state): State<Arc<AppState>>,
    Query(query): Query<DashboardQuery>,
) -> Json<DashboardResponse> {
    let baseline = query
        .baseline
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| state.baseline_default.clone());
    let dataset = state.dataset.read().expect("dataset lock poisoned").clone();
    let rebuild = state
        .rebuild_status
        .read()
        .expect("rebuild lock poisoned")
        .clone();
    Json(build_dashboard_response(&dataset, &baseline, rebuild))
}

fn spawn_rebuild(state: AppState) {
    {
        let mut status = state.rebuild_status.write().expect("rebuild lock poisoned");
        *status = RebuildStatus::running("Rebuilding Rust cache from raw .sca/.vec files");
    }

    tokio::task::spawn_blocking(move || {
        let result = rebuild_raw_dataset(&state.results_dir, state.threads);
        match result {
            Ok(dataset) => {
                {
                    let mut current = state.dataset.write().expect("dataset lock poisoned");
                    *current = dataset;
                }
                let mut status = state.rebuild_status.write().expect("rebuild lock poisoned");
                *status = RebuildStatus::finished("Rust cache rebuilt from raw result files");
            }
            Err(error) => {
                let mut status = state.rebuild_status.write().expect("rebuild lock poisoned");
                *status = RebuildStatus::failed(format!("Rust cache rebuild failed: {error:#}"));
            }
        }
    });
}

trait ResolvePath {
    fn resolve(self) -> PathBuf;
}

impl ResolvePath for PathBuf {
    fn resolve(self) -> PathBuf {
        self.canonicalize().unwrap_or(self)
    }
}
