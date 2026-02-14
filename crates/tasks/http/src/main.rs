mod generated;

use async_trait::async_trait;
use axum::{routing::get, Json, Router};
use generated::enums::TaskStatus;
use generated::models::*;
use generated::server::*;
use lib_http_common::version_header_layer;
use std::path::PathBuf;
use std::sync::Arc;
use tasks_core::{TaskId, TaskManager};
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use lib_env_parse::{env_vars, env_opt};

env_vars! {
    Port => "PORT",
    ProjectPath => "PROJECT_PATH",
}

struct AppState {
    tasks: RwLock<Option<TaskManager>>,
}

fn to_api_error(e: tasks_core::Error) -> ApiError {
    let (status, code) = match &e {
        tasks_core::Error::TaskNotFound(_) => (404, "not_found"),
        tasks_core::Error::DependencyNotFound { .. } => (404, "not_found"),
        tasks_core::Error::WouldCreateCycle { .. } => (409, "conflict"),
        tasks_core::Error::SelfDependency(_) => (400, "bad_request"),
        _ => (500, "internal_error"),
    };
    ApiError {
        status,
        code: code.to_string(),
        message: e.to_string(),
    }
}

fn unavailable() -> ApiError {
    ApiError {
        status: 503,
        code: "unavailable".to_string(),
        message: "Tasks not initialized".to_string(),
    }
}

fn task_to_model(t: &tasks_core::Task) -> Task {
    Task {
        id: t.id.0,
        title: t.title.clone(),
        description: t.description.clone(),
        status: match t.status {
            tasks_core::TaskStatus::Todo => TaskStatus::Todo,
            tasks_core::TaskStatus::InProgress => TaskStatus::InProgress,
            tasks_core::TaskStatus::Done => TaskStatus::Done,
            tasks_core::TaskStatus::Blocked => TaskStatus::Blocked,
            tasks_core::TaskStatus::Cancelled => TaskStatus::Cancelled,
        },
        symbol_id: t.symbol_id,
        project_path: t.project_path.as_ref().map(|p| p.to_string()),
        created_at: t.created_at,
        updated_at: t.updated_at,
    }
}

#[async_trait]
impl TaskServiceHandler for AppState {
    async fn list(&self, query: TaskServiceListQuery) -> Result<Vec<Task>, ApiError> {
        let tasks = self.tasks.read().await;
        let t = tasks.as_ref().ok_or_else(unavailable)?;

        let result = if let Some(status_str) = query.status {
            let status: tasks_core::TaskStatus = status_str
                .parse()
                .map_err(|_| ApiError {
                    status: 400,
                    code: "bad_request".to_string(),
                    message: "Invalid status".to_string(),
                })?;
            t.get_by_status(status).map_err(to_api_error)?
        } else {
            t.list().map_err(to_api_error)?
        };

        Ok(result.iter().map(task_to_model).collect())
    }

    async fn create(&self, body: CreateTaskInput) -> Result<IdResponse, ApiError> {
        let tasks = self.tasks.read().await;
        let t = tasks.as_ref().ok_or_else(unavailable)?;

        let mut create = tasks_core::CreateTask::new(&body.title);
        create.description = body.description;
        create.symbol_id = body.symbol_id;

        if let Some(deps) = body.depends_on {
            create = create.with_dependencies(deps.into_iter().map(TaskId).collect());
        }

        let id = t.create_task(create).map_err(to_api_error)?;
        Ok(IdResponse { id: id.0 })
    }

    async fn get(&self, id: i64) -> Result<TaskWithDependencies, ApiError> {
        let tasks = self.tasks.read().await;
        let t = tasks.as_ref().ok_or_else(unavailable)?;

        let result = t
            .get_task_with_dependencies(TaskId(id))
            .map_err(to_api_error)?;
        Ok(TaskWithDependencies {
            task: task_to_model(&result.task),
            depends_on: result.depends_on.iter().map(task_to_model).collect(),
            dependents: result.dependents.iter().map(task_to_model).collect(),
        })
    }

