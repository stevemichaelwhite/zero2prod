use actix_web::{HttpResponse, web};
use sqlx::{types::chrono::Utc, PgPool};
use uuid::Uuid;
#[derive(serde::Deserialize)]
#[allow(dead_code)]
pub struct FormData {
    email: String,
    name: String,
}

// #[post("/subscriptions")]
pub async fn subscribe(
    form: web::Form<FormData>,
    connection_pool: web::Data<PgPool>,
) -> HttpResponse {
    match sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at)
        VALUES ($1, $2, $3, $4)
        "#,
        Uuid::new_v4(),
        form.email,
        form.name,
        Utc::now()
    )
    // We use `get_ref` to get an immutable reference to the `PgConnection`
    // wrapped by `web::Data`.
    .execute(connection_pool.get_ref())
    .await {
    Ok(_) => HttpResponse::Ok().finish(),
    Err(e) => {
        println!("Failed to execute query: {}", e);
        HttpResponse::InternalServerError().finish()
    }
}

}
