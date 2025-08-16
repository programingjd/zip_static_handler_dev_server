use crate::http::headers::Line;
use crate::http::request::Request;
use crate::http::response::StatusCode;
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Empty, Full};
use hyper::body::Bytes;
use hyper::http::{HeaderName, HeaderValue};
use std::str::from_utf8;

type HyperResponse = hyper::Response<BoxBody<Bytes, hyper::Error>>;
type HyperRequest = hyper::Request<BoxBody<Bytes, hyper::Error>>;

pub struct RequestAdapter {
    pub inner: HyperRequest,
}

impl Request<HyperResponse> for RequestAdapter {
    fn method(&self) -> &[u8] {
        self.inner.method().as_str().as_bytes()
    }

    fn path(&self) -> &[u8] {
        self.inner.uri().path().as_bytes()
    }

    fn first_header_value(&self, key: &'static [u8]) -> Option<&[u8]> {
        from_utf8(key)
            .ok()
            .and_then(|key| self.inner.headers().get(key).map(|it| it.as_bytes()))
    }

    fn response<'a>(
        self,
        code: StatusCode,
        headers: impl Iterator<Item = &'a Line>,
        body: Option<&'a [u8]>,
    ) -> HyperResponse {
        let code: u16 = code.into();
        let mut builder = hyper::Response::builder().status(code);
        let map = builder.headers_mut().unwrap();
        headers.for_each(|line| {
            if let Ok(name) = HeaderName::from_bytes(line.key)
                && let Ok(value) = HeaderValue::from_bytes(line.value.as_ref())
            {
                map.append(name, value);
            }
        });
        let body = body.map(Self::full).unwrap_or_else(Self::empty);
        builder.body(body).unwrap()
    }
}

impl RequestAdapter {
    fn full(slice: impl AsRef<[u8]> + Send) -> BoxBody<Bytes, hyper::Error> {
        Full::new(Bytes::copy_from_slice(slice.as_ref()))
            .map_err(|never| match never {})
            .boxed()
    }
    fn empty() -> BoxBody<Bytes, hyper::Error> {
        Empty::<Bytes>::new()
            .map_err(|never| match never {})
            .boxed()
    }
}