    async fn update(&self, id: i64, body: UpdateTaskInput) -> Result<Task, ApiError> {
        let tasks = self.tasks.read().await;
        let t = tasks.as_ref().ok_or_else(unavailable)?;

        let mut task = t.get_task(TaskId(id)).map_err(to_api_error)?;

        if let Some(title) = body.title {
            task.title = title;
        }
        if let Some(description) = body.description {
            task.description = Some(description);
        }
        if let Some(status_str) = body.status {
            task.status = status_str.parse().map_err(|_| ApiError {
                status: 400,
                code: "bad_request".to_string(),
                message: "Invalid status".to_string(),
            })?;
        }
        if let Some(symbol_id) = body.symbol_id {
            task.symbol_id = Some(symbol_id);
        }

        t.update_task(&task).map_err(to_api_error)?;
        Ok(task_to_model(&task))
    }

    async fn delete(&self, id: i64) -> Result<DeletedResponse, ApiError> {
        let tasks = self.tasks.read().await;
        let t = tasks.as_ref().ok_or_else(unavailable)?;
        t.delete_task(TaskId(id)).map_err(to_api_error)?;
        Ok(DeletedResponse { deleted: id })
    }

    async fn update_status(&self, id: i64, body: UpdateStatusInput) -> Result<Task, ApiError> {
        let tasks = self.tasks.read().await;
        let t = tasks.as_ref().ok_or_else(unavailable)?;

        let status: tasks_core::TaskStatus = body.status.parse().map_err(|_| ApiError {
            status: 400,
            code: "bad_request".to_string(),
            message: "Invalid status".to_string(),
        })?;

        t.update_status(TaskId(id), status).map_err(to_api_error)?;
        let task = t.get_task(TaskId(id)).map_err(to_api_error)?;
        Ok(task_to_model(&task))
    }

    async fn get_dependencies(&self, id: i64) -> Result<Vec<Task>, ApiError> {
        let tasks = self.tasks.read().await;
        let t = tasks.as_ref().ok_or_else(unavailable)?;
        let deps = t.get_dependencies(TaskId(id)).map_err(to_api_error)?;
        Ok(deps.iter().map(task_to_model).collect())
    }

    async fn add_dependency(&self, id: i64, body: AddDependencyInput) -> Result<DependencyResponse, ApiError> {
        let tasks = self.tasks.read().await;
        let t = tasks.as_ref().ok_or_else(unavailable)?;
        t.add_dependency(TaskId(id), TaskId(body.depends_on))
            .map_err(to_api_error)?;
        Ok(DependencyResponse {
            from: id,
            to: body.depends_on,
        })
    }

    async fn remove_dependency(&self, id: i64, dep_id: i64) -> Result<RemovedResponse, ApiError> {
        let tasks = self.tasks.read().await;
        let t = tasks.as_ref().ok_or_else(unavailable)?;
        t.remove_dependency(TaskId(id), TaskId(dep_id))
            .map_err(to_api_error)?;
        Ok(RemovedResponse { removed: true })
    }

    async fn get_dependents(&self, id: i64) -> Result<Vec<Task>, ApiError> {
        let tasks = self.tasks.read().await;
        let t = tasks.as_ref().ok_or_else(unavailable)?;
        let deps = t.get_dependents(TaskId(id)).map_err(to_api_error)?;
        Ok(deps.iter().map(task_to_model).collect())
    }

    async fn search(&self, query: TaskServiceSearchQuery) -> Result<Vec<Task>, ApiError> {
        let tasks = self.tasks.read().await;
        let t = tasks.as_ref().ok_or_else(unavailable)?;
        let limit = query.limit.map(|l| l as usize).unwrap_or(10);
        let results = t.search(&query.q, limit).map_err(to_api_error)?;
        Ok(results.iter().map(task_to_model).collect())
    }

    async fn get_ready(&self) -> Result<Vec<Task>, ApiError> {
        let tasks = self.tasks.read().await;
        let t = tasks.as_ref().ok_or_else(unavailable)?;
        let ready = t.get_ready().map_err(to_api_error)?;
        Ok(ready.iter().map(task_to_model).collect())
    }

