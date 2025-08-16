use crate::http::headers::{
    CONTENT_ENCODING, CONTENT_LENGTH, ETAG, IF_MATCH, IF_NONE_MATCH, LOCATION, Line,
};
use crate::http::method;
use crate::http::request::Request;
use crate::http::response::StatusCode;
use crate::path::{extension, filename};
use crc32fast::hash;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

pub struct Handler<T: HeaderSelector> {
    pub prefix: &'static str,
    pub header_selector: T,
}

impl<T: HeaderSelector> Handler<T> {
    pub async fn handle<Resp, Req: Request<Resp>>(&self, request: Req) -> Resp {
        let method = request.method();
        let path = String::from_utf8_lossy(request.path());
        if let Some(value) = request.first_header_value(CONTENT_LENGTH)
            && value != b"0"
        {
            return request.response(
                StatusCode::BadRequest,
                self.header_selector.error_headers().iter(),
                None::<&[u8]>,
            );
        }
        let is_get = match method {
            method::GET => true,
            method::HEAD => false,
            _ => {
                return request.response(
                    StatusCode::MethodNotAllowed,
                    self.header_selector.error_headers().iter(),
                    None::<&[u8]>,
                );
            }
        };
        let path = path.strip_prefix('/').unwrap_or(&path);
        if let Some(path) = path.strip_prefix(self.prefix) {
            let path = path.strip_prefix('/').unwrap_or(path);
            if let Some(path_without_trailing_slash) = path.strip_suffix('/') {
                if let Some(HeadersAndCompression { mut headers, .. }) = self
                    .header_selector
                    .headers_for_extension(path_without_trailing_slash, "308")
                {
                    let location = format!("/{}{path_without_trailing_slash}", self.prefix);
                    headers.push(Line::with_owned_value(LOCATION, location.into_bytes()));
                    return request.response(StatusCode::PermanentRedirect, headers.iter(), None);
                } else {
                    return request.response(
                        StatusCode::NotFound,
                        self.header_selector.error_headers().iter(),
                        None::<&[u8]>,
                    );
                }
            }
            if path.starts_with('.') || path.contains("/.") {
                return request.response(
                    StatusCode::NotFound,
                    self.header_selector.error_headers().iter(),
                    None::<&[u8]>,
                );
            }
            let mut candidates: Vec<String> = vec![];
            if path.is_empty() {
                candidates.push("index.html".to_string());
            } else {
                candidates.push(path.to_string());
                candidates.push(format!("{}.html", &path));
            };
            if path.is_empty() {
                candidates.push("index.307".to_string());
                candidates.push("index.308".to_string());
            } else {
                candidates.push(format!("{}.307", &path));
                candidates.push(format!("{}.308", &path));
            }
            for path in candidates {
                let filename = filename(&path);
                let extension = extension(filename);
                if let Some(HeadersAndCompression {
                    mut headers,
                    compressible,
                    redirection,
                }) = self
                    .header_selector
                    .headers_for_extension(filename, extension)
                {
                    let meta = if compressible {
                        if let Ok(mut file) = File::open(format!("{path}.br")).await {
                            let len = file
                                .metadata()
                                .await
                                .map(|it| it.len() as usize)
                                .unwrap_or(4096_usize);
                            let mut buf = Vec::with_capacity(len);
                            if file.read_to_end(&mut buf).await.is_ok() {
                                let crc32 = hash(&buf);
                                let etag = format!("{crc32:x}");
                                Some((true, etag, buf))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    let meta = match meta {
                        Some(it) => Some(it),
                        None => {
                            if let Ok(mut file) = File::open(&path).await {
                                let len = file
                                    .metadata()
                                    .await
                                    .map(|it| it.len() as usize)
                                    .unwrap_or(4096_usize);
                                let mut buf = Vec::with_capacity(len);
                                if file.read_to_end(&mut buf).await.is_ok() {
                                    let crc32 = hash(&buf);
                                    let etag = format!("{crc32:x}");
                                    Some((false, etag, buf))
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        }
                    };
                    if let Some((compressed, etag, content)) = meta {
                        if redirection {
                            headers.push(Line::with_slice_value(CONTENT_LENGTH, b"0"));
                            let end = content
                                .iter()
                                .position(|&b| b.is_ascii_whitespace())
                                .unwrap_or(content.len());
                            headers.push(Line::with_owned_value(LOCATION, content[..end].into()));
                        } else {
                            headers.push(Line::with_owned_value(
                                CONTENT_LENGTH,
                                format!("{}", content.len()).into_bytes(),
                            ));
                            if compressed {
                                headers.push(Line::with_array_ref_value(CONTENT_ENCODING, b"br"));
                            }
                        }
                        let etag = if extension == "308" { None } else { Some(etag) };
                        if let Some(ref etag) = etag {
                            headers.push(Line::with_owned_value(ETAG, etag.as_bytes().to_vec()));
                            let etag = Some(etag.as_bytes());
                            let none_match = request.first_header_value(IF_NONE_MATCH);
                            let if_match = request.first_header_value(IF_MATCH);
                            if none_match.is_some() && none_match == etag {
                                return request.response(
                                    StatusCode::NotModified,
                                    headers.iter(),
                                    None::<&[u8]>,
                                );
                            } else if if_match.is_some() && if_match != etag {
                                return request.response(
                                    StatusCode::PreconditionFailed,
                                    headers.iter(),
                                    None::<&[u8]>,
                                );
                            }
                        }
                        return if redirection {
                            if etag.is_some() {
                                request.response(
                                    StatusCode::TemporaryRedirect,
                                    headers.iter(),
                                    None,
                                )
                            } else {
                                request.response(
                                    StatusCode::PermanentRedirect,
                                    headers.iter(),
                                    None,
                                )
                            }
                        } else {
                            request.response(
                                StatusCode::OK,
                                headers.iter(),
                                if is_get {
                                    Some(content.as_slice())
                                } else {
                                    None
                                },
                            )
                        };
                    }
                }
            }
        }
        request.response(
            StatusCode::NotFound,
            self.header_selector.error_headers().iter(),
            None::<&[u8]>,
        )
    }
}

pub trait HeaderSelector {
    fn headers_for_extension(
        &self,
        filename: &str,
        extension: &str,
    ) -> Option<HeadersAndCompression>;
    fn error_headers(&self) -> &'static [Line];
}

pub struct HeadersAndCompression {
    pub headers: Vec<Line>,
    pub compressible: bool,
    pub redirection: bool,
}
