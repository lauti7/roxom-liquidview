use crate::liquidity_service::{LiquidityPoint, LiquidityService};
use actix_web::{HttpResponse, Responder, web};
use chrono::{DateTime, Duration, TimeZone, Timelike, Utc};
use serde::{Deserialize, Serialize};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/ping", web::get().to(ping))
        .route("/liquidity", web::get().to(liquidity));
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Debug, Serialize)]
struct ApiResponse<T> {
    data: T,
}

#[derive(Debug, Deserialize)]
pub struct LiquidityQuery {
    pub from: i64,
    pub to: i64,
    pub symbol: String,
    pub amount: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiquidityPointResponse {
    pub bucket_ts_unix: i64,
    pub avg_bps: f64,
}

pub async fn ping() -> impl Responder {
    HttpResponse::Ok().json(ApiResponse { data: "ok" })
}

pub async fn liquidity(
    query: web::Query<LiquidityQuery>,
    service: web::Data<LiquidityService>,
) -> impl Responder {
    let query = query.into_inner();
    let from_dt = match parse_unix_seconds(query.from) {
        Ok(value) => value,
        Err(msg) => return HttpResponse::BadRequest().json(ErrorResponse { error: msg }),
    };
    let to_dt = match parse_unix_seconds(query.to) {
        Ok(value) => value,
        Err(msg) => return HttpResponse::BadRequest().json(ErrorResponse { error: msg }),
    };

    if query.amount <= 0 {
        return HttpResponse::BadRequest().json(ErrorResponse {
            error: "amount must be a positive integer".to_string(),
        });
    }

    if query.from > query.to {
        return HttpResponse::BadRequest().json(ErrorResponse {
            error: "from must be less than or equal to to".to_string(),
        });
    }

    let from_aligned = truncate_to_hour(from_dt);
    let to_aligned = ceil_to_hour(to_dt);

    let points = match service
        .get_liquidity(&query.symbol, query.amount, from_aligned, to_aligned)
        .await
    {
        Ok(points) => points,
        Err(err) => {
            tracing::error!("Failed to fetch liquidity data: {}", err);
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "failed to fetch liquidity data".to_string(),
            });
        }
    };

    HttpResponse::Ok().json(ApiResponse {
        data: points_to_response(points),
    })
}

fn points_to_response(points: Vec<LiquidityPoint>) -> Vec<LiquidityPointResponse> {
    points
        .into_iter()
        .map(|point| LiquidityPointResponse {
            bucket_ts_unix: point.bucket_ts.timestamp(),
            avg_bps: point.avg_bps,
        })
        .collect()
}

fn parse_unix_seconds(value: i64) -> Result<DateTime<Utc>, String> {
    Utc.timestamp_opt(value, 0)
        .single()
        .ok_or_else(|| "invalid unix timestamp".to_string())
}

fn truncate_to_hour(value: DateTime<Utc>) -> DateTime<Utc> {
    let hour = value.hour();
    let date = value.date_naive();
    let naive = date.and_hms_opt(hour, 0, 0).expect("valid hour for chrono");
    DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc)
}

fn ceil_to_hour(value: DateTime<Utc>) -> DateTime<Utc> {
    let truncated = truncate_to_hour(value);
    if value == truncated {
        truncated
    } else {
        truncated + Duration::hours(1)
    }
}
