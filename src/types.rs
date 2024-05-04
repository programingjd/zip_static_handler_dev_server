use crate::handler::{HeaderSelector, HeadersAndCompression};
use crate::http::headers::{
    Line, ALLOW, CACHE_CONTROL, COEP, CONTENT_LENGTH, CONTENT_TYPE, COOP, CORP, CSP, HSTS,
    SERVICE_WORKER_ALLOWED, X_CONTENT_TYPE_OPTIONS, X_FRAME_OPTIONS, X_XSS_PROTECTION,
};
use lazy_static::lazy_static;

lazy_static! {
    pub static ref DEFAULT_HEADERS: Vec<Line> = {
        let headers/*: Vec<(&'static [u8], &'static [u8])>*/ = vec![
            (ALLOW, b"GET, HEAD".as_slice()).into(),
            (X_CONTENT_TYPE_OPTIONS, b"nosniff".as_slice()).into(),
            (X_FRAME_OPTIONS, b"DENY".as_slice()).into(),
            (X_XSS_PROTECTION, b"1; mode=block".as_slice()).into(),
            (CORP, b"same-site".as_slice()).into(),
            (COEP, b"crendentialless".as_slice()).into(),
            (COOP, b"same-origin".as_slice()).into(),
            (CSP, b"default-src 'self';script-src 'wasm-unsafe-eval';script-src-elem 'self' 'unsafe-inline';script-src-attr 'none';worker-src 'self' blob:;style-src 'self' 'unsafe-inline';img-src 'self' data: blob:;font-src 'self' data:;frame-src 'none';object-src 'none';base-uri 'none';frame-ancestors 'none';form-action 'none'".as_slice()).into(),
            (HSTS, b"max-age=63072000; includeSubDomains; preload".as_slice()).into(),
        ];
        headers
    };
    pub static ref ERROR_HEADERS: Vec<Line> = {
        let headers/*: Vec<(&'static [u8], &'static [u8])>*/ = vec![
            (ALLOW, b"GET, HEAD".as_slice()).into(),
            (CONTENT_LENGTH, b"0".as_slice()).into(),
            //(HSTS, b"max-age=63072000; includeSubDomains; preload".as_slice()),
        ];
        headers
    };
}

const CACHE_CONTROL_NO_CACHE: &[u8] =
    b"public,no-cache,max-age=0,must-revalidate;stale-if-error=3600";
const CACHE_CONTROL_REVALIDATE: &[u8] = b"public,max-age=3600,must-revalidate,stale-if-error=3600";
const CACHE_CONTROL_IMMUTABLE: &[u8] =
    b"public,max-age=86400,immutable,stale-while-revalidate=864000,stale-if-error=3600";

pub(crate) fn default_headers() -> impl Iterator<Item = &'static Line> {
    DEFAULT_HEADERS.iter()
}

pub(crate) fn default_error_headers() -> &'static [Line] {
    ERROR_HEADERS.as_slice()
}

