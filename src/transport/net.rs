use {Error, futures, reqwest, std, transport, url};
use error::TransportErrorKind;
use futures::Future;
use reqwest::{Method, StatusCode};
use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use transport::Transport;

const MAX_IDLE_WORKERS: usize = 1;

#[derive(Debug)]
pub struct NetTransport {
    client_state: Mutex<ClientState>,
    task_queue: Arc<TaskQueue>,
}

#[derive(Debug)]
struct ClientState {
    http_client: reqwest::Client,
    server_url: url::Url,
}

#[derive(Debug)]
struct TaskQueue {
    wakeup: Condvar,
    mutexed: Mutex<TaskQueueMutexed>,
}

#[derive(Debug)]
struct TaskQueueMutexed {
    max_idle_workers: usize,
    n_idle_workers: usize,
    queue: VecDeque<TaskItem>,
}

#[derive(Debug)]
struct TaskItem {
    request_builder: reqwest::RequestBuilder,
    response_tx: futures::sync::oneshot::Sender<TaskResult>,
}

type TaskResult = Result<Response, Error>;

impl NetTransport {
    pub fn new(server_url: url::Url) -> Result<Self, Error> {

        // The rationale for using worker threads is to carry out HTTP requests
        // asynchronously, in the background, while using Reqwest's synchronous
        // HTTP client.
        //
        // The number of parallel HTTP requests is limited by the number of the
        // worker threads, which grows without bound, though the maximum number
        // of idle threads we keep around is one.
        //
        // Someday we'll replace worker threads with an asynchronous HTTP
        // client.

        Ok(NetTransport {
            client_state: Mutex::new(ClientState {
                http_client: reqwest::Client::new().map_err(|e| {
                    Error::Transport { kind: TransportErrorKind::Reqwest(e) }
                })?,
                server_url: server_url,
            }),
            task_queue: Arc::new(TaskQueue::new()),
        })
    }
}

impl Transport for NetTransport {
    type Request = Request;
    fn request(&self, method: Method, path: &str) -> Self::Request {

        let client_state = self.client_state.lock().unwrap();

        let u = {
            let mut u = client_state.server_url.clone();
            u.set_path(path);
            u
        };

        Request {
            request_builder: client_state.http_client.request(method, u),
            task_queue: self.task_queue.clone(),
        }
    }
}

#[derive(Debug)]
pub struct Request {
    request_builder: reqwest::RequestBuilder,
    task_queue: Arc<TaskQueue>,
}

impl transport::Request for Request {
    type Response = Response;

    type Future = futures::future::AndThen<
        futures::future::MapErr<
            futures::sync::oneshot::Receiver<TaskResult>,
            fn(futures::sync::oneshot::Canceled) -> Error,
        >,
        Result<Response, Error>,
        fn(TaskResult) -> Result<Response, Error>,
    >;

    fn send(self) -> Self::Future {

        let (response_tx, response_rx) = futures::sync::oneshot::channel();
        TaskQueue::push(&self.task_queue, self.request_builder, response_tx);

        fn f1(_: futures::sync::oneshot::Canceled) -> Error {
            Error::TransportWorker
        }

        fn f2(worker_result: TaskResult) -> Result<Response, Error> {
            worker_result
        }

        response_rx.map_err(f1 as _).and_then(f2 as _)
    }
}

#[derive(Debug)]
pub struct Response {
    status_code: StatusCode,
    headers: reqwest::header::Headers,
    body: Vec<u8>,
}

impl transport::Response for Response {
    fn status_code(&self) -> StatusCode {
        self.status_code
    }
}

impl TaskQueue {
    fn new() -> Self {
        TaskQueue {
            wakeup: Condvar::new(),
            mutexed: Mutex::new(TaskQueueMutexed {
                max_idle_workers: MAX_IDLE_WORKERS,
                n_idle_workers: 0,
                queue: VecDeque::new(),
            }),
        }
    }

    fn push(
        task_queue: &Arc<Self>,
        request_builder: reqwest::RequestBuilder,
        response_tx: futures::sync::oneshot::Sender<TaskResult>,
    ) {

        let task = TaskItem {
            request_builder: request_builder,
            response_tx: response_tx,
        };

        let mut m = task_queue.mutexed.lock().unwrap();
        m.queue.push_back(task);

        if 0 < m.n_idle_workers {
            task_queue.wakeup.notify_one();
            return;
        }

        let task_queue = task_queue.clone();
        std::thread::spawn(move || {
            let mut m = task_queue.mutexed.lock().unwrap();
            loop {
                match m.queue.pop_front() {
                    None => {
                        if m.max_idle_workers <= m.n_idle_workers {
                            return; // quit this worker
                        }
                        m.n_idle_workers += 1;
                        m = task_queue.wakeup.wait(m).unwrap();
                        m.n_idle_workers -= 1;
                    }
                    Some(task) => {
                        drop(m);
                        let result = match task.request_builder.send() {
                            Ok(mut response) => {

                                // Long-term, the body should be a future that
                                // would lend itself to stream-parsing. The Rust
                                // ecosystem is not there, yet. So for now, we
                                // read in the whole body and buffer it.

                                use std::io::Read;

                                let mut body = Vec::new();
                                response
                                    .read_to_end(&mut body)
                                    .map_err(|e| {
                                        Error::Io {
                                            description: "Failed to read HTTP response body",
                                            cause: e,
                                        }
                                    })
                                    .map(|_| {
                                        Response {
                                            status_code: *response.status(),
                                            headers: response.headers().clone(),
                                            body: body,
                                        }
                                    })
                            }
                            Err(e) => Err(Error::Transport { kind: TransportErrorKind::Reqwest(e) }),
                        };
                        m = task_queue.mutexed.lock().unwrap();
                        task.response_tx.send(result).unwrap();
                    }
                }
            }
        });
    }

