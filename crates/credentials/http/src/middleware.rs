use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use lib_analytics_core::AnalyticsEvent;
use std::time::Instant;

use crate::AppState;

pub async fn analytics_middleware(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Response {
    let start = Instant::now();
    let method = req.method().to_string();
    let path = req.uri().path().to_string();
    let user_id = req.extensions().get::<uuid::Uuid>().copied();

    let response = next.run(req).await;

    let duration_ms = start.elapsed().as_millis() as i64;
    let status_code = response.status().as_u16();

    state.analytics.track(AnalyticsEvent::ApiRequest {
        service: "adi-credentials".to_string(),
        endpoint: path,
        method,
        status_code,
        duration_ms,
        user_id,
    });

    response
}
