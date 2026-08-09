use crate::{AppState, errors::AppError};
use askama::Template;
use axum::{Form, extract::State, response::Html};
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
    if pairing_code.len() != 11 {
        return render_error("Invalid pairing code".to_owned(), form);
    }

    let node_id = state.next_node_id()?;
    if let Err(e) = state
        .device_manager
        .commission_with_code(&pairing_code, node_id, &form.name)
        .await
    {
        render_error(format!("Failed to commission: {}", e), form)
    } else {
        let template = SuccessTemplate { name: form.name };
        Ok(Html(template.render()?))
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
    name: String,
}

#[cfg(test)]
mod tests {
    use matc::onboarding::decode_manual_pairing_code;

    #[test]
    fn decode() {
        decode_manual_pairing_code("00170936664").unwrap();
    }
}
