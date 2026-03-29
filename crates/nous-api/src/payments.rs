use axum::extract::{Path, Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::{IntoParams, ToSchema};

use nous_payments::{Escrow, EscrowStatus, Invoice, InvoiceStatus, LineItem, Transaction, Wallet};

use crate::error::ApiError;
use crate::state::AppState;

// ── Request / Response types ────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateWalletRequest {
    pub did: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct WalletResponse {
    pub did: String,
    pub balances: Vec<BalanceEntry>,
    pub nonce: u64,
    pub created_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BalanceEntry {
    pub token: String,
    pub amount: String,
}

impl From<&Wallet> for WalletResponse {
    fn from(w: &Wallet) -> Self {
        let mut balances: Vec<BalanceEntry> = w
            .balances
            .iter()
            .map(|(token, amount)| BalanceEntry {
                token: token.clone(),
                amount: amount.to_string(),
            })
            .collect();
        balances.sort_by(|a, b| a.token.cmp(&b.token));
        Self {
            did: w.did.clone(),
            balances,
            nonce: w.nonce,
            created_at: w.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreditRequest {
    pub token: String,
    pub amount: u128,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct DebitRequest {
    pub token: String,
    pub amount: u128,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct TransferRequest {
    pub from_did: String,
    pub to_did: String,
    pub token: String,
    pub amount: u128,
    pub memo: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TransactionResponse {
    pub id: String,
    pub from_did: String,
    pub to_did: String,
    pub token: String,
    pub amount: String,
    pub fee: String,
    pub memo: Option<String>,
    pub status: String,
    pub timestamp: String,
}

impl From<&Transaction> for TransactionResponse {
    fn from(tx: &Transaction) -> Self {
        Self {
            id: tx.id.clone(),
            from_did: tx.from_did.clone(),
            to_did: tx.to_did.clone(),
            token: tx.token.clone(),
            amount: tx.amount.to_string(),
            fee: tx.fee.to_string(),
            memo: tx.memo.clone(),
            status: format!("{:?}", tx.status),
            timestamp: tx.timestamp.to_rfc3339(),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateEscrowRequest {
    pub buyer_did: String,
    pub seller_did: String,
    pub arbiter_did: Option<String>,
    pub token: String,
    pub amount: u128,
    pub description: String,
    pub duration_hours: i64,
    pub conditions: Option<Vec<String>>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct EscrowResponse {
    pub id: String,
    pub buyer_did: String,
    pub seller_did: String,
    pub arbiter_did: Option<String>,
    pub token: String,
    pub amount: String,
    pub status: String,
    pub description: String,
    pub conditions: Vec<String>,
    pub created_at: String,
    pub expires_at: String,
}

impl From<&Escrow> for EscrowResponse {
    fn from(e: &Escrow) -> Self {
        Self {
            id: e.id.clone(),
            buyer_did: e.buyer_did.clone(),
            seller_did: e.seller_did.clone(),
            arbiter_did: e.arbiter_did.clone(),
            token: e.token.clone(),
            amount: e.amount.to_string(),
            status: format!("{:?}", e.status),
            description: e.description.clone(),
            conditions: e.release_conditions.clone(),
            created_at: e.created_at.to_rfc3339(),
            expires_at: e.expires_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct EscrowActionRequest {
    pub caller_did: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateInvoiceRequest {
    pub from_did: String,
    pub to_did: String,
    pub token: String,
    pub days_until_due: i64,
    pub memo: Option<String>,
    pub items: Vec<LineItemRequest>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct LineItemRequest {
    pub description: String,
    pub quantity: u32,
    pub unit_price: u128,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct InvoiceResponse {
    pub id: String,
    pub from_did: String,
    pub to_did: String,
    pub token: String,
    pub total: String,
    pub status: String,
    pub memo: Option<String>,
    pub items: Vec<LineItemResponse>,
    pub created_at: String,
    pub due_at: String,
    pub paid_at: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LineItemResponse {
    pub description: String,
    pub quantity: u32,
    pub unit_price: String,
    pub total: String,
}

impl From<&Invoice> for InvoiceResponse {
    fn from(inv: &Invoice) -> Self {
        Self {
            id: inv.id.clone(),
            from_did: inv.from_did.clone(),
            to_did: inv.to_did.clone(),
            token: inv.token.clone(),
            total: inv.total().to_string(),
            status: format!("{:?}", inv.status),
            memo: inv.memo.clone(),
            items: inv
                .items
                .iter()
                .map(|i| LineItemResponse {
                    description: i.description.clone(),
                    quantity: i.quantity,
                    unit_price: i.unit_price.to_string(),
                    total: i.total().to_string(),
                })
                .collect(),
            created_at: inv.created_at.to_rfc3339(),
            due_at: inv.due_at.to_rfc3339(),
            paid_at: inv.paid_at.map(|t| t.to_rfc3339()),
        }
    }
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct InvoiceQuery {
    pub did: String,
    pub role: Option<String>,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct TransactionQuery {
    pub limit: Option<usize>,
}

// ── Handlers — Wallets ─────────────────────────────────────────────

#[utoipa::path(
    post, path = "/api/v1/wallets",
    tag = "payments",
    request_body = CreateWalletRequest,
    responses(
        (status = 200, description = "Wallet created", body = WalletResponse),
        (status = 409, description = "Wallet already exists")
    )
)]
pub async fn create_wallet(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateWalletRequest>,
) -> Result<Json<WalletResponse>, ApiError> {
    if req.did.is_empty() {
        return Err(ApiError::bad_request("did is required"));
    }

    let mut wallets = state.wallets.write().await;
    if wallets.contains_key(&req.did) {
        return Err(ApiError {
            status: 409,
            message: format!("wallet already exists for {}", req.did),
        });
    }

    let wallet = Wallet::new(&req.did);
    let resp = WalletResponse::from(&wallet);
    wallets.insert(req.did.clone(), wallet);
    Ok(Json(resp))
}

#[utoipa::path(
    get, path = "/api/v1/wallets/{did}",
    tag = "payments",
    params(("did" = String, Path, description = "Wallet owner DID")),
    responses(
        (status = 200, description = "Wallet details", body = WalletResponse),
        (status = 404, description = "Wallet not found")
    )
)]
pub async fn get_wallet(
    State(state): State<Arc<AppState>>,
    Path(did): Path<String>,
) -> Result<Json<WalletResponse>, ApiError> {
    let wallets = state.wallets.read().await;
    wallets
        .get(&did)
        .map(|w| Json(WalletResponse::from(w)))
        .ok_or_else(|| ApiError::not_found(format!("wallet not found for {did}")))
}

#[utoipa::path(
    post, path = "/api/v1/wallets/{did}/credit",
    tag = "payments",
    params(("did" = String, Path, description = "Wallet owner DID")),
    request_body = CreditRequest,
    responses(
        (status = 200, description = "Wallet credited", body = WalletResponse),
        (status = 404, description = "Wallet not found")
    )
)]
pub async fn credit_wallet(
    State(state): State<Arc<AppState>>,
    Path(did): Path<String>,
    Json(req): Json<CreditRequest>,
) -> Result<Json<WalletResponse>, ApiError> {
    if req.amount == 0 {
        return Err(ApiError::bad_request("amount must be positive"));
    }

    let mut wallets = state.wallets.write().await;
    let wallet = wallets
        .get_mut(&did)
        .ok_or_else(|| ApiError::not_found(format!("wallet not found for {did}")))?;

    wallet.credit(&req.token, req.amount);
    Ok(Json(WalletResponse::from(&*wallet)))
}

#[utoipa::path(
    post, path = "/api/v1/wallets/{did}/debit",
    tag = "payments",
    params(("did" = String, Path, description = "Wallet owner DID")),
    request_body = DebitRequest,
    responses(
        (status = 200, description = "Wallet debited", body = WalletResponse),
        (status = 400, description = "Insufficient balance"),
        (status = 404, description = "Wallet not found")
    )
)]
pub async fn debit_wallet(
    State(state): State<Arc<AppState>>,
    Path(did): Path<String>,
    Json(req): Json<DebitRequest>,
) -> Result<Json<WalletResponse>, ApiError> {
    if req.amount == 0 {
        return Err(ApiError::bad_request("amount must be positive"));
    }

    let mut wallets = state.wallets.write().await;
    let wallet = wallets
        .get_mut(&did)
        .ok_or_else(|| ApiError::not_found(format!("wallet not found for {did}")))?;

    wallet.debit(&req.token, req.amount).map_err(ApiError::from)?;
    Ok(Json(WalletResponse::from(&*wallet)))
}

#[utoipa::path(
    get, path = "/api/v1/wallets/{did}/transactions",
    tag = "payments",
    params(
        ("did" = String, Path, description = "Wallet owner DID"),
        TransactionQuery,
    ),
    responses((status = 200, description = "Transaction history"))
)]
pub async fn get_transactions(
    State(state): State<Arc<AppState>>,
    Path(did): Path<String>,
    Query(query): Query<TransactionQuery>,
) -> Result<Json<Vec<TransactionResponse>>, ApiError> {
    let transactions = state.transactions.read().await;
    let limit = query.limit.unwrap_or(50).min(200);

    let result: Vec<TransactionResponse> = transactions
        .iter()
        .rev()
        .filter(|tx| tx.from_did == did || tx.to_did == did)
        .take(limit)
        .map(TransactionResponse::from)
        .collect();

    Ok(Json(result))
}

// ── Handlers — Transfers ───────────────────────────────────────────

#[utoipa::path(
    post, path = "/api/v1/transfers",
    tag = "payments",
    request_body = TransferRequest,
    responses(
        (status = 200, description = "Transfer completed", body = TransactionResponse),
        (status = 400, description = "Invalid transfer"),
        (status = 404, description = "Wallet not found")
    )
)]
pub async fn create_transfer(
    State(state): State<Arc<AppState>>,
    Json(req): Json<TransferRequest>,
) -> Result<Json<TransactionResponse>, ApiError> {
    let mut wallets = state.wallets.write().await;

    // Both wallets must exist
    if !wallets.contains_key(&req.from_did) {
        return Err(ApiError::not_found(format!(
            "sender wallet not found for {}",
            req.from_did
        )));
    }
    if !wallets.contains_key(&req.to_did) {
        return Err(ApiError::not_found(format!(
            "receiver wallet not found for {}",
            req.to_did
        )));
    }

    // Extract both wallets for the transfer
    let mut sender = wallets.remove(&req.from_did).unwrap();
    let mut receiver = wallets.remove(&req.to_did).unwrap();

    let result = nous_payments::transfer(&mut sender, &mut receiver, &req.token, req.amount);

    // Put wallets back regardless of result
    wallets.insert(sender.did.clone(), sender);
    wallets.insert(receiver.did.clone(), receiver);

    let mut tx = result.map_err(ApiError::from)?;
    if let Some(ref memo) = req.memo {
        tx.memo = Some(memo.clone());
    }

    let resp = TransactionResponse::from(&tx);
    state.transactions.write().await.push(tx);
    Ok(Json(resp))
}

// ── Handlers — Escrows ─────────────────────────────────────────────

#[utoipa::path(
    post, path = "/api/v1/escrows",
    tag = "payments",
    request_body = CreateEscrowRequest,
    responses(
        (status = 200, description = "Escrow created", body = EscrowResponse),
        (status = 400, description = "Invalid escrow")
    )
)]
pub async fn create_escrow(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateEscrowRequest>,
) -> Result<Json<EscrowResponse>, ApiError> {
    let mut escrow = Escrow::new(
        &req.buyer_did,
        &req.seller_did,
        &req.token,
        req.amount,
        &req.description,
        req.duration_hours,
    )
    .map_err(ApiError::from)?;

    if let Some(ref arbiter) = req.arbiter_did {
        escrow = escrow.with_arbiter(arbiter);
    }
    if let Some(ref conditions) = req.conditions {
        for c in conditions {
            escrow.add_condition(c);
        }
    }

    let resp = EscrowResponse::from(&escrow);
    state
        .escrows
        .write()
        .await
        .insert(escrow.id.clone(), escrow);
    Ok(Json(resp))
}

#[utoipa::path(
    get, path = "/api/v1/escrows/{escrow_id}",
    tag = "payments",
    params(("escrow_id" = String, Path, description = "Escrow ID")),
    responses(
        (status = 200, description = "Escrow details", body = EscrowResponse),
        (status = 404, description = "Escrow not found")
    )
)]
pub async fn get_escrow(
    State(state): State<Arc<AppState>>,
    Path(escrow_id): Path<String>,
) -> Result<Json<EscrowResponse>, ApiError> {
    let escrows = state.escrows.read().await;
    escrows
        .get(&escrow_id)
        .map(|e| Json(EscrowResponse::from(e)))
        .ok_or_else(|| ApiError::not_found(format!("escrow {escrow_id} not found")))
}

#[utoipa::path(
    post, path = "/api/v1/escrows/{escrow_id}/release",
    tag = "payments",
    params(("escrow_id" = String, Path, description = "Escrow ID")),
    request_body = EscrowActionRequest,
    responses(
        (status = 200, description = "Escrow released", body = EscrowResponse),
        (status = 400, description = "Cannot release"),
        (status = 404, description = "Escrow not found")
    )
)]
pub async fn release_escrow(
    State(state): State<Arc<AppState>>,
    Path(escrow_id): Path<String>,
    Json(req): Json<EscrowActionRequest>,
) -> Result<Json<EscrowResponse>, ApiError> {
    let mut escrows = state.escrows.write().await;
    let escrow = escrows
        .get_mut(&escrow_id)
        .ok_or_else(|| ApiError::not_found(format!("escrow {escrow_id} not found")))?;

    escrow.release(&req.caller_did).map_err(ApiError::from)?;
    Ok(Json(EscrowResponse::from(&*escrow)))
}

#[utoipa::path(
    post, path = "/api/v1/escrows/{escrow_id}/refund",
    tag = "payments",
    params(("escrow_id" = String, Path, description = "Escrow ID")),
    request_body = EscrowActionRequest,
    responses(
        (status = 200, description = "Escrow refunded", body = EscrowResponse),
        (status = 400, description = "Cannot refund"),
        (status = 404, description = "Escrow not found")
    )
)]
pub async fn refund_escrow(
    State(state): State<Arc<AppState>>,
    Path(escrow_id): Path<String>,
    Json(req): Json<EscrowActionRequest>,
) -> Result<Json<EscrowResponse>, ApiError> {
    let mut escrows = state.escrows.write().await;
    let escrow = escrows
        .get_mut(&escrow_id)
        .ok_or_else(|| ApiError::not_found(format!("escrow {escrow_id} not found")))?;

    escrow.refund(&req.caller_did).map_err(ApiError::from)?;
    Ok(Json(EscrowResponse::from(&*escrow)))
}

#[utoipa::path(
    post, path = "/api/v1/escrows/{escrow_id}/dispute",
    tag = "payments",
    params(("escrow_id" = String, Path, description = "Escrow ID")),
    request_body = EscrowActionRequest,
    responses(
        (status = 200, description = "Escrow disputed", body = EscrowResponse),
        (status = 400, description = "Cannot dispute"),
        (status = 404, description = "Escrow not found")
    )
)]
pub async fn dispute_escrow(
    State(state): State<Arc<AppState>>,
    Path(escrow_id): Path<String>,
    Json(req): Json<EscrowActionRequest>,
) -> Result<Json<EscrowResponse>, ApiError> {
    let mut escrows = state.escrows.write().await;
    let escrow = escrows
        .get_mut(&escrow_id)
        .ok_or_else(|| ApiError::not_found(format!("escrow {escrow_id} not found")))?;

    escrow.dispute(&req.caller_did).map_err(ApiError::from)?;
    Ok(Json(EscrowResponse::from(&*escrow)))
}

