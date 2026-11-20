use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use axum::extract::{Query, State};
use axum::response::Html;
use axum::routing::get;
use axum::{Json, Router};
use clap::Parser;
use kpi_dashboard::{
    build_dashboard_response, build_distribution_response, discover_results_packages,
    load_startup_dataset, DashboardDataset, DashboardResponse, DensityOption,
    DistributionResponse, INDEX_HTML,
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
    datasets: Arc<HashMap<String, DashboardDataset>>,
    density_options: Vec<DensityOption>,
    default_density: String,
    baseline_default: String,
}

#[derive(Debug, Deserialize)]
struct DashboardQuery {
    density: Option<String>,
    baseline: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();
    let packages = discover_results_packages(args.results.map(|path| path.resolve()));
    if packages.is_empty() {
        anyhow::bail!("no results directories found; pass --results or populate veins_inet_highway_*/results");
    }

    let mut datasets = HashMap::new();
    for package in &packages {
        let dataset = load_startup_dataset(&package.path, args.rebuild, args.threads)
            .with_context(|| {
                format!(
                    "failed to load KPI dataset from {}",
                    package.path.display()
                )
            })?;
        datasets.insert(package.id.clone(), dataset);
    }

    let density_options: Vec<DensityOption> = packages
        .iter()
        .map(|package| DensityOption {
            id: package.id.clone(),
            label: package.label.clone(),
        })
        .collect();
    let default_density = packages
        .first()
        .map(|package| package.id.clone())
        .expect("packages checked non-empty");

    let state = AppState {
        datasets: Arc::new(datasets),
        density_options: density_options.clone(),
        default_density,
        baseline_default: args.baseline,
    };

    let addr: SocketAddr = format!("{}:{}", args.host, args.port)
        .parse()
        .context("invalid host/port")?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("Veins QoS KPI dashboard: http://{}", addr);
    for option in &density_options {
        println!("  - {} ({})", option.label, option.id);
    }

    let app = Router::new()
        .route("/", get(index))
        .route("/api/dashboard", get(api_dashboard))
        .route("/api/distributions", get(api_distributions))
        .with_state(Arc::new(state));
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
    let density = query
        .density
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| state.default_density.clone());
    let density = if state.datasets.contains_key(&density) {
        density
    } else {
        state.default_density.clone()
    };

    let baseline = query
        .baseline
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| state.baseline_default.clone());

    let dataset = state
        .datasets
        .get(&density)
        .expect("density validated above");

    Json(build_dashboard_response(
        dataset,
        &density,
        &state.density_options,
        &baseline,
    ))
}

async fn api_distributions(
    State(state): State<Arc<AppState>>,
    Query(query): Query<DashboardQuery>,
) -> Json<DistributionResponse> {
    let density = query
        .density
        .filter(|value| state.datasets.contains_key(value))
        .unwrap_or_else(|| state.default_density.clone());
    let dataset = state
        .datasets
        .get(&density)
        .expect("density validated above");
    Json(build_distribution_response(dataset, &density))
}

trait ResolvePath {
    fn resolve(self) -> PathBuf;
}

impl ResolvePath for PathBuf {
    fn resolve(self) -> PathBuf {
        self.canonicalize().unwrap_or(self)
    }
}
