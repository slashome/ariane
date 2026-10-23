//! The front door: an HTTP server on `127.0.0.1` that forwards each request to
//! the port of the route its `Host` names (ADR-0016).
//!
//! One upstream connection per request, opened with `hyper`'s low-level
//! client: on the loopback interface a connection costs nothing, and there is
//! no pool to keep consistent. A `101 Switching Protocols` answer — the
//! WebSocket a development server uses to reload the page — joins the two
//! upgraded connections and copies bytes both ways until one side closes.
//!
//! Like nginx's `proxy_pass`, the request reaches the upstream with `Host` set
//! to its own address, `localhost:<port>`, which development servers accept
//! without configuration; the name typed in the browser travels in
//! `X-Forwarded-Host`. A redirect to that address is rewritten back to the name.
//!
//! Every answer the front door writes itself carries [`MARK`], which is how
//! `ariane proxy status` tells it apart from another server on the same port.

use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;

use anyhow::{Context, Result};
use axum::Router;
use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use hyper_util::rt::TokioIo;

use super::{Route, normalize};

/// The header every answer of the front door itself carries.
pub const MARK: &str = "x-ariane-door";

/// Finds the route of a host name, as the `Host` header gives it.
pub type Lookup = Arc<dyn Fn(&str) -> Option<Route> + Send + Sync>;

/// Binds the front door's port on `127.0.0.1`.
///
/// # Errors
///
/// Fails when the port is taken, naming the port and its configuration key.
pub async fn bind(port: u16) -> Result<tokio::net::TcpListener> {
    let address = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    tokio::net::TcpListener::bind(address)
        .await
        .with_context(|| format!("opening the front door on {address} (`proxy.port`)"))
}

/// Serves the front door on `listener` until the task is dropped.
///
/// # Errors
///
/// Fails only when the server itself stops on an I/O error.
pub async fn serve(listener: tokio::net::TcpListener, lookup: Lookup) -> Result<()> {
    axum::serve(listener, router(lookup))
        .await
        .context("serving the front door")
}

/// The front door's router: every request, whatever its path, goes to [`forward`].
pub fn router(lookup: Lookup) -> Router {
    Router::new().fallback(forward).with_state(lookup)
}

/// Forwards `request` to the route of its `Host`, or answers why it cannot.
async fn forward(State(lookup): State<Lookup>, request: Request) -> Response {
    let Some(host) = host_of(&request) else {
        return answer(
            StatusCode::BAD_REQUEST,
            "no Host header: the front door routes by name.",
        );
    };
    let Some(route) = lookup(&host) else {
        return answer(
            StatusCode::NOT_FOUND,
            &format!(
                "no route named {host}.\nDeclare it in the project's node.toml:\n\n[proxy]\nhost = \"{host}\"\nport = 3000\n\nthen run `ariane proxy setup` again so that the name reaches 127.0.0.2."
            ),
        );
    };
    match relay(request, &host, &route).await {
        Ok(response) => response,
        Err(error) => answer(
            StatusCode::BAD_GATEWAY,
            &format!(
                "{host} forwards to 127.0.0.1:{} ({}), which does not answer: {error:#}.\nStart its server, or fix `port` in its [proxy] table.",
                route.port,
                route.owner()
            ),
        ),
    }
}

/// The host name a request is addressed to: its `Host` header, or the
/// authority of an absolute URI.
fn host_of(request: &Request) -> Option<String> {
    let raw = request
        .headers()
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .or_else(|| {
            request
                .uri()
                .authority()
                .map(|authority| authority.as_str())
        })?;
    Some(normalize(raw)).filter(|host| !host.is_empty())
}

