use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use extendr_api::prelude::*;

struct HttpClient {
    thread_pool: rayon::ThreadPool,
    agent: ureq::Agent,
}

#[derive(Clone, Copy, Default)]
enum HttpVerb {
    #[default]
    Get,
    Post,
    Put,
    Delete,
}

#[extendr]
impl HttpClient {
    fn new(num_threads: i32) -> HttpClient {
        let thread_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads as usize)
            .build()
            .expect("Unable to build the thread pool");
        let agent = ureq::agent();
        HttpClient { agent, thread_pool }
    }
}

#[derive(Default)]
struct ResponseContent {
    content: String,
}

struct Response {
    response_container: Arc<Mutex<Option<ResponseContent>>>,
}

#[extendr]
impl Response {
    fn poll(&self) -> bool {
        let response_container = self.response_container.lock().expect("POISONENENENE");
        response_container.is_some()
    }
    fn get_content_string(&self) -> String {
        let mut content = Some(ResponseContent::default());
        let mut response_container = self.response_container.lock().expect("POISONENENENE");
        std::mem::swap(&mut *response_container, &mut content);
        content
            .expect("This function should only be called after the promise is ready")
            .content
    }
}

#[derive(Default)]
struct RequestBuilder {
    url: String,
    verb: HttpVerb,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

#[extendr]
impl RequestBuilder {
    fn new(verb: &str, url: String) -> Self {
        let verb = match verb {
            "get" => HttpVerb::Get,
            "post" => HttpVerb::Post,
            "put" => HttpVerb::Put,
            "delete" => HttpVerb::Delete,
            _ => panic!("Http Verb '{}' is not supported", verb),
        };
        RequestBuilder {
            url,
            verb,
            headers: HashMap::new(),
            body: Vec::new(),
        }
    }
    fn set_header(&mut self, header_name: String, header_value: String) {
        self.headers.insert(header_name, header_value);
    }
    fn set_body_raw(&mut self, body: Raw) {
        self.body.clear();
        self.body.extend_from_slice(body.as_slice());
    }
    fn send_request(&mut self, http_client: &HttpClient) -> Response {
        // Why did I do this?
        //
        // I want to own the request builder
        // but extendr_api does not allow for
        // owned self so I have to create a new
        // instance and swap the pointers
        let mut request_builder = RequestBuilder::default();
        std::mem::swap(self, &mut request_builder);

        // We need a mutex that we can check on
        // at a specific rate and see if it's ready
        // with a response
        let response_container = Arc::new(Mutex::new(None));

        {
            let agent = http_client.agent.clone();
            let response_container = Arc::clone(&response_container);
            http_client.thread_pool.spawn(move || {
                let mut request = match request_builder.verb {
                    HttpVerb::Get => agent.get(&request_builder.url),
                    _ => todo!(),
                };

                // Set the headers
                for (header_name, header_value) in request_builder.headers {
                    request = request.set(&header_name, &header_value);
                }

                // Set the body
                match request.send_bytes(request_builder.body.as_slice()) {
                    Err(e) => eprintln!("Error send request: {e}"),
                    Ok(res) => {
                        let res_body = res.into_string().expect("Unable to get body");
                        let mut response_container = response_container.lock().expect("Posioned");
                        *response_container = Some(ResponseContent { content: res_body });
                    }
                };
            });
        }

        Response { response_container }
    }
}

// Macro to generate exports.
// This ensures exported functions are registered with R.
// See corresponding C code in `entrypoint.c`.
extendr_module! {
    mod asynchttp;
    impl RequestBuilder;
    impl HttpClient;
    impl Response;
}
