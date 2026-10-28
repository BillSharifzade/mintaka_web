//! Everything the router doesn't name: manifest routes, `/` by `Accept-Language`, `/{page}` →
//! `/{page}/`, curl mode and the 404 page (docs/CONTRACTS.md §4.1, §4.3, §4.4).

use std::sync::Arc;
use std::time::Instant;

use axum::body::Body;
use axum::extract::{Request, State};
use axum::response::Response;
use http::header::{
    ACCEPT_ENCODING, ACCEPT_LANGUAGE, CACHE_CONTROL, CONTENT_LANGUAGE, CONTENT_TYPE, IF_NONE_MATCH,
    LOCATION, VARY,
};
use http::{HeaderMap, HeaderValue, StatusCode};

use super::{GET_HEAD, RouteName, curl, internal_error, is_read, mark, method_not_allowed};
use crate::app::AppState;
use crate::assets::Variants;
use crate::negotiate;

pub(crate) async fn handler(State(app): State<Arc<AppState>>, req: Request) -> Response {
    let site = &app.site;
    let path = req.uri().path();
    let headers = req.headers();
    let read = is_read(req.method());

    if curl::is_cli_agent(headers)
        && let Some(target) = curl::target(site, path, headers)
    {
        let routed = Instant::now();
        let response = if read { curl::respond(&app, &req, &target) } else { method_not_allowed(GET_HEAD) };
        return mark(response, RouteName::Named(target.label()), routed);
    }

    if let Some((index, route)) = site.route(path) {
        let routed = Instant::now();
        let response = if read { serve(&route.variants, headers) } else { method_not_allowed(GET_HEAD) };
        return mark(response, RouteName::Static(index), routed);
    }

    if path == "/" {
        let routed = Instant::now();
        let response = if read {
            let lang = negotiate::language(headers.get_all(ACCEPT_LANGUAGE), &site.langs, &site.default_lang);
            site.root(lang).map_or_else(internal_error, |variants| serve(variants, headers))
        } else {
            method_not_allowed(GET_HEAD)
        };
        return mark(response, RouteName::Named("/"), routed);
    }

    if site.needs_slash(path) {
        let routed = Instant::now();
        let response = if read { add_slash(path, req.uri().query()) } else { method_not_allowed(GET_HEAD) };
        return mark(response, RouteName::Named("/{page} -> /{page}/"), routed);
    }

    let routed = Instant::now();
    mark(not_found(&app, path, headers), RouteName::Named("404"), routed)
}

/// Picks the encoding, answers `If-None-Match`, clones the pre-built headers and body handle.
fn serve(variants: &Variants, headers: &HeaderMap) -> Response {
    let coding = negotiate::coding(headers.get_all(ACCEPT_ENCODING), variants.br.is_some(), variants.gzip.is_some());
    let variant = variants.pick(coding);
    let (status, body, prebuilt) = if negotiate::none_match(headers.get_all(IF_NONE_MATCH), &variant.etag) {
        (StatusCode::NOT_MODIFIED, Body::empty(), &variant.not_modified)
    } else {
        (StatusCode::OK, Body::from(variant.body.clone()), &variant.headers)
    };
    let mut response = Response::new(body);
    *response.status_mut() = status;
    *response.headers_mut() = prebuilt.clone();
    response
}

/// `301` to the same path with a trailing slash; the query string is kept. `path` is a manifest
/// page minus its slash, never arbitrary input.
fn add_slash(path: &str, query: Option<&str>) -> Response {
    let location = match query {
        Some(query) => format!("{path}/?{query}"),
        None => format!("{path}/"),
    };
    let Ok(location) = HeaderValue::try_from(location) else { return internal_error() };
    let mut response = Response::new(Body::empty());
    *response.status_mut() = StatusCode::MOVED_PERMANENTLY;
    response.headers_mut().insert(LOCATION, location);
    response.headers_mut().insert(CACHE_CONTROL, HeaderValue::from_static("public, max-age=3600"));
    response
}

/// The 404 page in the path's language (else the negotiated one), with the closest page route
/// as a suggestion. The requested path itself is never written into the page.
fn not_found(app: &AppState, path: &str, headers: &HeaderMap) -> Response {
    let site = &app.site;
    let prefixed = site.lang_of_path(path);
    let lang = prefixed.unwrap_or_else(|| {
        negotiate::language(headers.get_all(ACCEPT_LANGUAGE), &site.langs, &site.default_lang)
    });
    let Some(page) = site.not_found(lang) else {
        let mut response = super::no_store(StatusCode::NOT_FOUND, "text/plain; charset=utf-8", "not found\n");
        response.headers_mut().insert(CACHE_CONTROL, HeaderValue::from_static("no-cache"));
        return response;
    };
    let body = page.render(site.suggest(path, &page.lang));
    let mut response = Response::new(Body::from(body));
    *response.status_mut() = StatusCode::NOT_FOUND;
    let h = response.headers_mut();
    h.insert(CONTENT_TYPE, HeaderValue::from_static("text/html; charset=utf-8"));
    h.insert(CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    h.insert(CONTENT_LANGUAGE, page.lang_header.clone());
    let vary = match (prefixed.is_none(), curl::is_curl_path(site, path)) {
        (true, true) => Some("Accept-Language, User-Agent"),
        (true, false) => Some("Accept-Language"),
        (false, true) => Some("User-Agent"),
        (false, false) => None,
    };
    if let Some(vary) = vary {
        h.insert(VARY, HeaderValue::from_static(vary));
    }
    response
}
