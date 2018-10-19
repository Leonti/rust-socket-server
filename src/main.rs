
#![allow(unused)]

#[macro_use]
extern crate crossbeam_channel;
use crossbeam_channel as channel;
use std::io::Write;
use std::io;
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time;
use std::sync::{Arc,RwLock};

// https://github.com/Nervengift/chat-server-example/blob/master/src/main.rs

fn handle_client(mut stream: TcpStream, recv: channel::Receiver<String>) {
    println!("Client connected");

    loop {
        {
            let msg = recv.recv().unwrap();
            println!("DEBUG arc.read() => {:?}", msg);
            stream.write_fmt(format_args!("{}", msg)).unwrap();
        }
    }
}

struct Connection {
    sender: channel::Sender<String>,
    id: usize
}

fn main() -> io::Result<()> {

    let connections_arc: Arc<RwLock<Vec<Connection>>> = Arc::new(RwLock::new(Vec::new()));

    let connections = Arc::clone(&connections_arc);
    thread::spawn(move|| {
        loop {
            for i in 1..1000 {

                for connection in connections.read().unwrap().iter() {
                    println!("DEBUG messages in the channel {:?}", connection.sender.len());
                    connection.sender.send(format!("Test message {}", i));
                }

                thread::sleep(time::Duration::from_millis(500));
            }
        }
    });

    let listener = TcpListener::bind("0.0.0.0:9999")?;

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

        thread::spawn(|| {
            handle_client(stream.unwrap(), r);
        });
    }
    Ok(())
}
