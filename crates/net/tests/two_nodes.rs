//! Two local nodes: A publishes a listing, B receives, verifies and stores it.
use lc_core::*;
use lc_net::{run, Command, Event};
use std::time::Duration;
use tokio::sync::mpsc;

#[tokio::test(flavor = "multi_thread")]
async fn listing_propagates_between_two_nodes() {
    let (ca, ra) = mpsc::channel(16); let (ea, mut rxa) = mpsc::channel(64);
    let (cb, rb) = mpsc::channel(16); let (eb, mut rxb) = mpsc::channel(64);
    tokio::spawn(run("/ip4/127.0.0.1/tcp/0".parse().unwrap(), ra, ea));
    tokio::spawn(run("/ip4/127.0.0.1/tcp/0".parse().unwrap(), rb, eb));
    let addr_a = loop { if let Some(Event::Listening(a)) = rxa.recv().await { break a } };
    cb.send(Command::Dial(addr_a)).await.unwrap();
    loop { if let Some(Event::Connected(_)) = rxb.recv().await { break } }
    tokio::time::sleep(Duration::from_secs(2)).await; // gossipsub mesh/subscription exchange

    let seller = Identity::generate();
    let env = seller.seal(&Payload::Listing(Listing { kind: ListingKind::Offer, title: "Used guitar".into(), desc: "".into(),
        price_hint: "trade".into(), category: "music".into(), expiry: 0, image_hashes: vec![], created: 0 }), 8);
    let ledger_b = lc_ledger::Ledger::memory(8).unwrap();
    let got = tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            ca.send(Command::Publish(env.clone())).await.unwrap();
            if let Ok(Some(Event::Received(e))) = tokio::time::timeout(Duration::from_millis(700), rxb.recv()).await { break e; }
        }
    }).await.expect("B did not receive listing");
    assert!(ledger_b.append(&got).unwrap());
    assert_eq!(ledger_b.get(&env.id()).unwrap().unwrap(), env);
}