// ── Handlers — Invoices ────────────────────────────────────────────

#[utoipa::path(
    post, path = "/api/v1/invoices",
    tag = "payments",
    request_body = CreateInvoiceRequest,
    responses(
        (status = 200, description = "Invoice created", body = InvoiceResponse),
        (status = 400, description = "Invalid invoice")
    )
)]
pub async fn create_invoice(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateInvoiceRequest>,
) -> Result<Json<InvoiceResponse>, ApiError> {
    if req.items.is_empty() {
        return Err(ApiError::bad_request("invoice must have at least one item"));
    }
    if req.days_until_due <= 0 {
        return Err(ApiError::bad_request("days_until_due must be positive"));
    }

    let mut invoice = Invoice::new(&req.from_did, &req.to_did, &req.token, req.days_until_due);
    if let Some(ref memo) = req.memo {
        invoice = invoice.with_memo(memo);
    }
    for item in &req.items {
        invoice.add_item(LineItem::new(&item.description, item.quantity, item.unit_price));
    }

    let resp = InvoiceResponse::from(&invoice);
    state
        .invoices
        .write()
        .await
        .insert(invoice.id.clone(), invoice);
    Ok(Json(resp))
}

#[utoipa::path(
    get, path = "/api/v1/invoices/{invoice_id}",
    tag = "payments",
    params(("invoice_id" = String, Path, description = "Invoice ID")),
    responses(
        (status = 200, description = "Invoice details", body = InvoiceResponse),
        (status = 404, description = "Invoice not found")
    )
)]
pub async fn get_invoice(
    State(state): State<Arc<AppState>>,
    Path(invoice_id): Path<String>,
) -> Result<Json<InvoiceResponse>, ApiError> {
    let invoices = state.invoices.read().await;
    invoices
        .get(&invoice_id)
        .map(|inv| Json(InvoiceResponse::from(inv)))
        .ok_or_else(|| ApiError::not_found(format!("invoice {invoice_id} not found")))
}

