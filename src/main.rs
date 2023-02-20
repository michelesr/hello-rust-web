use signal_hook::{
    consts::{SIGINT, SIGTERM},
    iterator::Signals,
};
use std::{
    error::Error,
    process::exit,
    sync::{Arc, Mutex},
};
use web::ThreadPool;

use std::{
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    thread,
    time::Duration,
};

fn main() -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();

    // wrap the thread pool in a mutex and reference counting so that it can be shared safely with
    // the signal handler running in a dedicated thread
    let pool = Arc::new(Mutex::new(ThreadPool::new(5)));
    let pool_clone = Arc::clone(&pool);

    // signals to handle
    let mut signals = Signals::new(&[SIGTERM, SIGINT])?;

    // use a dedicated thread: the iterator from listener.incoming() blocks when waiting for
    // connections, and could wait forever if no new connections are established: using a dedicated
    // thread the worker threads in the pool can be shut and the program can terminate even if the
    // main thread is blocked
    thread::spawn(move || {
        signals.forever().next();
        println!("caught signal");
        pool_clone.lock().unwrap().stop();
        exit(0);
    });

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        pool.lock().unwrap().execute(|| handle_stream(stream));
    }

    pool.lock().unwrap().stop();
    Ok(())
}

fn handle_stream(mut stream: TcpStream) {
    let buf_reader = BufReader::new(&mut stream);

    // first line of the request
    let request = buf_reader.lines().next().unwrap().unwrap();

    println!("{}", request);

    let (status, content) = match request.as_str() {
        "GET / HTTP/1.1" => ("HTTP/1.1 200 OK", "Hello\r\n"),
        "GET /sleep HTTP/1.1" => {
            thread::sleep(Duration::from_secs(5));
            ("HTTP/1.1 200 OK", "I'm awake!\r\n")
        }
        _ => ("HTTP/1.1 404 Not Found", "Not found!\r\n"),
    };

    let length = content.len();

    let response = format!("{status}\r\nContent-Length: {length}\r\n\r\n{content}");
    stream.write_all(response.as_bytes()).unwrap();
}