/// Sends `request` to the route's port and returns the upstream's answer.
///
/// # Errors
///
/// Fails when nothing listens on the port or the upstream breaks the exchange.
async fn relay(mut request: Request, host: &str, route: &Route) -> Result<Response> {
    let client_side = hyper::upgrade::on(&mut request);
    let stream = tokio::net::TcpStream::connect((Ipv4Addr::LOCALHOST, route.port))
        .await
        .context("connecting")?;
    let (mut sender, connection) = hyper::client::conn::http1::handshake(TokioIo::new(stream))
        .await
        .context("handshake")?;
    tokio::spawn(async move {
        let _ = connection.with_upgrades().await;
    });
    address(&mut request, host, route.port);
    let mut response = sender.send_request(request).await.context("forwarding")?;
    if response.status() == StatusCode::SWITCHING_PROTOCOLS {
        let upstream_side = hyper::upgrade::on(&mut response);
        tokio::spawn(async move {
            if let (Ok(client), Ok(upstream)) = (client_side.await, upstream_side.await) {
                let _ = tokio::io::copy_bidirectional(
                    &mut TokioIo::new(client),
                    &mut TokioIo::new(upstream),
                )
                .await;
            }
        });
    }
    relocate(response.headers_mut(), host, route.port);
    Ok(response.map(Body::new))
}

/// Readdresses `request` to the upstream: an origin-form URI, `Host` set to
/// `localhost:<port>`, and the name typed in the browser in `X-Forwarded-Host`.
fn address(request: &mut Request, host: &str, port: u16) {
    let path = request
        .uri()
        .path_and_query()
        .map_or("/", |path| path.as_str())
        .to_owned();
    if let Ok(uri) = path.parse() {
        *request.uri_mut() = uri;
    }
    let headers = request.headers_mut();
    if let Ok(value) = HeaderValue::from_str(&format!("localhost:{port}")) {
        headers.insert(header::HOST, value);
    }
    if let Ok(value) = HeaderValue::from_str(host) {
        headers.insert("x-forwarded-host", value);
    }
    headers.insert("x-forwarded-proto", HeaderValue::from_static("http"));
}

/// Rewrites a `Location` that points at the upstream's own address back to
/// the name, so that a redirect keeps the browser on the front door.
fn relocate(headers: &mut axum::http::HeaderMap, host: &str, port: u16) {
    let Some(location) = headers
        .get(header::LOCATION)
        .and_then(|value| value.to_str().ok())
    else {
        return;
    };
    let rewritten = ["localhost", "127.0.0.1"].iter().find_map(|name| {
        location
            .strip_prefix(&format!("http://{name}:{port}"))
            .filter(|rest| rest.is_empty() || rest.starts_with(['/', '?', '#']))
            .map(|rest| format!("http://{host}{rest}"))
    });
    if let Some(value) = rewritten.and_then(|text| HeaderValue::from_str(&text).ok()) {
        headers.insert(header::LOCATION, value);
    }
}

/// An answer written by the front door itself: plain text, marked with [`MARK`].
fn answer(status: StatusCode, text: &str) -> Response {
    (
        status,
        [
            (header::CONTENT_TYPE, "text/plain; charset=utf-8"),
            (header::HeaderName::from_static(MARK), "1"),
        ],
        format!("ariane proxy: {text}\n"),
    )
        .into_response()
}

/// Requests sent through a front door bound to a free port, to upstreams
/// bound to free ports.
#[cfg(test)]
mod tests {
    use super::*;
    use axum::routing::get;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    /// Serves `router` on a free port of `127.0.0.1` and returns the port.
    async fn spawn(router: Router) -> u16 {
        let listener = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .await
            .unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        port
    }

    /// A front door whose only route is `app.local` to `port`.
    async fn door(port: u16) -> u16 {
        let route = Route {
            host: "app.local".to_owned(),
            port,
            node: Some(crate::tree::NodePath::new("p/app")),
        };
        let lookup: Lookup = Arc::new(move |host| (host == route.host).then(|| route.clone()));
        spawn(router(lookup)).await
    }

    /// Sends a raw HTTP/1.1 request to `port` and returns the whole answer.
    async fn raw(port: u16, request: &str) -> String {
        let mut stream = tokio::net::TcpStream::connect((Ipv4Addr::LOCALHOST, port))
            .await
            .unwrap();
        stream.write_all(request.as_bytes()).await.unwrap();
        let mut answer = String::new();
        stream.read_to_string(&mut answer).await.unwrap();
        answer
    }

