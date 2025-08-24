pub mod config;
pub mod controller;
pub mod state;
pub mod states;

// 重构后的状态管理模块
pub mod state_core;
pub mod state_connection;
pub mod state_database;
pub mod state_key_navigation;
pub mod state_search;
pub mod state_tree_view;
pub mod state_progress;
pub mod state_confirmation;
pub mod state_export_import;
pub mod state_bulk_operations;

pub use config::AppConfig;
pub use controller::AppController;
pub use state::AppState;
pub use states::*;