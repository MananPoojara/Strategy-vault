mod policy;

use axum::{
    body::{to_bytes, Body},
    extract::{Path, Query, State},
    http::{header, HeaderMap, Method, Request, StatusCode, Uri},
    response::{IntoResponse, Redirect, Response},
    routing::{any, get},
    Json, Router,
};
use serde::Deserialize;
use std::{path::PathBuf, sync::Arc, time::Duration};
use tokio::sync::Semaphore;
use tower_http::{compression::CompressionLayer, services::ServeDir, trace::TraceLayer};
use tower_http::compression::predicate::{DefaultPredicate,NotForContentType,Predicate};

#[derive(Clone)]
struct App {
    client: reqwest::Client,
    auth: String,
    india: String,
    us: String,
    internal_token: String,
    frontend: PathBuf,
    public: PathBuf,
    optimization: Arc<Semaphore>,
    reads: Arc<Semaphore>,
}
#[derive(Deserialize)]
struct Session {
    user: User,
}
#[derive(Deserialize)]
struct User {
    role: String,
}

fn error(status: StatusCode, message: &str) -> Response {
    (status, Json(serde_json::json!({"error":message}))).into_response()
}
async fn session(app: &App, headers: &HeaderMap) -> Result<Session, Response> {
    let Some(cookie) = headers.get(header::COOKIE) else {
        return Err(error(StatusCode::UNAUTHORIZED, "Please sign in."));
    };
    let response = app
        .client
        .get(format!("{}/internal/session", app.auth))
        .header(header::COOKIE, cookie)
        .header("x-vault-internal-token", &app.internal_token)
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .map_err(|_| {
            error(
                StatusCode::SERVICE_UNAVAILABLE,
                "Sign-in service is unavailable.",
            )
        })?;
    if response.status() == StatusCode::UNAUTHORIZED {
        return Err(error(StatusCode::UNAUTHORIZED, "Please sign in."));
    }
    if !response.status().is_success() {
        return Err(error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Sign-in service is unavailable.",
        ));
    }
    let session: Session = response.json().await.map_err(|_| {
        error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Sign-in service is unavailable.",
        )
    })?;
    if !matches!(
        session.user.role.as_str(),
        "admin" | "researcher" | "viewer"
    ) {
        return Err(error(StatusCode::FORBIDDEN, "Unknown access level."));
    }
    Ok(session)
}
async fn proxy(app: &App, url: String, request: Request<Body>, timeout: Duration) -> Response {
    let (parts, body) = request.into_parts();
    let book_file=parts.uri.path().starts_with("/api/app/books/files/");
    let bytes = match to_bytes(body, 1024 * 1024).await {
        Ok(bytes) => bytes,
        Err(_) => return error(StatusCode::PAYLOAD_TOO_LARGE, "Request too large."),
    };
    let mut upstream = app
        .client
        .request(parts.method, url)
        .timeout(timeout)
        .body(bytes);
    for name in [
        header::COOKIE,
        header::CONTENT_TYPE,
        header::ORIGIN,
        header::USER_AGENT,
        header::ACCEPT,
        header::REFERER,
        header::RANGE,
        header::IF_RANGE,
    ] {
        if let Some(value) = parts.headers.get(&name) {
            upstream = upstream.header(name, value);
        }
    }
    if let Some(ip) = parts.headers.get("x-vault-client-ip") {
        upstream = upstream.header("x-vault-client-ip", ip);
    }
    let mut response = match upstream.send().await {
        Ok(response) => response,
        Err(cause) => {
            tracing::warn!(timeout = cause.is_timeout(), "upstream request failed");
            return error(
                if cause.is_timeout() {
                    StatusCode::GATEWAY_TIMEOUT
                } else {
                    StatusCode::BAD_GATEWAY
                },
                "The requested service did not respond. Please try again.",
            );
        }
    };
    let status = response.status();
    let headers = response.headers().clone();
    // Only the authenticated fixed-manifest book route streams large binary data.
    // Other API responses retain the existing bounded buffered contract.
    if book_file && headers.get(header::CONTENT_TYPE).is_some_and(|v|v.as_bytes().starts_with(b"application/pdf")||v.as_bytes().starts_with(b"application/octet-stream")) {
        let mut result=Response::new(Body::from_stream(response.bytes_stream()));
        *result.status_mut()=status;
        for name in [header::CONTENT_TYPE,header::CONTENT_LENGTH,header::CONTENT_RANGE,
            header::ACCEPT_RANGES,header::CONTENT_DISPOSITION] {
            if let Some(value)=headers.get(&name){result.headers_mut().insert(name,value.clone());}
        }
        result.headers_mut().insert(header::CACHE_CONTROL,header::HeaderValue::from_static("private, no-store"));
        result.headers_mut().insert(header::X_CONTENT_TYPE_OPTIONS,header::HeaderValue::from_static("nosniff"));
        return result;
    }
    if response
        .content_length()
        .is_some_and(|n| n > 64 * 1024 * 1024)
    {
        return error(
            StatusCode::BAD_GATEWAY,
            "Result is too large for this request.",
        );
    }
    let mut bytes = Vec::new();
    loop {
        match response.chunk().await {
            Ok(Some(chunk)) => {
                if bytes.len() + chunk.len() > 64 * 1024 * 1024 {
                    return error(
                        StatusCode::BAD_GATEWAY,
                        "Result is too large for this request.",
                    );
                }
                bytes.extend_from_slice(&chunk);
            }
            Ok(None) => break,
            Err(_) => return error(StatusCode::BAD_GATEWAY, "Could not read the result."),
        }
    }
    let mut result = (status, bytes).into_response();
    for name in [
        header::CONTENT_TYPE,
        header::SET_COOKIE,
        header::LOCATION,
        header::RETRY_AFTER,
    ] {
        for value in headers.get_all(&name) {
            result.headers_mut().append(name.clone(), value.clone());
        }
    }
    result.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-store"),
    );
    if book_file {
        for name in [header::CONTENT_RANGE,header::ACCEPT_RANGES] {
            if let Some(value)=headers.get(&name){result.headers_mut().insert(name,value.clone());}
        }
    }
    result
}
fn with_query(base: &str, uri: &Uri) -> String {
    match uri.query() {
        Some(q) => format!("{base}?{q}"),
        None => base.into(),
    }
}
async fn auth_proxy(State(app): State<App>, uri: Uri, request: Request<Body>) -> Response {
    if !policy::valid_query(uri.query()) {
        return error(StatusCode::BAD_REQUEST, "Invalid query.");
    }
    proxy(
        &app,
        with_query(&format!("{}{}", app.auth, uri.path()), &uri),
        request,
        Duration::from_secs(30),
    )
    .await
}
async fn app_proxy(State(app): State<App>, uri: Uri, request: Request<Body>) -> Response {
    if !policy::valid_query(uri.query()) {
        return error(StatusCode::BAD_REQUEST, "Invalid query.");
    }
    // The application-data service validates the session and write origin itself.
    let timeout=if uri.path().starts_with("/api/app/books/files/") {180} else {30};
    proxy(
        &app,
        with_query(&format!("{}{}", app.auth, uri.path()), &uri),
        request,
        Duration::from_secs(timeout),
    )
    .await
}
async fn engine_proxy(
    State(app): State<App>,
    Path((market, path)): Path<(String, String)>,
    uri: Uri,
    request: Request<Body>,
) -> Response {
    if !matches!(market.as_str(), "in" | "us") || !policy::valid_query(uri.query()) {
        return error(StatusCode::BAD_REQUEST, "Invalid market or query.");
    }
    let user = match session(&app, request.headers()).await {
        Ok(session) => session.user,
        Err(response) => return response,
    };
    if path == "notes" {
        return proxy(
            &app,
            with_query(&format!("{}/api/app/notes", app.auth), &uri),
            request,
            Duration::from_secs(30),
        )
        .await;
    }
    if request.method() != Method::GET {
        return error(StatusCode::METHOD_NOT_ALLOWED, "GET required.");
    }
    if !policy::allowed_engine_path(&path) {
        return error(StatusCode::NOT_FOUND, "Unknown endpoint.");
    }
    let query = match Query::<std::collections::BTreeMap<String, String>>::try_from_uri(&uri) {
        Ok(Query(query)) => query,
        Err(_) => return error(StatusCode::BAD_REQUEST, "Invalid query."),
    };
    if !policy::safe_engine_numbers(&query) {
        return error(StatusCode::BAD_REQUEST,"Use finite parameter values and a grid of at most 50,000 combinations, with at most 1,000 values per setting.");
    }
    if let Some(problem) = policy::lab_spec_problem(&path, &query) {
        return error(StatusCode::BAD_REQUEST, problem);
    }
    let optimizing = policy::is_optimization_path(&path);
    let optimizing = optimizing || policy::is_lab_optimization_path(&path);
    if optimizing && !policy::can_optimize(&user.role) {
        return error(
            StatusCode::FORBIDDEN,
            "Researcher access is required for optimization.",
        );
    }
    if optimizing
        && request
            .headers()
            .get("sec-fetch-site")
            .is_some_and(|v| v == "cross-site")
    {
        return error(StatusCode::FORBIDDEN, "Invalid request origin.");
    }
    let slots = if optimizing {
        &app.optimization
    } else {
        &app.reads
    };
    let acquired = if optimizing {
        slots.clone().try_acquire_owned().map_err(|_| ())
    } else {
        match tokio::time::timeout(Duration::from_secs(15), slots.clone().acquire_owned()).await {
            Ok(Ok(permit)) => Ok(permit),
            _ => Err(()),
        }
    };
    let _permit = match acquired {
        Ok(permit) => permit,
        Err(_) => {
            let mut response = error(
                StatusCode::TOO_MANY_REQUESTS,
                if optimizing {
                    "An optimization is already running. Please retry when it finishes."
                } else {
                    "The server is busy. Please retry shortly."
                },
            );
            response
                .headers_mut()
                .insert(header::RETRY_AFTER, header::HeaderValue::from_static("5"));
            return response;
        }
    };
    let engine = if market == "in" { &app.india } else { &app.us };
    proxy(
        &app,
        with_query(&format!("{engine}/api/{path}"), &uri),
        request,
        Duration::from_secs(if optimizing { 600 } else { 120 }),
    )
    .await
}
async fn health(State(app): State<App>) -> Response {
    let response = app
        .client
        .get(format!("{}/healthz", app.auth))
        .timeout(Duration::from_secs(3))
        .send()
        .await;
    let mut ready = response.is_ok_and(|r| r.status().is_success());
    for engine in [&app.india, &app.us] {
        ready &= app
            .client
            .get(format!("{engine}/api/tickers"))
            .timeout(Duration::from_secs(3))
            .send()
            .await
            .is_ok_and(|r| r.status().is_success());
    }
    (
        if ready {
            StatusCode::OK
        } else {
            StatusCode::SERVICE_UNAVAILABLE
        },
        Json(serde_json::json!({"status":if ready{"ok"}else{"starting"}})),
    )
        .into_response()
}
async fn static_files(State(app): State<App>, request: Request<Body>) -> Response {
    let path = request.uri().path().to_string();
    if request.method() != Method::GET && request.method() != Method::HEAD {
        return error(StatusCode::METHOD_NOT_ALLOWED, "GET required.");
    }
    let public = matches!(
        path.as_str(),
        "/login.html"
            | "/login.js"
            | "/login.css"
            | "/app-session.js"
            | "/admin.js"
            | "/app-runtime.js"
    );
    if !public && !path.starts_with("/vendor/") {
        let verified = session(&app, request.headers()).await;
        if path == "/admin.html" && verified.as_ref().is_ok_and(|s| s.user.role != "admin") {
            return error(StatusCode::FORBIDDEN, "Administrator access required.");
        }
        if let Err(response) = verified {
            if response.status() == StatusCode::UNAUTHORIZED
                && (path == "/" || path.ends_with(".html"))
            {
                return Redirect::to("/login.html").into_response();
            }
            return response;
        }
    }
    if path == "/app-runtime.js" {
        return (
            [
                (header::CONTENT_TYPE, "text/javascript; charset=utf-8"),
                (header::CACHE_CONTROL, "no-store"),
            ],
            "window.VAULT_APP = true;",
        )
            .into_response();
    }
    let root = if public || path == "/admin.html" {
        &app.public
    } else {
        &app.frontend
    };
    match ServeDir::new(root).try_call(request).await {
        Ok(response) => response.map(Body::new),
        Err(_) => error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Could not load this page.",
        ),
    }
}
async fn headers(request: Request<Body>, next: axum::middleware::Next) -> Response {
    let private_book=request.uri().path().starts_with("/api/app/books/files/");
    let mut response = next.run(request).await;
    for (name,value) in [
        ("x-content-type-options","nosniff"),
        ("x-frame-options","DENY"),
        ("referrer-policy","same-origin"),
        ("content-security-policy","default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; connect-src 'self'; frame-ancestors 'none'; base-uri 'self'; form-action 'self'"),
        ("cache-control","no-store"),
    ] { response.headers_mut().insert(header::HeaderName::from_static(name),header::HeaderValue::from_static(value)); }
    if private_book {response.headers_mut().insert(header::CACHE_CONTROL,header::HeaderValue::from_static("private, no-store"));}
    response
}
#[tokio::main(worker_threads = 2)]
async fn main() {
    if std::env::args().nth(1).as_deref() == Some("--healthcheck") {
        let ready = reqwest::Client::new()
            .get("http://127.0.0.1:8181/healthz")
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .is_ok_and(|r| r.status().is_success());
        std::process::exit(if ready { 0 } else { 1 });
    }

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "vault_webapp=info,tower_http=warn".into()),
        )
        .init();
    let env = |key: &str, default: &str| std::env::var(key).unwrap_or_else(|_| default.into());
    let app = App {
        client: reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("HTTP client"),
        auth: env("VAULT_AUTH_URL", "http://127.0.0.1:8102"),
        india: env("VAULT_ENGINE_IN", "http://127.0.0.1:8090"),
        us: env("VAULT_ENGINE_US", "http://127.0.0.1:8091"),
        internal_token: std::env::var("VAULT_INTERNAL_TOKEN")
            .expect("VAULT_INTERNAL_TOKEN is required"),
        frontend: PathBuf::from(env("VAULT_FRONTEND", "../frontend")),
        public: PathBuf::from(env("VAULT_PUBLIC", "public")),
        optimization: Arc::new(Semaphore::new(1)),
        reads: Arc::new(Semaphore::new(4)),
    };
    let router = Router::new()
        .route("/healthz", get(health))
        .route("/api/auth/{*path}", any(auth_proxy))
        .route("/api/app/{*path}", any(app_proxy))
        .route("/api/{market}/{*path}", any(engine_proxy))
        .fallback(static_files)
        .layer(axum::extract::DefaultBodyLimit::max(1024 * 1024))
        .layer(axum::middleware::from_fn(headers))
        .layer(CompressionLayer::new().compress_when(DefaultPredicate::new()
            .and(NotForContentType::const_new("application/pdf"))
            .and(NotForContentType::const_new("application/octet-stream"))))
        .layer(TraceLayer::new_for_http())
        .with_state(app);
    let address = env("VAULT_BIND", "127.0.0.1:8181");
    let listener = tokio::net::TcpListener::bind(&address)
        .await
        .expect("bind gateway");
    tracing::info!(%address,"Strategy Vault gateway ready");
    axum::serve(listener, router)
        .with_graceful_shutdown(async {
            #[cfg(unix)]
            {
                let mut term =
                    tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                        .expect("SIGTERM");
                tokio::select! { _=tokio::signal::ctrl_c()=>{}, _=term.recv()=>{} }
            }
            #[cfg(not(unix))]
            {
                let _ = tokio::signal::ctrl_c().await;
            }
        })
        .await
        .expect("serve gateway");
}
