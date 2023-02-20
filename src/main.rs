use signal_hook::{
    consts::{SIGINT, SIGTERM},
    iterator::Signals,
};
use std::{
    error::Error,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use web::ThreadPool;

use std::{
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    thread,
    time::Duration,
};

fn main() -> Result<(), Box<dyn Error>> {
    const ENDPOINT: &str = "127.0.0.1:8080";
    let listener = TcpListener::bind(ENDPOINT).unwrap();

    let pool = ThreadPool::new(5);

    // atomic boolean used to stop the main loop
    let stop = Arc::new(AtomicBool::new(false));
    let stop_me = Arc::clone(&stop);

    // signals to handle
    let mut signals = Signals::new(&[SIGTERM, SIGINT])?;

    // listener.incoming() blocks when waiting for connections: signals are handled in a dedicated
    // thread that sets the stop flag to true to break the loop that handles incoming requests
    //
    // https://rust-cli.github.io/book/in-depth/signals.html
    thread::spawn(move || {
        // only the first signal is needed from the iterator
        signals.forever().next();
        // set flag to break the main loop
        stop.store(true, Ordering::Relaxed);
        // signal the listener by opening a new connection
        TcpStream::connect(ENDPOINT).unwrap();
    });

    for stream in listener.incoming() {
        if stop_me.load(Ordering::Relaxed) {
            break;
        }
        let stream = stream.unwrap();
        pool.execute(|| handle_stream(stream));
    }

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
