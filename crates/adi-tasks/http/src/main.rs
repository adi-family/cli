use adi_tasks_core::{CreateTask, TaskId, TaskManager};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, put},
    Json, Router,
};
use lib_http_common::version_header_layer;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

struct AppState {
    tasks: RwLock<Option<TaskManager>>,
    #[allow(dead_code)]
    project_path: Option<PathBuf>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8081);

    let project_path = std::env::var("PROJECT_PATH").ok().map(PathBuf::from);

    let tasks = if let Some(ref path) = project_path {
        TaskManager::open(path).ok()
    } else {
        TaskManager::open_global().ok()
    };

    let state = Arc::new(AppState {
        tasks: RwLock::new(tasks),
        project_path,
    });

    let app = Router::new()
        // Health
        .route("/", get(health))
        .route("/health", get(health))
        .route("/status", get(status))
        // Tasks CRUD
        .route("/tasks", get(list_tasks).post(create_task))
        .route(
            "/tasks/:id",
            get(get_task).put(update_task).delete(delete_task),
        )
        .route("/tasks/:id/status", put(update_status))
        // Dependencies
        .route(
            "/tasks/:id/dependencies",
            get(get_dependencies).post(add_dependency),
        )
        .route("/tasks/:id/dependencies/:dep_id", delete(remove_dependency))
        .route("/tasks/:id/dependents", get(get_dependents))
        // Queries
        .route("/tasks/search", get(search_tasks))
        .route("/tasks/ready", get(get_ready_tasks))
        .route("/tasks/blocked", get(get_blocked_tasks))
        // Graph
        .route("/graph", get(get_graph))
        .route("/graph/cycles", get(detect_cycles))
        // Symbol linking
        .route("/tasks/:id/link/:symbol_id", put(link_to_symbol))
        .route("/tasks/:id/link", delete(unlink_symbol))
        .with_state(state)
        .layer(version_header_layer(
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
        ))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive());

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .unwrap();

    tracing::info!("ADI Tasks HTTP server listening on port {}", port);

    axum::serve(listener, app).await.unwrap();
}

// --- Health & Status ---

async fn health() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok", "service": "adi-tasks-http" }))
}

async fn status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let tasks = state.tasks.read().await;
    match tasks.as_ref() {
        Some(t) => match t.status() {
            Ok(status) => (StatusCode::OK, Json(serde_json::json!(status))),
            Err(e) => error_response(e),
        },
        None => unavailable_response(),
    }
}

// --- Task CRUD ---

#[derive(Deserialize)]
struct ListQuery {
    status: Option<String>,
}

async fn list_tasks(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListQuery>,
) -> impl IntoResponse {
    let tasks = state.tasks.read().await;
    match tasks.as_ref() {
        Some(t) => {
            let result = if let Some(status_str) = query.status {
                match status_str.parse() {
                    Ok(status) => t.get_by_status(status),
                    Err(_) => {
                        return (
                            StatusCode::BAD_REQUEST,
                            Json(serde_json::json!({ "error": "Invalid status" })),
                        );
                    }
                }
            } else {
                t.list()
            };

            match result {
                Ok(tasks) => (StatusCode::OK, Json(serde_json::json!(tasks))),
                Err(e) => error_response(e),
            }
        }
        None => unavailable_response(),
    }
}

#[derive(Deserialize)]
struct CreateTaskInput {
    title: String,
    description: Option<String>,
    depends_on: Option<Vec<i64>>,
    symbol_id: Option<i64>,
}

async fn create_task(
    State(state): State<Arc<AppState>>,
    Json(input): Json<CreateTaskInput>,
) -> impl IntoResponse {
    let tasks = state.tasks.read().await;
    match tasks.as_ref() {
        Some(t) => {
            let mut create_input = CreateTask::new(&input.title);
            create_input.description = input.description;
            create_input.symbol_id = input.symbol_id;

            if let Some(deps) = input.depends_on {
                create_input =
                    create_input.with_dependencies(deps.into_iter().map(TaskId).collect());
            }

            match t.create_task(create_input) {
                Ok(id) => (StatusCode::CREATED, Json(serde_json::json!({ "id": id.0 }))),
                Err(e) => error_response(e),
            }
        }
        None => unavailable_response(),
    }
}

