use std::{io::{self, Write}, process::Command, thread, time::Duration};

fn main() {
    let args: Vec<_> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("echo") => println!("{}", args[2]),
        Some("success") => {
            println!("standard output");
            eprintln!("standard error");
        }
        Some("failure") => {
            println!("partial result");
            eprintln!("real failure diagnostic");
            std::process::exit(7);
        }
        Some("sleep") => {
            println!("started");
            io::stdout().flush().unwrap();
            thread::sleep(Duration::from_secs(10));
        }
        Some("flood") => {
            io::stdout().write_all(&vec![b'a'; 256 * 1024]).unwrap();
            io::stderr().write_all(&vec![b'b'; 256 * 1024]).unwrap();
        }
        Some("bytes") => {
            io::stdout().write_all(&[0xff, 0x00, b'x']).unwrap();
            io::stderr().write_all(&[0xfe, b'y']).unwrap();
        }
        Some("descendant") => {
            let child = Command::new(std::env::current_exe().unwrap())
                .arg("sleep").spawn().unwrap();
            println!("descendant {}", child.id());
            // The parent exits, leaving the descendant holding both pipes.
        }
        _ => std::process::exit(9),
    }
}