#[utoipa::path(
    get, path = "/api/v1/invoices",
    tag = "payments",
    params(InvoiceQuery),
    responses((status = 200, description = "User's invoices"))
)]
pub async fn list_invoices(
    State(state): State<Arc<AppState>>,
    Query(query): Query<InvoiceQuery>,
) -> Result<Json<Vec<InvoiceResponse>>, ApiError> {
    let invoices = state.invoices.read().await;
    let role = query.role.as_deref().unwrap_or("any");

    let result: Vec<InvoiceResponse> = invoices
        .values()
        .filter(|inv| match role {
            "sender" => inv.from_did == query.did,
            "receiver" => inv.to_did == query.did,
            _ => inv.from_did == query.did || inv.to_did == query.did,
        })
        .map(InvoiceResponse::from)
        .collect();

    Ok(Json(result))
}

#[utoipa::path(
    post, path = "/api/v1/invoices/{invoice_id}/pay",
    tag = "payments",
    params(("invoice_id" = String, Path, description = "Invoice ID")),
    responses(
        (status = 200, description = "Invoice paid", body = InvoiceResponse),
        (status = 400, description = "Cannot pay"),
        (status = 404, description = "Invoice not found")
    )
)]
pub async fn pay_invoice(
    State(state): State<Arc<AppState>>,
    Path(invoice_id): Path<String>,
) -> Result<Json<InvoiceResponse>, ApiError> {
    let mut invoices = state.invoices.write().await;
    let invoice = invoices
        .get_mut(&invoice_id)
        .ok_or_else(|| ApiError::not_found(format!("invoice {invoice_id} not found")))?;

    invoice.mark_paid().map_err(ApiError::from)?;
    Ok(Json(InvoiceResponse::from(&*invoice)))
}