    fn signal_shutdown(&self) {
        {
            let mut m = self.mutexed.lock().unwrap();
            m.max_idle_workers = 0;
        }
        self.wakeup.notify_all();
    }
}

impl Drop for TaskQueue {
    fn drop(&mut self) {
        self.signal_shutdown();
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn transport_supports_concurrent_requests() {

        use super::*;
        use {futures, hyper, std};
        use futures::Future;
        use std::collections::HashMap;
        use std::sync::{Arc, Mutex};
        use transport::{Method, Request, Response, StatusCode, Transport};

        // Start up an HTTP server that will hold requests until signaled. This
        // allows us to queue up client requests and ensure that they all get
        // handled and nothing blocks.

        struct HttpService {
            state: Arc<Mutex<ServiceState>>,
        }

        struct ServiceState {
            signals: HashMap<String, futures::sync::oneshot::Sender<hyper::server::Response>>,
            slots: HashMap<String, futures::sync::oneshot::Receiver<hyper::server::Response>>,
        }

        impl ServiceState {
            fn start_request(&mut self, transport: &NetTransport, key: &str) -> <super::Request as Request>::Future {
                let (tx, rx) = futures::sync::oneshot::channel();
                self.slots.insert(String::from(key), rx);
                self.signals.insert(String::from(key), tx);
                transport.request(Method::Get, key).send()
            }

            fn signal_request(&mut self, key: &str) {
                self.signals
                    .remove(key)
                    .unwrap()
                    .send(hyper::server::Response::new().with_status(
                        hyper::StatusCode::Ok,
                    ))
                    .unwrap();
            }
        }

        impl hyper::server::Service for HttpService {
            type Request = hyper::server::Request;
            type Response = hyper::server::Response;
            type Error = hyper::Error;
            type Future = futures::BoxFuture<Self::Response, Self::Error>;

            fn call(&self, request: hyper::server::Request) -> Self::Future {

                let path = {
                    let mut p = request.uri().path();
                    if p.starts_with("/") {
                        p = &p[1..];
                    }
                    p
                };

                let mut state = self.state.lock().unwrap();
                let rx = state.slots.remove(path).unwrap();
                rx.map_err(|_| hyper::Error::Timeout).boxed()
            }
        }

        let (server_url, service_state, shutdown_tx) = {

            let (shutdown_tx, shutdown_rx) = futures::sync::oneshot::channel();
            let (server_info_tx, server_info_rx) = std::sync::mpsc::channel();

            std::thread::spawn(move || {
                let addr = "127.0.0.1:0".parse().unwrap();
                let service_state = Arc::new(Mutex::new(ServiceState {
                    slots: HashMap::new(),
                    signals: HashMap::new(),
                }));
                let service_state_clone = service_state.clone();
                let server = hyper::server::Http::new()
                    .bind(&addr, move || {
                        Ok(HttpService { state: service_state.clone() })
                    })
                    .unwrap();
                server_info_tx
                    .send((server.local_addr().unwrap(), service_state_clone))
                    .unwrap();
                server.run_until(shutdown_rx.map_err(|_| ())).unwrap();
            });

            let (server_addr, service_state) = server_info_rx.recv().unwrap();
            let server_url: url::Url = format!("http://{}", server_addr).parse().unwrap();
            (server_url, service_state, shutdown_tx)
        };

        let transport = NetTransport::new(server_url).unwrap();

        // The important thing here is that we start two requests, in order,
        // then have the HTTP server handle and send a response for only the
        // second request. The client should get the response to that request
        // even though the first request is still blocking. This implies the
        // transport handles requests concurrently.

        let _r_alpha = service_state.lock().unwrap().start_request(
            &transport,
            "alpha",
        );
        let r_bravo = service_state.lock().unwrap().start_request(
            &transport,
            "bravo",
        );

        service_state.lock().unwrap().signal_request("bravo");

        let response = r_bravo.wait().unwrap();
        assert_eq!(response.status_code(), StatusCode::Ok);

        shutdown_tx.send(()).unwrap();
    }
}