pub(crate) fn headers_for_type(filename: &str, extension: &str) -> Option<HeadersAndCompression> {
    match extension {
        "html" | "htm" => Some(headers_and_compression(
            Some(b"text/html"),
            Some(CACHE_CONTROL_REVALIDATE),
            true,
        )),
        "css" => Some(headers_and_compression(
            Some(b"text/css"),
            Some(CACHE_CONTROL_REVALIDATE),
            true,
        )),
        "js" | "mjs" | "map" => Some(
            if filename.starts_with("service-worker.") || filename.starts_with("sw.") {
                let mut headers_and_compression = headers_and_compression(
                    Some(b"application/javascript"),
                    Some(CACHE_CONTROL_NO_CACHE),
                    true,
                );
                headers_and_compression
                    .headers
                    .push(Line::with_array_ref_value(SERVICE_WORKER_ALLOWED, b"/"));
                headers_and_compression
            } else {
                headers_and_compression(
                    Some(b"application/javascript"),
                    Some(CACHE_CONTROL_REVALIDATE),
                    true,
                )
            },
        ),
        "json" => Some(headers_and_compression(
            Some(b"application/json"),
            Some(CACHE_CONTROL_REVALIDATE),
            true,
        )),
        "txt" => Some(headers_and_compression(
            Some(b"text/plain"),
            Some(CACHE_CONTROL_REVALIDATE),
            true,
        )),
        "csv" => Some(headers_and_compression(
            Some(b"text/csv"),
            Some(CACHE_CONTROL_REVALIDATE),
            true,
        )),
        "md" => Some(headers_and_compression(
            Some(b"text/markdown"),
            Some(CACHE_CONTROL_REVALIDATE),
            true,
        )),
        "wasm" => Some(headers_and_compression(
            Some(b"application/wasm"),
            Some(CACHE_CONTROL_REVALIDATE),
            true,
        )),
        "woff2" => Some(headers_and_compression(
            Some(b"font/woff2"),
            Some(CACHE_CONTROL_REVALIDATE),
            false,
        )),
        "ico" => Some(headers_and_compression(
            Some(b"image/x-icon"),
            Some(CACHE_CONTROL_IMMUTABLE),
            true,
        )),
        "webp" => Some(headers_and_compression(
            Some(b"image/webp"),
            Some(CACHE_CONTROL_IMMUTABLE),
            false,
        )),
        "avif" => Some(headers_and_compression(
            Some(b"image/avif"),
            Some(CACHE_CONTROL_IMMUTABLE),
            false,
        )),
        "gif" => Some(headers_and_compression(
            Some(b"image/gif"),
            Some(CACHE_CONTROL_IMMUTABLE),
            false,
        )),
        "heif" => Some(headers_and_compression(
            Some(b"image/heif"),
            Some(CACHE_CONTROL_IMMUTABLE),
            false,
        )),
        "heic" => Some(headers_and_compression(
            Some(b"image/heic"),
            Some(CACHE_CONTROL_IMMUTABLE),
            false,
        )),
        "png" => Some(headers_and_compression(
            Some(b"image/png"),
            Some(CACHE_CONTROL_IMMUTABLE),
            false,
        )),
        "jpg" => Some(headers_and_compression(
            Some(b"image/jpeg"),
            Some(CACHE_CONTROL_IMMUTABLE),
            false,
        )),
        "mp3" => Some(headers_and_compression(
            Some(b"audio/mp3"),
            Some(CACHE_CONTROL_REVALIDATE),
            false,
        )),
        "mp4" => Some(headers_and_compression(
            Some(b"video/mp4"),
            Some(CACHE_CONTROL_REVALIDATE),
            false,
        )),
        "svg" => Some(headers_and_compression(
            Some(b"image/svg+xml"),
            Some(CACHE_CONTROL_IMMUTABLE),
            true,
        )),
        "pdf" => Some(headers_and_compression(
            Some(b"application/pdf"),
            Some(CACHE_CONTROL_REVALIDATE),
            true,
        )),
        "zip" => Some(headers_and_compression(
            Some(b"application/zip"),
            Some(CACHE_CONTROL_REVALIDATE),
            false,
        )),
        "307" => Some(headers_and_compression(
            None,
            Some(CACHE_CONTROL_REVALIDATE),
            false,
        )),
        "308" => Some(headers_and_compression(None, None, false)),
        _ => None,
    }
}

fn headers_and_compression(
    content_type: Option<&'static [u8]>,
    cache_control: Option<&'static [u8]>,
    compressible: bool,
) -> HeadersAndCompression {
    let default_headers = default_headers();
    let mut new_headers = vec![];
    if let Some(content_type) = content_type {
        new_headers.push(Line::with_slice_value(CONTENT_TYPE, content_type));
    }
    if let Some(cache_control) = cache_control {
        new_headers.push(Line::with_slice_value(CACHE_CONTROL, cache_control));
    }
    HeadersAndCompression {
        headers: if new_headers.is_empty() {
            default_headers.cloned().collect()
        } else {
            default_headers.cloned().chain(new_headers).collect()
        },
        compressible,
        redirection: content_type.is_none(),
    }
}

pub(crate) struct DefaultHeaderSelector;

impl HeaderSelector for DefaultHeaderSelector {
    fn headers_for_extension(
        &self,
        filename: &str,
        extension: &str,
    ) -> Option<HeadersAndCompression> {
        headers_for_type(filename, extension)
    }
    fn error_headers(&self) -> &'static [Line] {
        default_error_headers()
    }
}
