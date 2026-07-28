use http::{Request, header::HeaderMap};
use std::task::{Context, Poll};
use tower::{Layer, Service};

#[derive(Debug)]
pub struct RequestHeadersLayer {
    headers: HeaderMap,
}

impl RequestHeadersLayer {
    pub fn new(headers: HeaderMap) -> Self {
        RequestHeadersLayer { headers }
    }
}

impl<S> Layer<S> for RequestHeadersLayer {
    type Service = RequestHeaders<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RequestHeaders {
            inner,
            headers: self.headers.clone(),
        }
    }
}

#[derive(Clone)]
pub struct RequestHeaders<S> {
    inner: S,
    headers: HeaderMap,
}

impl<Body, S> Service<Request<Body>> for RequestHeaders<S>
where
    S: Service<Request<Body>>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<Body>) -> Self::Future {
        if !self.headers.is_empty() {
            req.headers_mut().extend(self.headers.clone());
        }

        self.inner.call(req)
    }
}
