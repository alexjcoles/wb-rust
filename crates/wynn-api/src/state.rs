use std::sync::Arc;
use wynn_core::db::ItemDb;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<ItemDb>,
}
