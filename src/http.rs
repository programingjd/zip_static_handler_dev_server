#[derive(Debug)]
pub enum OwnedOrStatic {
    Owned(Vec<u8>),
    Static(&'static [u8]),
}

impl Clone for OwnedOrStatic {
    fn clone(&self) -> Self {
        match self {
            OwnedOrStatic::Owned(vec) => OwnedOrStatic::Owned(vec.clone()),
            OwnedOrStatic::Static(slice) => OwnedOrStatic::Static(slice),
        }
    }
}

impl AsRef<[u8]> for OwnedOrStatic {
    fn as_ref(&self) -> &[u8] {
        match self {
            OwnedOrStatic::Owned(vec) => vec,
            OwnedOrStatic::Static(slice) => slice,
        }
    }
}

pub mod headers {
    use crate::http::OwnedOrStatic;

    pub const ALLOW: &[u8] = b"allow";
    pub const CACHE_CONTROL: &[u8] = b"cache-control";
    pub const CONTENT_ENCODING: &[u8] = b"content-encoding";
    pub const CONTENT_LENGTH: &[u8] = b"content-length";
    pub const CONTENT_TYPE: &[u8] = b"content-type";
    pub const COEP: &[u8] = b"cross-origin-embedder-policy";
    pub const COOP: &[u8] = b"cross-origin-opener-policy";
    pub const CORP: &[u8] = b"cross-origin-resource-policy";
    pub const CSP: &[u8] = b"content-security-policy";
    pub const ETAG: &[u8] = b"etag";
    pub const IF_MATCH: &[u8] = b"if-match";
    pub const IF_NONE_MATCH: &[u8] = b"if-none-match";
    pub const LOCATION: &[u8] = b"location";
    pub const HSTS: &[u8] = b"strict-transport-security";
    pub const SERVICE_WORKER_ALLOWED: &[u8] = b"service-worker-allowed";
    // pub const WWW_AUTHENTICATE: &[u8] = b"www-authenticate";
    pub const X_CONTENT_TYPE_OPTIONS: &[u8] = b"x-content-type-options";
    pub const X_FRAME_OPTIONS: &[u8] = b"x-frame-options";
    pub const X_XSS_PROTECTION: &[u8] = b"x-xss-protection";

    pub trait Headers {
        fn append(&mut self, key: &[u8], value: &[u8]) -> &mut Self;
    }

    #[derive(Debug)]
    pub struct Line {
        pub key: &'static [u8],
        pub value: OwnedOrStatic,
    }

    impl Line {
        pub fn with_array_ref_value<const N: usize>(
            key: &'static [u8],
            value: &'static [u8; N],
        ) -> Self {
            Self {
                key,
                value: OwnedOrStatic::Static(value.as_slice()),
            }
        }
        pub fn with_slice_value(key: &'static [u8], value: &'static [u8]) -> Self {
            Self {
                key,
                value: OwnedOrStatic::Static(value),
            }
        }
        pub fn with_owned_value(key: &'static [u8], value: Vec<u8>) -> Self {
            Self {
                key,
                value: OwnedOrStatic::Owned(value),
            }
        }
    }

    impl AsRef<Line> for Line {
        fn as_ref(&self) -> &Line {
            self
        }
    }

    impl Clone for Line {
        fn clone(&self) -> Self {
            Self {
                key: self.key,
                value: self.value.clone(),
            }
        }
    }

    impl From<(&'static [u8], &'static [u8])> for Line {
        fn from(value: (&'static [u8], &'static [u8])) -> Self {
            Self {
                key: value.0,
                value: OwnedOrStatic::Static(value.1),
            }
        }
    }

    impl<const N: usize> From<(&'static [u8], &'static [u8; N])> for Line {
        fn from(value: (&'static [u8], &'static [u8; N])) -> Self {
            Self {
                key: value.0,
                value: OwnedOrStatic::Static(value.1.as_slice()),
            }
        }
    }
}

pub mod response {
    pub enum StatusCode {
        OK,
        NoContent,
        NotModified,
        TemporaryRedirect,
        PermanentRedirect,
        BadRequest,
        Unauthorized,
        Forbidden,
        NotFound,
        MethodNotAllowed,
        PreconditionFailed,
        RequestTooLarge,
        InternalServerError,
    }

    impl From<StatusCode> for u16 {
        fn from(value: StatusCode) -> Self {
            match value {
                StatusCode::OK => 200,
                StatusCode::NoContent => 204,
                StatusCode::NotModified => 304,
                StatusCode::TemporaryRedirect => 307,
                StatusCode::PermanentRedirect => 308,
                StatusCode::BadRequest => 400,
                StatusCode::Unauthorized => 401,
                StatusCode::Forbidden => 403,
                StatusCode::NotFound => 404,
                StatusCode::MethodNotAllowed => 405,
                StatusCode::PreconditionFailed => 412,
                StatusCode::RequestTooLarge => 413,
                StatusCode::InternalServerError => 500,
            }
        }
    }
}

pub mod method {
    pub const HEAD: &[u8] = b"HEAD";
    pub const GET: &[u8] = b"GET";
    // pub const OPTIONS: &[u8] = b"OPTIONS";
}

pub mod request {
    use crate::http::headers::Line;
    use crate::http::response::StatusCode;

    pub trait Request<R> {
        fn method(&self) -> &[u8];
        fn path(&self) -> &[u8];
        fn first_header_value(&self, key: &'static [u8]) -> Option<&[u8]>;
        fn response<'b>(
            self,
            code: StatusCode,
            headers: impl Iterator<Item = &'b Line>,
            body: Option<&'b [u8]>,
        ) -> R;
    }
}
