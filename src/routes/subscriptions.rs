use actix_web::{HttpResponse, web};
#[derive(serde::Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

// #[post("/subscriptions")]
pub async fn subscribe(_form: web::Form<FormData>) -> HttpResponse {
    HttpResponse::Ok().finish()
}