    async fn get_blocked(&self) -> Result<Vec<Task>, ApiError> {
        let tasks = self.tasks.read().await;
        let t = tasks.as_ref().ok_or_else(unavailable)?;
        let blocked = t.get_blocked().map_err(to_api_error)?;
        Ok(blocked.iter().map(task_to_model).collect())
    }

    async fn link_to_symbol(&self, id: i64, symbol_id: i64) -> Result<LinkResponse, ApiError> {
        let tasks = self.tasks.read().await;
        let t = tasks.as_ref().ok_or_else(unavailable)?;
        t.link_to_symbol(TaskId(id), symbol_id)
            .map_err(to_api_error)?;
        Ok(LinkResponse {
            task_id: id,
            symbol_id,
        })
    }

    async fn unlink_symbol(&self, id: i64) -> Result<UnlinkResponse, ApiError> {
        let tasks = self.tasks.read().await;
        let t = tasks.as_ref().ok_or_else(unavailable)?;
        t.unlink_symbol(TaskId(id)).map_err(to_api_error)?;
        Ok(UnlinkResponse {
            task_id: id,
            unlinked: true,
        })
    }
}

#[async_trait]
impl GraphServiceHandler for AppState {
    async fn get_graph(&self) -> Result<Vec<GraphNode>, ApiError> {
        let tasks = self.tasks.read().await;
        let t = tasks.as_ref().ok_or_else(unavailable)?;

        let all_tasks = t.list().map_err(to_api_error)?;
        let mut graph = Vec::new();
        for task in &all_tasks {
            let deps = t
                .get_dependencies(task.id)
                .map(|deps| deps.iter().map(|d| d.id.0).collect())
                .unwrap_or_default();
            graph.push(GraphNode {
                task: task_to_model(task),
                dependencies: deps,
            });
        }
        Ok(graph)
    }

    async fn detect_cycles(&self) -> Result<CyclesResponse, ApiError> {
        let tasks = self.tasks.read().await;
        let t = tasks.as_ref().ok_or_else(unavailable)?;

        let cycles = t.detect_cycles().map_err(to_api_error)?;
        let cycle_ids: Vec<Vec<i64>> = cycles
            .into_iter()
            .map(|c| c.into_iter().map(|id| id.0).collect())
            .collect();
        Ok(CyclesResponse { cycles: cycle_ids })
    }
}

#[async_trait]
impl StatusServiceHandler for AppState {
    async fn get_status(&self) -> Result<TasksStatus, ApiError> {
        let tasks = self.tasks.read().await;
        let t = tasks.as_ref().ok_or_else(unavailable)?;

        let status = t.status().map_err(to_api_error)?;
        Ok(TasksStatus {
            total_tasks: status.total_tasks,
            todo_count: status.todo_count,
            in_progress_count: status.in_progress_count,
            done_count: status.done_count,
            blocked_count: status.blocked_count,
            cancelled_count: status.cancelled_count,
            total_dependencies: status.total_dependencies,
            has_cycles: status.has_cycles,
        })
    }
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok", "service": "adi-tasks-http" }))
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    let port = env_opt(EnvVar::Port.as_str())
        .and_then(|p| p.parse().ok())
        .unwrap_or(8081);

    let project_path = env_opt(EnvVar::ProjectPath.as_str()).map(PathBuf::from);

    let tasks = if let Some(ref path) = project_path {
        TaskManager::open(path).ok()
    } else {
        TaskManager::open_global().ok()
    };

    let state = Arc::new(AppState {
        tasks: RwLock::new(tasks),
    });

    let app = Router::new()
        .route("/", get(health))
        .route("/health", get(health))
        .merge(generated::server::create_router::<AppState>())
        .layer(version_header_layer(
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
        ))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .unwrap();

    tracing::info!("ADI Tasks HTTP server listening on port {}", port);

    axum::serve(listener, app).await.unwrap();
}
