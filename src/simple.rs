
#![allow(unused)]

#[macro_use]
extern crate crossbeam_channel;
extern crate bufstream;
extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate serde_derive;

use std::io::BufRead;
use crossbeam_channel as channel;
use std::io::Write;
use std::io;
use std::net::{TcpListener, TcpStream};
use std::thread;
use bufstream::BufStream;
use std::time;
use std::sync::{Arc,RwLock};

// https://github.com/Nervengift/chat-server-example/blob/master/src/main.rs


fn handle_client(stream: &mut BufStream<TcpStream>, recv: channel::Receiver<String>, sender: channel::Sender<String>) {
    println!("Client connected");

    loop {
        {
            let msg = recv.recv().unwrap();
            println!("DEBUG arc.read() => {:?}", msg);
            stream.write_fmt(format_args!("{}", msg)).unwrap();
        }
        stream.flush().unwrap();

        let mut reads = String::new();
        stream.read_line(&mut reads).unwrap(); //TODO: non-blocking read
        if reads.trim().len() != 0 {
            println!("DEBUG: reads len =>>>>> {}", reads.len());
            sender.send(format!("received: {}", reads));
            //println!("DEBUG: got '{}' from {}", reads.trim(), name);
        }
    }
}

fn handle_reads(stream: &mut BufStream<TcpStream>, ) {

}

struct Connection {
    sender: channel::Sender<String>,
    id: usize
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
enum Wheel {
    Left,
    Right,
}

#[derive(Serialize)]
enum Event {
    #[serde(rename = "encoder")]
    Encoder {
        wheel: Wheel
    }
}

#[derive(Serialize)]
struct Message {
    time: u32,
    event: Event,
}

fn main() -> io::Result<()> {

    let message = Message {
        time: 0,
        event: Event::Encoder { wheel: Wheel::Left }
    };

    let j = serde_json::to_string(&message)?;
    println!("{}", j);

    let connections_arc: Arc<RwLock<Vec<Connection>>> = Arc::new(RwLock::new(Vec::new()));

    let connections = Arc::clone(&connections_arc);
    let (send, recv) = channel::unbounded();

    thread::spawn(move|| {
        loop {
            for i in 1..1000 {

                for connection in connections.read().unwrap().iter() {
                    println!("DEBUG messages in the channel {:?}", connection.sender.len());

                    // TODO better way to detect closed connections and remove `Connection` from the list
                    if (connection.sender.len() < 100) {
                        connection.sender.send(format!("{}\n", j));
                    }
                }

                thread::sleep(time::Duration::from_millis(500));
            }
        }
    });

    thread::spawn(move|| {
        loop {
            let msg = recv.recv().unwrap();
            println!("Received from socket => {:?}", msg);
        }
    });

    let listener = TcpListener::bind("0.0.0.0:5000")?;

    let mut client_count = 0;
    let arc_w = connections_arc.clone();

    for stream in listener.incoming() {
        client_count += 0;

        let (s, r) = channel::unbounded();
        let connection = Connection {
            sender: s,
            id: client_count
        };

        {
            let mut arc_w = arc_w.write().unwrap();
            arc_w.push(connection);
        } // write lock is released at the end of this scope
        let client_send = send.clone();

        thread::spawn(|| {
            let mut st = BufStream::new(stream.unwrap());
            handle_client(&mut st, r, client_send);
        });
    }
    Ok(())
}