    #[tokio::test]
    async fn a_request_reaches_the_upstream_with_its_own_address_as_host() {
        let upstream = spawn(Router::new().route(
            "/echo",
            get(|request: Request| async move {
                let header = |name: &str| {
                    request
                        .headers()
                        .get(name)
                        .and_then(|value| value.to_str().ok())
                        .unwrap_or("")
                        .to_owned()
                };
                format!(
                    "{} {} {}",
                    request.uri(),
                    header("host"),
                    header("x-forwarded-host")
                )
            }),
        ))
        .await;
        let door = door(upstream).await;
        let answer = raw(
            door,
            "GET /echo?a=1 HTTP/1.1\r\nHost: App.local\r\nConnection: close\r\n\r\n",
        )
        .await;
        assert!(answer.starts_with("HTTP/1.1 200"), "{answer}");
        assert!(
            answer.ends_with(&format!("/echo?a=1 localhost:{upstream} app.local")),
            "{answer}"
        );
    }

    #[test]
    fn a_redirect_to_the_upstream_comes_back_to_the_name() {
        let location = |value: &'static str| {
            let mut headers = axum::http::HeaderMap::new();
            headers.insert(header::LOCATION, HeaderValue::from_static(value));
            relocate(&mut headers, "app.local", 3000);
            headers[header::LOCATION].to_str().unwrap().to_owned()
        };
        assert_eq!(
            location("http://localhost:3000/login?x"),
            "http://app.local/login?x"
        );
        assert_eq!(location("http://127.0.0.1:3000"), "http://app.local");
        assert_eq!(
            location("http://localhost:30001/x"),
            "http://localhost:30001/x"
        );
        assert_eq!(location("/relative"), "/relative");
    }

    #[tokio::test]
    async fn an_unknown_name_and_a_silent_upstream_get_marked_answers() {
        let closed = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .await
            .unwrap();
        let port = closed.local_addr().unwrap().port();
        drop(closed);
        let door = door(port).await;
        let unknown = raw(
            door,
            "GET / HTTP/1.1\r\nHost: x.local\r\nConnection: close\r\n\r\n",
        )
        .await;
        assert!(unknown.starts_with("HTTP/1.1 404"), "{unknown}");
        assert!(unknown.contains(&format!("{MARK}: 1")), "{unknown}");
        let silent = raw(
            door,
            "GET / HTTP/1.1\r\nHost: app.local\r\nConnection: close\r\n\r\n",
        )
        .await;
        assert!(silent.starts_with("HTTP/1.1 502"), "{silent}");
        assert!(silent.contains("node p/app"), "{silent}");
    }

    #[tokio::test]
    async fn an_upgraded_connection_carries_bytes_both_ways() {
        let upstream = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .await
            .unwrap();
        let upstream_port = upstream.local_addr().unwrap().port();
        tokio::spawn(async move {
            let (mut socket, _) = upstream.accept().await.unwrap();
            let mut buffer = vec![0; 1024];
            let read = socket.read(&mut buffer).await.unwrap();
            assert!(String::from_utf8_lossy(&buffer[..read]).contains("upgrade: websocket"));
            socket
                .write_all(b"HTTP/1.1 101 Switching Protocols\r\nConnection: Upgrade\r\nUpgrade: websocket\r\n\r\n")
                .await
                .unwrap();
            let read = socket.read(&mut buffer).await.unwrap();
            let mut echoed = b"echo:".to_vec();
            echoed.extend_from_slice(&buffer[..read]);
            socket.write_all(&echoed).await.unwrap();
        });
        let door = door(upstream_port).await;
        let mut stream = tokio::net::TcpStream::connect((Ipv4Addr::LOCALHOST, door))
            .await
            .unwrap();
        stream
            .write_all(b"GET /hmr HTTP/1.1\r\nHost: app.local\r\nConnection: Upgrade\r\nUpgrade: websocket\r\n\r\n")
            .await
            .unwrap();
        let mut buffer = vec![0; 1024];
        let read = stream.read(&mut buffer).await.unwrap();
        assert!(String::from_utf8_lossy(&buffer[..read]).starts_with("HTTP/1.1 101"));
        stream.write_all(b"ping").await.unwrap();
        let read = stream.read(&mut buffer).await.unwrap();
        assert_eq!(&buffer[..read], b"echo:ping");
    }
}
