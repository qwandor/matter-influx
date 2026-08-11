use crate::{AppState, errors::AppError};
use askama::Template;
use axum::{Form, extract::State, response::Html};
use matter_controller::NodeInfo;
use serde::Deserialize;
use std::sync::Arc;

pub async fn commission() -> Result<Html<String>, AppError> {
    let template = CommissionTemplate::default();
    Ok(Html(template.render()?))
}

pub async fn submit(
    State(state): State<Arc<AppState>>,
    Form(form): Form<CommissionForm>,
) -> Result<Html<String>, AppError> {
    let name = form.name.trim();
    let pairing_code = form.pairing_code.trim();

    if name.is_empty() {
        return render_error("Name must not be empty".to_owned(), form);
    }

    match state
        .matter_controller
        .commission(&pairing_code, Some(form.name.to_owned()))
        .await
    {
        Ok(node_info) => {
            let template = SuccessTemplate { node_info };
            Ok(Html(template.render()?))
        }
        Err(e) => render_error(format!("Failed to commission: {}", e), form),
    }
}

fn render_error(error: String, form: CommissionForm) -> Result<Html<String>, AppError> {
    let template = CommissionTemplate {
        error: Some(error),
        form,
    };
    return Ok(Html(template.render()?));
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
pub struct CommissionForm {
    pairing_code: String,
    name: String,
}

#[derive(Default, Template)]
#[template(path = "commission.html")]
struct CommissionTemplate {
    error: Option<String>,
    form: CommissionForm,
}

#[derive(Template)]
#[template(path = "commission_success.html")]
struct SuccessTemplate {
    node_info: NodeInfo,
}
