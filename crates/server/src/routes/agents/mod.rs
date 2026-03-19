pub mod control;
pub mod registry;
pub mod status;

use axum::{
    Router,
    routing::{get, post},
};

use crate::DeploymentImpl;

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    let agent_id_router = Router::new()
        .route("/", get(registry::get_agent).put(registry::update_agent).delete(registry::delete_agent))
        .route("/status", get(status::get_agent_status))
        .route("/sessions", get(control::list_sessions))
        .route("/task", post(control::send_task));

    let agents_router = Router::new()
        .route("/", get(registry::list_agents).post(registry::create_agent))
        .nest("/{id}", agent_id_router);

    Router::new().nest("/agents", agents_router)
}