async fn get_task(State(state): State<Arc<AppState>>, Path(id): Path<i64>) -> impl IntoResponse {
    let tasks = state.tasks.read().await;
    match tasks.as_ref() {
        Some(t) => match t.get_task_with_dependencies(TaskId(id)) {
            Ok(task) => (StatusCode::OK, Json(serde_json::json!(task))),
            Err(e) => error_response(e),
        },
        None => unavailable_response(),
    }
}

#[derive(Deserialize)]
struct UpdateTaskInput {
    title: Option<String>,
    description: Option<String>,
    status: Option<String>,
    symbol_id: Option<i64>,
}

async fn update_task(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(input): Json<UpdateTaskInput>,
) -> impl IntoResponse {
    let tasks = state.tasks.read().await;
    match tasks.as_ref() {
        Some(t) => {
            let mut task = match t.get_task(TaskId(id)) {
                Ok(task) => task,
                Err(e) => return error_response(e),
            };

            if let Some(title) = input.title {
                task.title = title;
            }
            if let Some(description) = input.description {
                task.description = Some(description);
            }
            if let Some(status_str) = input.status {
                match status_str.parse() {
                    Ok(status) => task.status = status,
                    Err(_) => {
                        return (
                            StatusCode::BAD_REQUEST,
                            Json(serde_json::json!({ "error": "Invalid status" })),
                        );
                    }
                }
            }
            if let Some(symbol_id) = input.symbol_id {
                task.symbol_id = Some(symbol_id);
            }

            match t.update_task(&task) {
                Ok(()) => (StatusCode::OK, Json(serde_json::json!(task))),
                Err(e) => error_response(e),
            }
        }
        None => unavailable_response(),
    }
}

async fn delete_task(State(state): State<Arc<AppState>>, Path(id): Path<i64>) -> impl IntoResponse {
    let tasks = state.tasks.read().await;
    match tasks.as_ref() {
        Some(t) => match t.delete_task(TaskId(id)) {
            Ok(()) => (StatusCode::OK, Json(serde_json::json!({ "deleted": id }))),
            Err(e) => error_response(e),
        },
        None => unavailable_response(),
    }
}

#[derive(Deserialize)]
struct UpdateStatusInput {
    status: String,
}

async fn update_status(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(input): Json<UpdateStatusInput>,
) -> impl IntoResponse {
    let tasks = state.tasks.read().await;
    match tasks.as_ref() {
        Some(t) => {
            let status = match input.status.parse() {
                Ok(s) => s,
                Err(_) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(serde_json::json!({ "error": "Invalid status" })),
                    );
                }
            };

            match t.update_status(TaskId(id), status) {
                Ok(()) => (
                    StatusCode::OK,
                    Json(serde_json::json!({ "id": id, "status": input.status })),
                ),
                Err(e) => error_response(e),
            }
        }
        None => unavailable_response(),
    }
}

// --- Dependencies ---

async fn get_dependencies(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    let tasks = state.tasks.read().await;
    match tasks.as_ref() {
        Some(t) => match t.get_dependencies(TaskId(id)) {
            Ok(deps) => (StatusCode::OK, Json(serde_json::json!(deps))),
            Err(e) => error_response(e),
        },
        None => unavailable_response(),
    }
}

#[derive(Deserialize)]
struct AddDependencyInput {
    depends_on: i64,
}

async fn add_dependency(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(input): Json<AddDependencyInput>,
) -> impl IntoResponse {
    let tasks = state.tasks.read().await;
    match tasks.as_ref() {
        Some(t) => match t.add_dependency(TaskId(id), TaskId(input.depends_on)) {
            Ok(()) => (
                StatusCode::CREATED,
                Json(serde_json::json!({ "from": id, "to": input.depends_on })),
            ),
            Err(e) => error_response(e),
        },
        None => unavailable_response(),
    }
}

async fn remove_dependency(
    State(state): State<Arc<AppState>>,
    Path((id, dep_id)): Path<(i64, i64)>,
) -> impl IntoResponse {
    let tasks = state.tasks.read().await;
    match tasks.as_ref() {
        Some(t) => match t.remove_dependency(TaskId(id), TaskId(dep_id)) {
            Ok(()) => (StatusCode::OK, Json(serde_json::json!({ "removed": true }))),
            Err(e) => error_response(e),
        },
        None => unavailable_response(),
    }
}

async fn get_dependents(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    let tasks = state.tasks.read().await;
    match tasks.as_ref() {
        Some(t) => match t.get_dependents(TaskId(id)) {
            Ok(deps) => (StatusCode::OK, Json(serde_json::json!(deps))),
            Err(e) => error_response(e),
        },
        None => unavailable_response(),
    }
}