#[utoipa::path(
    post, path = "/api/v1/invoices/{invoice_id}/cancel",
    tag = "payments",
    params(("invoice_id" = String, Path, description = "Invoice ID")),
    responses(
        (status = 200, description = "Invoice cancelled", body = InvoiceResponse),
        (status = 400, description = "Cannot cancel"),
        (status = 404, description = "Invoice not found")
    )
)]
pub async fn cancel_invoice(
    State(state): State<Arc<AppState>>,
    Path(invoice_id): Path<String>,
) -> Result<Json<InvoiceResponse>, ApiError> {
    let mut invoices = state.invoices.write().await;
    let invoice = invoices
        .get_mut(&invoice_id)
        .ok_or_else(|| ApiError::not_found(format!("invoice {invoice_id} not found")))?;

    invoice.cancel().map_err(ApiError::from)?;
    Ok(Json(InvoiceResponse::from(&*invoice)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ApiConfig;
    use crate::router;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    async fn test_app() -> axum::Router {
        router(ApiConfig::default())
    }

    fn json_body(value: &serde_json::Value) -> Body {
        Body::from(serde_json::to_vec(value).unwrap())
    }

    fn json_request(method: &str, uri: &str, body: &serde_json::Value) -> Request<Body> {
        Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", "application/json")
            .body(json_body(body))
            .unwrap()
    }

    async fn parse_json(response: axum::http::Response<Body>) -> serde_json::Value {
        let body = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&body).unwrap()
    }

    async fn create_test_wallet(app: &axum::Router, did: &str) -> serde_json::Value {
        let req = serde_json::json!({"did": did});
        let resp = app
            .clone()
            .oneshot(json_request("POST", "/api/v1/wallets", &req))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        parse_json(resp).await
    }

    // ── Wallet tests ───────────────────────────────────────────────

    #[tokio::test]
    async fn create_wallet_returns_empty_balances() {
        let app = test_app().await;
        let json = create_test_wallet(&app, "did:key:alice").await;
        assert_eq!(json["did"], "did:key:alice");
        assert_eq!(json["nonce"], 0);
        assert!(json["balances"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn duplicate_wallet_rejected() {
        let app = test_app().await;
        create_test_wallet(&app, "did:key:alice").await;

        let req = serde_json::json!({"did": "did:key:alice"});
        let resp = app
            .oneshot(json_request("POST", "/api/v1/wallets", &req))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn get_wallet_works() {
        let app = test_app().await;
        create_test_wallet(&app, "did:key:alice").await;

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/wallets/did:key:alice")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = parse_json(resp).await;
        assert_eq!(json["did"], "did:key:alice");
    }

    #[tokio::test]
    async fn get_nonexistent_wallet() {
        let app = test_app().await;
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/wallets/did:key:nobody")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn credit_and_debit_wallet() {
        let app = test_app().await;
        create_test_wallet(&app, "did:key:alice").await;

        // Credit
        let req = serde_json::json!({"token": "ETH", "amount": 1000});
        let resp = app
            .clone()
            .oneshot(json_request(
                "POST",
                "/api/v1/wallets/did:key:alice/credit",
                &req,
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = parse_json(resp).await;
        assert_eq!(json["balances"][0]["token"], "ETH");
        assert_eq!(json["balances"][0]["amount"], "1000");

        // Debit
        let req = serde_json::json!({"token": "ETH", "amount": 300});
        let resp = app
            .oneshot(json_request(
                "POST",
                "/api/v1/wallets/did:key:alice/debit",
                &req,
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = parse_json(resp).await;
        assert_eq!(json["balances"][0]["amount"], "700");
    }

    #[tokio::test]
    async fn debit_insufficient_balance() {
        let app = test_app().await;
        create_test_wallet(&app, "did:key:alice").await;

        let req = serde_json::json!({"token": "ETH", "amount": 100});
        let resp = app
            .oneshot(json_request(
                "POST",
                "/api/v1/wallets/did:key:alice/debit",
                &req,
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn zero_amount_credit_rejected() {
        let app = test_app().await;
        create_test_wallet(&app, "did:key:alice").await;

        let req = serde_json::json!({"token": "ETH", "amount": 0});
        let resp = app
            .oneshot(json_request(
                "POST",
                "/api/v1/wallets/did:key:alice/credit",
                &req,
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    // ── Transfer tests ─────────────────────────────────────────────

    #[tokio::test]
    async fn transfer_between_wallets() {
        let app = test_app().await;
        create_test_wallet(&app, "did:key:alice").await;
        create_test_wallet(&app, "did:key:bob").await;

        // Fund alice
        let credit = serde_json::json!({"token": "ETH", "amount": 1000});
        let _ = app
            .clone()
            .oneshot(json_request(
                "POST",
                "/api/v1/wallets/did:key:alice/credit",
                &credit,
            ))
            .await
            .unwrap();

        // Transfer
        let req = serde_json::json!({
            "from_did": "did:key:alice",
            "to_did": "did:key:bob",
            "token": "ETH",
            "amount": 400,
            "memo": "payment for services"
        });
        let resp = app
            .clone()
            .oneshot(json_request("POST", "/api/v1/transfers", &req))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = parse_json(resp).await;
        assert_eq!(json["amount"], "400");
        assert_eq!(json["status"], "Confirmed");
        assert_eq!(json["memo"], "payment for services");

        // Verify balances
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/wallets/did:key:alice")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let alice = parse_json(resp).await;
        assert_eq!(alice["balances"][0]["amount"], "600");

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/wallets/did:key:bob")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let bob = parse_json(resp).await;
        assert_eq!(bob["balances"][0]["amount"], "400");
    }

    #[tokio::test]
    async fn transfer_insufficient_funds() {
        let app = test_app().await;
        create_test_wallet(&app, "did:key:alice").await;
        create_test_wallet(&app, "did:key:bob").await;

        let req = serde_json::json!({
            "from_did": "did:key:alice",
            "to_did": "did:key:bob",
            "token": "ETH",
            "amount": 100
        });
        let resp = app
            .oneshot(json_request("POST", "/api/v1/transfers", &req))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn transfer_missing_wallet() {
        let app = test_app().await;
        create_test_wallet(&app, "did:key:alice").await;

        let req = serde_json::json!({
            "from_did": "did:key:alice",
            "to_did": "did:key:nobody",
            "token": "ETH",
            "amount": 100
        });
        let resp = app
            .oneshot(json_request("POST", "/api/v1/transfers", &req))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn transaction_history() {
        let app = test_app().await;
        create_test_wallet(&app, "did:key:alice").await;
        create_test_wallet(&app, "did:key:bob").await;

        // Fund and transfer
        let credit = serde_json::json!({"token": "ETH", "amount": 1000});
        let _ = app
            .clone()
            .oneshot(json_request(
                "POST",
                "/api/v1/wallets/did:key:alice/credit",
                &credit,
            ))
            .await
            .unwrap();

        let transfer = serde_json::json!({
            "from_did": "did:key:alice",
            "to_did": "did:key:bob",
            "token": "ETH",
            "amount": 200
        });
        let _ = app
            .clone()
            .oneshot(json_request("POST", "/api/v1/transfers", &transfer))
            .await
            .unwrap();

        // Check history
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/wallets/did:key:alice/transactions?limit=10")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let txs: Vec<serde_json::Value> = serde_json::from_slice(
            &resp.into_body().collect().await.unwrap().to_bytes(),
        )
        .unwrap();
        assert_eq!(txs.len(), 1);
        assert_eq!(txs[0]["amount"], "200");
    }

    // ── Escrow tests ───────────────────────────────────────────────

    #[tokio::test]
    async fn create_and_get_escrow() {
        let app = test_app().await;
        let req = serde_json::json!({
            "buyer_did": "did:key:buyer",
            "seller_did": "did:key:seller",
            "token": "ETH",
            "amount": 500,
            "description": "art commission",
            "duration_hours": 72,
            "conditions": ["design approved", "final delivery"]
        });
        let resp = app
            .clone()
            .oneshot(json_request("POST", "/api/v1/escrows", &req))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = parse_json(resp).await;
        assert_eq!(json["status"], "Active");
        assert_eq!(json["amount"], "500");
        assert_eq!(json["conditions"].as_array().unwrap().len(), 2);

        let escrow_id = json["id"].as_str().unwrap();

        // Get it
        let uri = format!("/api/v1/escrows/{escrow_id}");
        let resp = app
            .oneshot(Request::builder().uri(&uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn escrow_release_flow() {
        let app = test_app().await;
        let req = serde_json::json!({
            "buyer_did": "did:key:buyer",
            "seller_did": "did:key:seller",
            "token": "ETH",
            "amount": 500,
            "description": "test",
            "duration_hours": 24
        });
        let resp = app
            .clone()
            .oneshot(json_request("POST", "/api/v1/escrows", &req))
            .await
            .unwrap();
        let json = parse_json(resp).await;
        let escrow_id = json["id"].as_str().unwrap();

        // Buyer releases
        let uri = format!("/api/v1/escrows/{escrow_id}/release");
        let action = serde_json::json!({"caller_did": "did:key:buyer"});
        let resp = app
            .oneshot(json_request("POST", &uri, &action))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = parse_json(resp).await;
        assert_eq!(json["status"], "Released");
    }

    #[tokio::test]
    async fn escrow_refund_flow() {
        let app = test_app().await;
        let req = serde_json::json!({
            "buyer_did": "did:key:buyer",
            "seller_did": "did:key:seller",
            "token": "ETH",
            "amount": 500,
            "description": "test",
            "duration_hours": 24
        });
        let resp = app
            .clone()
            .oneshot(json_request("POST", "/api/v1/escrows", &req))
            .await
            .unwrap();
        let json = parse_json(resp).await;
        let escrow_id = json["id"].as_str().unwrap();

        // Seller refunds
        let uri = format!("/api/v1/escrows/{escrow_id}/refund");
        let action = serde_json::json!({"caller_did": "did:key:seller"});
        let resp = app
            .oneshot(json_request("POST", &uri, &action))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = parse_json(resp).await;
        assert_eq!(json["status"], "Refunded");
    }

    #[tokio::test]
    async fn escrow_dispute_then_arbiter_refund() {
        let app = test_app().await;
        let req = serde_json::json!({
            "buyer_did": "did:key:buyer",
            "seller_did": "did:key:seller",
            "arbiter_did": "did:key:judge",
            "token": "ETH",
            "amount": 500,
            "description": "disputed item",
            "duration_hours": 24
        });
        let resp = app
            .clone()
            .oneshot(json_request("POST", "/api/v1/escrows", &req))
            .await
            .unwrap();
        let json = parse_json(resp).await;
        let escrow_id = json["id"].as_str().unwrap();

        // Buyer disputes
        let uri = format!("/api/v1/escrows/{escrow_id}/dispute");
        let action = serde_json::json!({"caller_did": "did:key:buyer"});
        let resp = app
            .clone()
            .oneshot(json_request("POST", &uri, &action))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = parse_json(resp).await;
        assert_eq!(json["status"], "Disputed");

        // Arbiter refunds
        let uri = format!("/api/v1/escrows/{escrow_id}/refund");
        let action = serde_json::json!({"caller_did": "did:key:judge"});
        let resp = app
            .oneshot(json_request("POST", &uri, &action))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = parse_json(resp).await;
        assert_eq!(json["status"], "Refunded");
    }

    #[tokio::test]
    async fn unauthorized_escrow_release() {
        let app = test_app().await;
        let req = serde_json::json!({
            "buyer_did": "did:key:buyer",
            "seller_did": "did:key:seller",
            "token": "ETH",
            "amount": 500,
            "description": "test",
            "duration_hours": 24
        });
        let resp = app
            .clone()
            .oneshot(json_request("POST", "/api/v1/escrows", &req))
            .await
            .unwrap();
        let json = parse_json(resp).await;
        let escrow_id = json["id"].as_str().unwrap();

        // Seller tries to release (unauthorized)
        let uri = format!("/api/v1/escrows/{escrow_id}/release");
        let action = serde_json::json!({"caller_did": "did:key:seller"});
        let resp = app
            .oneshot(json_request("POST", &uri, &action))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn invalid_escrow_same_parties() {
        let app = test_app().await;
        let req = serde_json::json!({
            "buyer_did": "did:key:alice",
            "seller_did": "did:key:alice",
            "token": "ETH",
            "amount": 500,
            "description": "test",
            "duration_hours": 24
        });
        let resp = app
            .oneshot(json_request("POST", "/api/v1/escrows", &req))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    // ── Invoice tests ──────────────────────────────────────────────

    #[tokio::test]
    async fn create_and_get_invoice() {
        let app = test_app().await;
        let req = serde_json::json!({
            "from_did": "did:key:alice",
            "to_did": "did:key:bob",
            "token": "ETH",
            "days_until_due": 30,
            "memo": "consulting work",
            "items": [
                {"description": "Design review", "quantity": 2, "unit_price": 100},
                {"description": "Implementation", "quantity": 1, "unit_price": 500}
            ]
        });
        let resp = app
            .clone()
            .oneshot(json_request("POST", "/api/v1/invoices", &req))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = parse_json(resp).await;
        assert_eq!(json["total"], "700");
        assert_eq!(json["status"], "Pending");
        assert_eq!(json["memo"], "consulting work");
        assert_eq!(json["items"].as_array().unwrap().len(), 2);

        let invoice_id = json["id"].as_str().unwrap();

        // Get it
        let uri = format!("/api/v1/invoices/{invoice_id}");
        let resp = app
            .oneshot(Request::builder().uri(&uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn pay_invoice_flow() {
        let app = test_app().await;
        let req = serde_json::json!({
            "from_did": "did:key:alice",
            "to_did": "did:key:bob",
            "token": "ETH",
            "days_until_due": 30,
            "items": [{"description": "Work", "quantity": 1, "unit_price": 100}]
        });
        let resp = app
            .clone()
            .oneshot(json_request("POST", "/api/v1/invoices", &req))
            .await
            .unwrap();
        let json = parse_json(resp).await;
        let invoice_id = json["id"].as_str().unwrap();

        // Pay it
        let uri = format!("/api/v1/invoices/{invoice_id}/pay");
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(&uri)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = parse_json(resp).await;
        assert_eq!(json["status"], "Paid");
        assert!(json["paid_at"].as_str().is_some());
    }

    #[tokio::test]
    async fn cancel_invoice_flow() {
        let app = test_app().await;
        let req = serde_json::json!({
            "from_did": "did:key:alice",
            "to_did": "did:key:bob",
            "token": "ETH",
            "days_until_due": 30,
            "items": [{"description": "Work", "quantity": 1, "unit_price": 100}]
        });
        let resp = app
            .clone()
            .oneshot(json_request("POST", "/api/v1/invoices", &req))
            .await
            .unwrap();
        let json = parse_json(resp).await;
        let invoice_id = json["id"].as_str().unwrap();

        // Cancel it
        let uri = format!("/api/v1/invoices/{invoice_id}/cancel");
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(&uri)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = parse_json(resp).await;
        assert_eq!(json["status"], "Cancelled");
    }

    #[tokio::test]
    async fn cannot_pay_cancelled_invoice() {
        let app = test_app().await;
        let req = serde_json::json!({
            "from_did": "did:key:alice",
            "to_did": "did:key:bob",
            "token": "ETH",
            "days_until_due": 30,
            "items": [{"description": "Work", "quantity": 1, "unit_price": 100}]
        });
        let resp = app
            .clone()
            .oneshot(json_request("POST", "/api/v1/invoices", &req))
            .await
            .unwrap();
        let json = parse_json(resp).await;
        let invoice_id = json["id"].as_str().unwrap();

        // Cancel
        let uri = format!("/api/v1/invoices/{invoice_id}/cancel");
        let _ = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(&uri)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // Try to pay
        let uri = format!("/api/v1/invoices/{invoice_id}/pay");
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(&uri)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn list_invoices_for_did() {
        let app = test_app().await;

        // Create invoice from alice to bob
        let req = serde_json::json!({
            "from_did": "did:key:alice",
            "to_did": "did:key:bob",
            "token": "ETH",
            "days_until_due": 30,
            "items": [{"description": "Work", "quantity": 1, "unit_price": 100}]
        });
        let _ = app
            .clone()
            .oneshot(json_request("POST", "/api/v1/invoices", &req))
            .await
            .unwrap();

        // Create invoice from carol to alice
        let req = serde_json::json!({
            "from_did": "did:key:carol",
            "to_did": "did:key:alice",
            "token": "ETH",
            "days_until_due": 15,
            "items": [{"description": "Other", "quantity": 1, "unit_price": 50}]
        });
        let _ = app
            .clone()
            .oneshot(json_request("POST", "/api/v1/invoices", &req))
            .await
            .unwrap();

        // List all for alice
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/invoices?did=did:key:alice")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let invoices: Vec<serde_json::Value> = serde_json::from_slice(
            &resp.into_body().collect().await.unwrap().to_bytes(),
        )
        .unwrap();
        assert_eq!(invoices.len(), 2);

        // List only as sender
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/invoices?did=did:key:alice&role=sender")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let invoices: Vec<serde_json::Value> = serde_json::from_slice(
            &resp.into_body().collect().await.unwrap().to_bytes(),
        )
        .unwrap();
        assert_eq!(invoices.len(), 1);
        assert_eq!(invoices[0]["from_did"], "did:key:alice");
    }

    #[tokio::test]
    async fn invoice_no_items_rejected() {
        let app = test_app().await;
        let req = serde_json::json!({
            "from_did": "did:key:alice",
            "to_did": "did:key:bob",
            "token": "ETH",
            "days_until_due": 30,
            "items": []
        });
        let resp = app
            .oneshot(json_request("POST", "/api/v1/invoices", &req))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn nonexistent_escrow_returns_404() {
        let app = test_app().await;
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/escrows/nonexistent")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn nonexistent_invoice_returns_404() {
        let app = test_app().await;
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/invoices/nonexistent")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
