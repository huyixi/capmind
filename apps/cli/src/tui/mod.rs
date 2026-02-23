mod app;
mod bottom_pane;
mod chat_widget;
mod composer;
mod render;
mod theme;
mod types;

use crate::error::AppError;
use crate::supabase::SupabaseClient;

pub async fn run(client: &SupabaseClient) -> Result<(), AppError> {
    app::ComposeApp::new(client).run().await
}
