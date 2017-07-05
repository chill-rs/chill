use {Error, futures, reqwest, std, transport, url};
use futures::Future;
use reqwest::{Method, StatusCode};
use std::sync::Mutex;
use transport::Transport;

#[derive(Debug)]
pub struct NetTransport {
    state: Mutex<NetTransportState>,
    worker_thread: std::thread::JoinHandle<()>,
}

#[derive(Debug)]
struct NetTransportState {
    http_client: reqwest::Client,
    server_url: url::Url,
    request_tx: std::sync::mpsc::Sender<WorkerTask>,
}

#[derive(Debug)]
struct WorkerTask {
    request_builder: reqwest::RequestBuilder,
    response_tx: futures::sync::oneshot::Sender<WorkerResult>,
}

type WorkerResult = Result<reqwest::Response, reqwest::Error>;

impl NetTransport {
    pub fn new(server_url: url::Url) -> Result<Self, Error> {

        // The purpose of the worker thread and channels is carry out HTTP
        // requests in the background and is exists only because there's no good
        // means in the Rust ecosystem for combining asynchronous client-side
        // HTTP with TLS. I.e., reqwest is the best HTTP client crate to use,
        // but it doesn't support asynchronous I/O.

        let (request_tx, request_rx) = std::sync::mpsc::channel::<WorkerTask>();

        let worker_thread = std::thread::spawn(move || while let Ok(task) = request_rx.recv() {
            task.response_tx
                .send(task.request_builder.send())
                .unwrap_or(());
        });

        let state = Mutex::new(NetTransportState {
            http_client: reqwest::Client::new().map_err(|e| {
                ("Failed to construct HTTP client", e)
            })?,
            server_url: server_url,
            request_tx: request_tx,
        });

        Ok(NetTransport {
            state: state,
            worker_thread: worker_thread,
        })
    }
}

impl Transport for NetTransport {
    type Request = Request;
    fn request(&self, method: Method, path: &str) -> Self::Request {

        let state = self.state.lock().unwrap();

        let u = {
            let mut u = state.server_url.clone();
            u.set_path(path);
            u
        };

        Request {
            request_builder: state.http_client.request(method, u),
            request_tx: state.request_tx.clone(),
        }
    }
}

#[derive(Debug)]
pub struct Request {
    request_builder: reqwest::RequestBuilder,
    request_tx: std::sync::mpsc::Sender<WorkerTask>,
}

impl transport::Request for Request {
    type Response = Response;

    type Future = futures::future::AndThen<
        futures::future::MapErr<
            futures::sync::oneshot::Receiver<WorkerResult>,
            fn(futures::sync::oneshot::Canceled) -> Error,
        >,
        Result<Response, Error>,
        fn(WorkerResult) -> Result<Response, Error>,
    >;

    fn send(self) -> Self::Future {

        let (response_tx, response_rx) = futures::sync::oneshot::channel();

        let task = WorkerTask {
            request_builder: self.request_builder,
            response_tx: response_tx,
        };

        self.request_tx.send(task).unwrap();

        fn f1(_: futures::sync::oneshot::Canceled) -> Error {
            Error::from(
                "Worker thread canceled and did not complete the HTTP request",
            )
        }

        fn f2(worker_result: WorkerResult) -> Result<Response, Error> {
            match worker_result {
                Ok(response) => Ok(Response { status_code: *response.status() }),
                Err(e) => Err(Error::from(("HTTP request failed", e))),
            }
        }

        response_rx.map_err(f1 as _).and_then(f2 as _)
    }
}

#[derive(Debug)]
pub struct Response {
    status_code: StatusCode,
}

impl transport::Response for Response {
    fn status_code(&self) -> StatusCode {
        self.status_code
    }
}
