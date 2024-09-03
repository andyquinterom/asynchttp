use std::{
    collections::HashMap, rc::Rc, sync::{atomic::{AtomicBool, Ordering}, Arc, Mutex},
    io::Read
};

use extendr_api::prelude::*;

struct HttpClient {
    thread_pool: Rc<rayon::ThreadPool>,
    agent: ureq::Agent,
}

impl Clone for HttpClient {
    fn clone(&self) -> Self {
        HttpClient {
            thread_pool: Rc::clone(&self.thread_pool),
            agent: self.agent.clone(),
        }
    }
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
            .expect("Unable to build the thread pool")
            .into();
        let agent = ureq::agent();
        HttpClient { agent, thread_pool }
    }
}

struct BodyStream {
    is_done: Arc<AtomicBool>,
    buffer: Arc<Mutex<Vec<u8>>>
}

impl BodyStream {
    fn new(pool: Rc<rayon::ThreadPool>, res: ureq::Response) -> Self {

        let is_done = Arc::new(AtomicBool::new(false));

        // Add a reserve capacity for the content length if possible
        let buffer = Arc::new(Mutex::new(Vec::<u8>::new()));
        {
            let buffer = Arc::clone(&buffer);
            let is_done = Arc::clone(&is_done);
            pool.spawn(move || {
                let mut res = res.into_reader();
                let mut temp_buffer = [0; 1024];
                loop {
                    let read_bytes = res.read(&mut temp_buffer);
                    match read_bytes {
                        Err(e) => {
                            eprintln!("{e}");
                            break;
                        },
                        Ok(0) => break,
                        Ok(n) => {
                            let mut buffer = buffer.lock().expect("Poisoned");
                            buffer.extend_from_slice(&temp_buffer[..n]);
                        }
                    } 
                }

                is_done.store(true, Ordering::SeqCst);
            });
        }
        BodyStream {
            is_done,
            buffer,
        }
    }
}

#[extendr]
impl BodyStream {
    fn is_done(&self) -> bool {
        self.is_done.load(Ordering::SeqCst)
    }
    fn poll(&self) -> Raw {
        let mut buffer = self.buffer.lock().expect("Buffer posioned");
        // Is there a better way to do this? We will always need memory
        // allocations?
        let raw = Raw::from_bytes(&buffer);
        buffer.clear();
        raw
    }
}


struct Response {
    thread_pool: Rc<rayon::ThreadPool>,
    response_container: Arc<Mutex<Option<ureq::Response>>>,
}

#[extendr]
impl Response {
    fn poll(&self) -> bool {
        let response_container = self.response_container.lock().expect("POISONENENENE");
        response_container.is_some()
    }
    fn get_content_string(&self) -> Result<String> {
        let mut response_container = self.response_container.lock().expect("POISONENENENE");
        let content = response_container.take();
        content
            .expect("This function should only be called after the promise is ready")
            .into_string()
            .map_err(|err| err.to_string().into())
    }
    fn get_content_stream(&self) -> Result<BodyStream> {
        let mut response_container = self.response_container.lock().expect("POISONENENENE");
        let content = response_container.take()
            .expect("This function should only be called after the promise is ready");
        Ok(BodyStream::new(self.thread_pool.clone(), content))
    }
}

struct RequestBuilder {
    client: Option<HttpClient>,
    url: String,
    verb: HttpVerb,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

impl Default for RequestBuilder {
    fn default() -> Self {
        RequestBuilder {
            client: None,
            url: String::new(),
            verb: HttpVerb::Get,
            headers: HashMap::new(),
            body: Vec::new()
        }
    }
}

#[extendr]
impl RequestBuilder {
    fn from_client(client: &HttpClient, url: String) -> Self {
        RequestBuilder {
            client: Some(client.clone()),
            url,
            verb: HttpVerb::Get,
            headers: HashMap::new(),
            body: Vec::new(),
        }
    }
    fn set_method(&mut self, verb: &str) {
        let verb = match verb.to_lowercase().as_str() {
            "get" => HttpVerb::Get,
            "post" => HttpVerb::Post,
            "put" => HttpVerb::Put,
            "delete" => HttpVerb::Delete,
            _ => panic!("Http Verb '{}' is not supported", verb),
        };
        self.verb = verb;
    }
    fn set_header(&mut self, header_name: String, header_value: String) {
        self.headers.insert(header_name, header_value);
    }
    fn set_body_raw(&mut self, body: Raw) {
        self.body.clear();
        self.body.extend_from_slice(body.as_slice());
    }
    fn send_request(&mut self) -> Result<Response> {
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
        let http_client = request_builder.client.ok_or("This request has already been executed")?;
        {
            let agent = http_client.agent;
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
                        let mut response_container = response_container.lock().expect("Posioned");
                        response_container.replace(res);
                    }
                };
            });
        }

        Ok(Response { response_container, thread_pool: http_client.thread_pool.clone() })
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
    impl BodyStream;
}
