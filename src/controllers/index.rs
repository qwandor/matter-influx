use crate::errors::AppError;
use askama::Template;
use axum::response::Html;

pub async fn index() -> Result<Html<String>, AppError> {
    let template = IndexTemplate {};
    Ok(Html(template.render()?))
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {}
