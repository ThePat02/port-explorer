use crate::{localisator, signatures::load_signatures};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};
use warp::ws::{Message, WebSocket};
use warp::Filter;

#[derive(Debug, Deserialize)]
pub struct ScanRequest {
    pub ip: String,
    pub start_port: u16,
    pub end_port: u16,
    pub max_threads: usize,
}

#[derive(Debug, Serialize)]
pub struct ScanResponse {
    pub success: bool,
    pub message: String,
    pub scan_id: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct ProgressUpdate {
    pub scan_id: String,
    pub current: u64,
    pub total: u64,
    pub percentage: f64,
}

#[derive(Debug, Serialize, Clone)]
pub struct ScanResult {
    pub scan_id: String,
    pub open_ports: Vec<PortResult>,
    pub duration: String,
    pub completed: bool,
}

#[derive(Debug, Serialize, Clone)]
pub struct PortResult {
    pub port: u16,
    pub service: Option<String>,
}

// Global state to track active scans
type ScanState = Arc<Mutex<HashMap<String, broadcast::Sender<String>>>>;

pub async fn start_gui_server(port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let scan_state: ScanState = Arc::new(Mutex::new(HashMap::new()));

    // Static file serving
    let static_files = warp::path("static")
        .and(warp::fs::dir("static"));

    // Main page
    let index = warp::path::end()
        .map(|| {
            let html = std::fs::read_to_string("static/index.html")
                .unwrap_or_else(|_| "Error: index.html not found".to_string());
            warp::reply::html(html)
        });

    // API routes
    let scan_state_filter = warp::any().map(move || scan_state.clone());

    let start_scan = warp::path("api")
        .and(warp::path("scan"))
        .and(warp::path("start"))
        .and(warp::post())
        .and(warp::body::json())
        .and(scan_state_filter.clone())
        .and_then(handle_start_scan);

    let websocket = warp::path("ws")
        .and(warp::ws())
        .and(scan_state_filter.clone())
        .map(|ws: warp::ws::Ws, state| {
            ws.on_upgrade(move |socket| handle_websocket(socket, state))
        });

    let routes = static_files
        .or(index)
        .or(start_scan)
        .or(websocket)
        .with(warp::cors().allow_any_origin().allow_headers(vec!["content-type"]).allow_methods(vec!["GET", "POST"]));

    println!("Starting GUI server on http://localhost:{}", port);
    warp::serve(routes)
        .run(([127, 0, 0, 1], port))
        .await;

    Ok(())
}

async fn handle_start_scan(
    request: ScanRequest,
    scan_state: ScanState,
) -> Result<impl warp::Reply, warp::Rejection> {
    // Validate the request
    let ip = match IpAddr::from_str(&request.ip) {
        Ok(ip) => ip,
        Err(_) => {
            return Ok(warp::reply::json(&ScanResponse {
                success: false,
                message: localisator::get("error_invalid_ip"),
                scan_id: None,
            }));
        }
    };

    if request.start_port == 0 || request.start_port > 65535 || request.end_port == 0 || request.end_port > 65535 {
        return Ok(warp::reply::json(&ScanResponse {
            success: false,
            message: "Invalid port range".to_string(),
            scan_id: None,
        }));
    }

    if request.start_port > request.end_port {
        return Ok(warp::reply::json(&ScanResponse {
            success: false,
            message: localisator::get("error_start_gt_end"),
            scan_id: None,
        }));
    }

    if request.max_threads == 0 || request.max_threads > 1000 {
        return Ok(warp::reply::json(&ScanResponse {
            success: false,
            message: "Invalid thread count".to_string(),
            scan_id: None,
        }));
    }

    let scan_id = format!("scan_{}", chrono::Utc::now().timestamp_millis());
    let (tx, _rx) = broadcast::channel(1000);
    
    {
        let mut state = scan_state.lock().await;
        state.insert(scan_id.clone(), tx.clone());
    }

    // Spawn the scan task
    let scan_id_clone = scan_id.clone();
    let tx_clone = tx.clone();
    tokio::spawn(async move {
        run_scan_async(ip, request.start_port, request.end_port, request.max_threads, scan_id_clone, tx_clone).await;
    });

    Ok(warp::reply::json(&ScanResponse {
        success: true,
        message: "Scan started".to_string(),
        scan_id: Some(scan_id),
    }))
}

async fn run_scan_async(
    ip: IpAddr,
    start_port: u16,
    end_port: u16,
    max_threads: usize,
    scan_id: String,
    tx: broadcast::Sender<String>,
) {
    let scan_start = std::time::Instant::now();
    
    let signatures = match load_signatures() {
        Ok(sigs) => Arc::new(sigs),
        Err(_e) => {
            let error_msg = serde_json::to_string(&ScanResult {
                scan_id: scan_id.clone(),
                open_ports: vec![],
                duration: "0s".to_string(),
                completed: false,
            }).unwrap();
            let _ = tx.send(error_msg);
            return;
        }
    };

    let ports: Vec<u16> = (start_port..=end_port).collect();
    let total_ports = ports.len() as u64;

    // Custom progress tracking for GUI
    struct GuiProgressBar {
        scan_id: String,
        tx: broadcast::Sender<String>,
        current: Arc<Mutex<u64>>,
        total: u64,
    }

    impl GuiProgressBar {
        fn new(scan_id: String, tx: broadcast::Sender<String>, total: u64) -> Self {
            Self {
                scan_id,
                tx,
                current: Arc::new(Mutex::new(0)),
                total,
            }
        }

        async fn inc(&self, delta: u64) {
            let mut current = self.current.lock().await;
            *current += delta;
            let progress = ProgressUpdate {
                scan_id: self.scan_id.clone(),
                current: *current,
                total: self.total,
                percentage: (*current as f64 / self.total as f64) * 100.0,
            };
            if let Ok(msg) = serde_json::to_string(&progress) {
                let _ = self.tx.send(msg);
            }
        }
    }

    let gui_pb = GuiProgressBar::new(scan_id.clone(), tx.clone(), total_ports);

    // Run the scan using the existing parallel scanning logic
    let result = tokio::task::spawn_blocking({
        let ip = Arc::new(ip);
        let signatures = signatures.clone();
        let gui_pb = Arc::new(gui_pb);
        move || {
            // Create a simple progress tracker that works with our GUI
            use indicatif::{ProgressBar, ProgressStyle};
            let pb = ProgressBar::new(total_ports);
            pb.set_style(ProgressStyle::default_bar());
            
            // We need to bridge between the blocking scan and async GUI updates
            let rt = tokio::runtime::Handle::current();
            let gui_pb_clone = gui_pb.clone();
            
            // Override the progress callback in the existing scan function
            crate::scan_ports_parallel_with_callback(
                ip,
                ports,
                signatures,
                max_threads,
                Box::new(move |_| {
                    let gui_pb = gui_pb_clone.clone();
                    rt.spawn(async move {
                        gui_pb.inc(1).await;
                    });
                }),
            )
        }
    }).await;

    let scan_duration = scan_start.elapsed();
    let duration_str = crate::format_duration(scan_duration);

    let scan_result = match result {
        Ok(Ok(open_ports)) => {
            let port_results: Vec<PortResult> = open_ports
                .into_iter()
                .map(|(port, service)| PortResult { port, service })
                .collect();

            ScanResult {
                scan_id: scan_id.clone(),
                open_ports: port_results,
                duration: duration_str,
                completed: true,
            }
        }
        _ => ScanResult {
            scan_id: scan_id.clone(),
            open_ports: vec![],
            duration: duration_str,
            completed: false,
        }
    };

    if let Ok(msg) = serde_json::to_string(&scan_result) {
        let _ = tx.send(msg);
    }
}

async fn handle_websocket(ws: WebSocket, scan_state: ScanState) {
    let (ws_tx, mut ws_rx) = ws.split();
    let ws_tx = Arc::new(Mutex::new(ws_tx));

    while let Some(result) = ws_rx.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(_) => break,
        };

        if let Ok(text) = msg.to_str() {
            // Client is requesting to subscribe to a scan_id
            if text.starts_with("subscribe:") {
                let scan_id = text.trim_start_matches("subscribe:");
                let state = scan_state.lock().await;
                if let Some(tx) = state.get(scan_id) {
                    let mut rx = tx.subscribe();
                    let ws_tx = ws_tx.clone();
                    tokio::spawn(async move {
                        while let Ok(update) = rx.recv().await {
                            let mut ws_tx_guard = ws_tx.lock().await;
                            if ws_tx_guard.send(Message::text(update)).await.is_err() {
                                break;
                            }
                        }
                    });
                }
            }
        }
    }
}