
use gossip::*;
use serde::{Serialize, Deserialize};
use std::error::Error;
use serde_cbor;
use sha2::{self, Sha256, Digest};

use std::convert::TryInto;
use lazy_static::lazy_static;

use dashmap::DashMap;

type Name = String;
type Addr = String;
type Event = usize;
type EventHash = [u8; 32];

lazy_static! {
    
    // Databases
    static ref PEER_DATABASE: DashMap<Name, Addr> = DashMap::new();
    static ref TX_DATABASE: DashMap<usize, Transaction> = DashMap::new();
    static ref HASH_DATABASE : DashMap<Event, (Vec<u8>, EventHash)> = DashMap::new();
            /* ^^^^^ kinda like bigtable */

    // Input for genesis hash (not genesis hash itself)
    static ref HASH: [u8; 32] = [0; 32]; 
}

pub struct MyUpdateHandler;

impl UpdateHandler for MyUpdateHandler {
    fn on_update(&self, update: Update) {

        match serde_cbor::from_slice(&update.content()) {

            // Deal with addition of new peers
            Ok(OperatorMessage::NewPeer((name, addr))) => {

                println!("mock handling new peer: {name} @ {addr}");

                // Enter into database
                let current_entry = (*PEER_DATABASE).insert(name.to_owned(), addr.to_owned());

                // Check if entry already existed
                if let Some(entry) = current_entry {
                    println!("ayo this peer {name}/{entry} exists already");
                }
            },

            // Deal with new tx
            Ok(OperatorMessage::Transaction(tx)) => {

                println!("mock handling tx: {tx:?}");

                // Enter into database
                let current_entry = (*TX_DATABASE).insert((*TX_DATABASE).len(), tx.clone());

                // Check if entry already existed
                if let Some(entry) = current_entry {
                    println!("ayo this tx {tx:?} exists already");
                }
            },

            // Deal with text message
            Ok(OperatorMessage::Message(msg)) => println!("mock handling msg: {msg}"),

            // Deal with err
            Err(err) => println!("error handling message: {err:?}"),
        }
        
    }
}

fn main() {

    // Configure logging
    configure_logging(log::LevelFilter::Info).unwrap();

    // Initialize hasher
    let mut hasher = sha2::Sha256::new_with_prefix(*HASH);
    println!("Initialize hash is {:?}", hasher.clone().finalize());

    // local machine IP and port for listening
    let my_address = "127.0.0.1:9000";
    let my_address2 = "127.0.0.1:9001";
    println!("using {my_address}");
    
    // existing peer(s) in the network
    let existing_peers = || Some(vec![ Peer::new("127.0.0.1:9001".to_owned()) ]);
    let existing_peers2 = || Some(vec![ Peer::new("127.0.0.1:9000".to_owned()) ]);

    // create and start the service
    let mut gossip_service = GossipService::new_with_defaults(my_address.parse().unwrap());
    gossip_service.start(Box::new(existing_peers), Box::new(MyUpdateHandler)).unwrap();

    let mut gossip_service2 = GossipService::new_with_defaults(my_address2.parse().unwrap());
    gossip_service2.start(Box::new(existing_peers2), Box::new(MyUpdateHandler)).unwrap();
    
    // submit new peer message
    let msg = serde_cbor::to_vec(
        &OperatorMessage::NewPeer(("Bob", "420 Degen Blvd"))
    ).unwrap();
    hasher.update(&msg);
    let event_hash = hasher.clone().finalize();
    (*HASH_DATABASE).insert(0, (msg.clone(), event_hash.try_into().unwrap()),);
    gossip_service.submit(msg).unwrap();


    // submit new tx message
    let msg = serde_cbor::to_vec(
        &OperatorMessage::Transaction(
            Transaction {
                shades: 10,
                from: "Me".to_owned(),
                to: "You".to_owned(),
            }
        )
    ).unwrap();
    hasher.update(&msg);
    let event_hash = hasher.clone().finalize();
    (*HASH_DATABASE).insert(1, (msg.clone(), event_hash.try_into().unwrap()));
    gossip_service.submit(msg).unwrap();

    // submit another new tx message
    let msg = serde_cbor::to_vec(
        &OperatorMessage::Transaction(
            Transaction {
                shades: 5,
                from: "You".to_owned(),
                to: "Me".to_owned(),
            }
        )
    ).unwrap();
    hasher.update(&msg);
    let event_hash = hasher.clone().finalize();
    (*HASH_DATABASE).insert(2, (msg.clone(), event_hash.try_into().unwrap()));
    gossip_service2.submit(msg).unwrap();

    // submit new string message
    let msg = serde_cbor::to_vec(
        &OperatorMessage::Message("hello from solana")
    ).unwrap();
    hasher.update(&msg);
    let event_hash = hasher.clone().finalize();
    (*HASH_DATABASE).insert(2, (msg.clone(), event_hash.try_into().unwrap()));
    gossip_service.submit(msg).unwrap();

    // submit new manually created text message (fails)
    let mut msg = 2_u32.to_le_bytes().to_vec();
    msg.append(&mut "joe".as_bytes().to_vec());
    gossip_service.submit(
        serde_cbor::to_vec(
            &msg
        ).unwrap()
    ).unwrap();

    std::thread::sleep(std::time::Duration::from_millis(2000));

    // shutdown the gossip protocol on exit
    // gossip_service.shutdown().unwrap()

    println!("\n----- CHECKING PEER_DATABASE -----");
    for peer_pair in (*PEER_DATABASE).iter() {
        println!("PEER_DATABASE contains: {:?}", peer_pair.pair());
    }

    println!("\n----- CHECKING TX_DATABASE -----");
    for tx_pair in (*TX_DATABASE).iter() {
        println!("TX_DATABASE contains: {:?}", tx_pair.pair());
    }

    println!("\n----- CHECKING HASH_DATABASE -----");
    for event_idx in 0..(*HASH_DATABASE).len() {
        let hash_pair = (*HASH_DATABASE).get(&event_idx).unwrap();
        println!("HASH_DATABASE contains: {:?}", hash_pair.pair());
    }
}



#[derive(Debug, Serialize, Deserialize)]
pub enum OperatorMessage<'a> {
    NewPeer((&'a str, &'a str)),
    Transaction(Transaction),
    Message(&'a str)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Transaction {
    shades: u64,
    from: String,
    to: String,
}

#[allow(dead_code)]
pub fn configure_logging(level: log::LevelFilter) -> Result<(), Box<dyn Error>> {

    use log4rs::encode::pattern::PatternEncoder;
    use log4rs::config::{Appender, Config, Root};
    use log4rs::append::console::ConsoleAppender;

    let console = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{h({l})} {T} - {m}{n}")))
        .build();

    let config = Config::builder()
        .appender(Appender::builder().build("console", Box::new(console)))
        .build(Root::builder()
            .appender("console")
            .build(level))?;

    log4rs::init_config(config)?;

    Ok(())
}