// --- Queries ---

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
    #[serde(default = "default_limit")]
    limit: usize,
}

fn default_limit() -> usize {
    10
}

async fn search_tasks(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SearchQuery>,
) -> impl IntoResponse {
    let tasks = state.tasks.read().await;
    match tasks.as_ref() {
        Some(t) => match t.search(&query.q, query.limit) {
            Ok(results) => (StatusCode::OK, Json(serde_json::json!(results))),
            Err(e) => error_response(e),
        },
        None => unavailable_response(),
    }
}

async fn get_ready_tasks(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let tasks = state.tasks.read().await;
    match tasks.as_ref() {
        Some(t) => match t.get_ready() {
            Ok(ready) => (StatusCode::OK, Json(serde_json::json!(ready))),
            Err(e) => error_response(e),
        },
        None => unavailable_response(),
    }
}

async fn get_blocked_tasks(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let tasks = state.tasks.read().await;
    match tasks.as_ref() {
        Some(t) => match t.get_blocked() {
            Ok(blocked) => (StatusCode::OK, Json(serde_json::json!(blocked))),
            Err(e) => error_response(e),
        },
        None => unavailable_response(),
    }
}

// --- Graph ---

#[derive(Serialize)]
struct GraphNode {
    task: adi_tasks_core::Task,
    dependencies: Vec<i64>,
}

async fn get_graph(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let tasks = state.tasks.read().await;
    match tasks.as_ref() {
        Some(t) => {
            let all_tasks = match t.list() {
                Ok(tasks) => tasks,
                Err(e) => return error_response(e),
            };

            let mut graph = Vec::new();
            for task in all_tasks {
                let deps = match t.get_dependencies(task.id) {
                    Ok(deps) => deps.iter().map(|d| d.id.0).collect(),
                    Err(_) => vec![],
                };
                graph.push(GraphNode {
                    task,
                    dependencies: deps,
                });
            }

            (StatusCode::OK, Json(serde_json::json!(graph)))
        }
        None => unavailable_response(),
    }
}

async fn detect_cycles(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let tasks = state.tasks.read().await;
    match tasks.as_ref() {
        Some(t) => match t.detect_cycles() {
            Ok(cycles) => {
                let cycle_ids: Vec<Vec<i64>> = cycles
                    .into_iter()
                    .map(|c| c.into_iter().map(|id| id.0).collect())
                    .collect();
                (
                    StatusCode::OK,
                    Json(serde_json::json!({ "cycles": cycle_ids })),
                )
            }
            Err(e) => error_response(e),
        },
        None => unavailable_response(),
    }
}

// --- Symbol Linking ---

async fn link_to_symbol(
    State(state): State<Arc<AppState>>,
    Path((id, symbol_id)): Path<(i64, i64)>,
) -> impl IntoResponse {
    let tasks = state.tasks.read().await;
    match tasks.as_ref() {
        Some(t) => match t.link_to_symbol(TaskId(id), symbol_id) {
            Ok(()) => (
                StatusCode::OK,
                Json(serde_json::json!({ "task_id": id, "symbol_id": symbol_id })),
            ),
            Err(e) => error_response(e),
        },
        None => unavailable_response(),
    }
}

async fn unlink_symbol(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    let tasks = state.tasks.read().await;
    match tasks.as_ref() {
        Some(t) => match t.unlink_symbol(TaskId(id)) {
            Ok(()) => (
                StatusCode::OK,
                Json(serde_json::json!({ "task_id": id, "unlinked": true })),
            ),
            Err(e) => error_response(e),
        },
        None => unavailable_response(),
    }
}

// --- Helper Functions ---

fn error_response(e: adi_tasks_core::Error) -> (StatusCode, Json<serde_json::Value>) {
    let status = match &e {
        adi_tasks_core::Error::TaskNotFound(_) => StatusCode::NOT_FOUND,
        adi_tasks_core::Error::DependencyNotFound { .. } => StatusCode::NOT_FOUND,
        adi_tasks_core::Error::WouldCreateCycle { .. } => StatusCode::CONFLICT,
        adi_tasks_core::Error::SelfDependency(_) => StatusCode::BAD_REQUEST,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };

    (status, Json(serde_json::json!({ "error": e.to_string() })))
}

fn unavailable_response() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(serde_json::json!({ "error": "Tasks not initialized" })),
    )
}
