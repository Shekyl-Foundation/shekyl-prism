// Copyright (c) 2026, The Shekyl Foundation
// All rights reserved.
// BSD-3-Clause

//! Private Chutney experiment. The in-memory relay carries public fixtures only.
use quorum_approval_experiment::{Collector, Payment, Policy};
use shekyl_prism::{SetId, P500};
use socks::Socks5Stream;
use std::{
    env, fs,
    io::{self, Read, Write},
    net::{TcpListener, TcpStream},
    path::Path,
    time::{Duration, Instant},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const STORED: u8 = 1;
const EMPTY: u8 = 0;

fn approval_len() -> usize {
    let set = SetId::of::<P500>();
    1 + set.tagged_public_key_len() + set.tagged_signature_len()
}

fn exchange(conn: &mut TcpStream, slots: &mut [Option<Vec<u8>>; 3]) -> Result<()> {
    conn.set_read_timeout(Some(Duration::from_secs(10)))?;
    conn.set_write_timeout(Some(Duration::from_secs(10)))?;
    let mut command = [0; 2];
    conn.read_exact(&mut command)?;
    let slot = slots
        .get_mut(usize::from(command[1]))
        .ok_or("unknown slot")?;
    match command[0] {
        b'P' => {
            let mut data = vec![0; approval_len()];
            conn.read_exact(&mut data)?;
            if let Some(old) = slot {
                if *old != data {
                    return Err("slot already occupied".into());
                }
            } else {
                *slot = Some(data);
            }
            conn.write_all(&[STORED])?;
        }
        b'G' => match slot {
            Some(data) => {
                conn.write_all(&[STORED])?;
                conn.write_all(data)?;
            }
            None => conn.write_all(&[EMPTY])?,
        },
        _ => return Err("unknown operation".into()),
    }
    Ok(())
}

fn serve() -> Result<()> {
    let mut slots = [None, None, None];
    let listener = TcpListener::bind(("127.0.0.1", 4747))?;
    for stream in listener.incoming() {
        if let Err(err) = exchange(&mut stream?, &mut slots) {
            eprintln!("incomplete or refused request: {err}");
        }
    }
    Ok(())
}

struct Route {
    onion: String,
    socks_port: u16,
}

impl Route {
    fn connect(&self) -> Result<Socks5Stream> {
        let stream =
            Socks5Stream::connect(("127.0.0.1", self.socks_port), (self.onion.as_str(), 5858))?;
        stream
            .get_ref()
            .set_read_timeout(Some(Duration::from_secs(20)))?;
        stream
            .get_ref()
            .set_write_timeout(Some(Duration::from_secs(20)))?;
        Ok(stream)
    }

    fn put(&self, slot: u8, bytes: &[u8], interrupt: bool) -> Result<()> {
        let mut stream = self.connect()?;
        stream.write_all(&[b'P', slot])?;
        if interrupt {
            stream.write_all(&bytes[..bytes.len() / 2])?;
            // EOF after half the frame; wait for the relay to discard it.
            stream.get_ref().shutdown(std::net::Shutdown::Write)?;
            let mut reply = [0];
            if stream.read(&mut reply)? != 0 {
                return Err("short frame was acknowledged".into());
            }
        } else {
            stream.write_all(bytes)?;
            let mut reply = [0];
            stream.read_exact(&mut reply)?;
            if reply != [STORED] {
                return Err("missing storage acknowledgement".into());
            }
        }
        Ok(())
    }

    fn get(&self, slot: u8) -> Result<Option<Vec<u8>>> {
        let mut stream = self.connect()?;
        stream.write_all(&[b'G', slot])?;
        let mut reply = [0];
        stream.read_exact(&mut reply)?;
        match reply[0] {
            EMPTY => Ok(None),
            STORED => {
                let mut data = vec![0; approval_len()];
                stream.read_exact(&mut data)?;
                Ok(Some(data))
            }
            _ => Err("unknown mailbox reply".into()),
        }
    }
}

fn run(route: &Route, inbox: &Path) -> Result<()> {
    let start = Instant::now();
    let root = Path::new("fixtures/p500");
    let set = SetId::of::<P500>();
    let roster = fs::read(root.join("enrollment.bin"))?;
    let keys: [&[u8]; 3] = roster
        .chunks(set.tagged_public_key_len())
        .collect::<Vec<_>>()
        .try_into()
        .map_err(|_| "invalid enrollment fixture")?;
    let policy = Policy::enroll(set, keys)?;
    let pay = Payment::fixture();
    let mut collector = Collector::new(&policy, &pay);
    let alice = fs::read(root.join("0.bin"))?;
    let bob = fs::read(root.join("1.bin"))?;
    let carol = fs::read(root.join("2.bin"))?;
    route.put(0, &alice, false)?;
    // Subsequent reads use the mailbox. Alice's signer is never called again.
    route.put(1, &bob, true)?;
    if route.get(1)?.is_some() {
        return Err("relay retained interrupted upload".into());
    }
    let retrieved = route.get(0)?.ok_or("Alice missing")?;
    fs::write(inbox.join("alice.bin"), &retrieved)?;
    assert!(collector.admit(&retrieved)?);
    assert!(!collector.admit(&fs::read(inbox.join("alice.bin"))?)?);
    assert!(collector.witness().is_none());

    // A restarted assembler can rebuild from delivered public approvals.
    let mut resumed = Collector::new(&policy, &pay);
    assert!(resumed.admit(&fs::read(inbox.join("alice.bin"))?)?);
    assert!(resumed.admit(&carol)?);
    let by_file = resumed.witness().ok_or("file carrier did not finish")?;
    policy.verify_witness(&pay, &by_file)?;
    route.put(2, &carol, false)?;
    route.put(2, &carol, false)?;
    let delivered = route.get(2)?.ok_or("Carol missing")?;
    assert!(collector.admit(&delivered)?);
    assert!(!collector.admit(&delivered)?);
    let by_onion = collector.witness().ok_or("online carrier did not finish")?;
    policy.verify_witness(&pay, &by_onion)?;
    assert_eq!(by_file, by_onion);
    println!("P500: interrupted Bob discarded; Alice reused across restart; Carol by file and onion: identical {}-byte witnesses, elapsed {:.2}s", by_onion.len(), start.elapsed().as_secs_f64());
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<_> = env::args().skip(1).collect();
    match args.as_slice() {
        [cmd] if cmd == "serve" => serve(),
        [cmd, hostname, port, inbox] if cmd == "run" => {
            let onion = fs::read_to_string(hostname)?.trim().to_owned();
            if !onion.ends_with(".onion") {
                return Err("expected a private test onion".into());
            }
            run(
                &Route {
                    onion,
                    socks_port: port.parse()?,
                },
                Path::new(inbox),
            )
        }
        _ => Err(
            io::Error::other("use serve | run CHUTNEY_HOSTNAME SOCKS_PORT EMPTY_DIRECTORY").into(),
        ),
    }
}
