use leveldb::iterator::Iterable;
use leveldb::kv::KV;
use rayon::iter::ParallelIterator;
use serde_json;
use std::collections::HashMap;
use std::time::Instant;

use std::sync::Mutex;

use leveldb;

mod bitcoin_parser;
mod exchange_rates;

#[derive(Debug)]
struct VecKey {
    key: Vec<u8>,
}

impl db_key::Key for VecKey {
    fn from_u8(key: &[u8]) -> Self {
        VecKey{key: Vec::from(key)}
    }

    fn as_slice<T, F: Fn(&[u8]) -> T>(&self, f: F) -> T {
        f(&self.key)
    }
}

fn main_old() {
    // let index_db = leveldb::database::Database::<VecKey>::open(std::path::Path::new("/Users/christopher/Documents/bitcoin-core/blocks/index/"), leveldb::database::options::Options::new()).expect("Could not open leveldb.");
    let index_db = leveldb::database::Database::<VecKey>::open(std::path::Path::new("/Users/christopher/Documents/bitcoin-core/indexes/txindex/"), leveldb::database::options::Options::new()).expect("Could not open leveldb.");

    let read_options = leveldb::database::options::ReadOptions::new();
    // let res = index_db.get(read_options, VecKey{key: b"test".to_vec()});
    // println!("Res: {:?}", res);
    /* for (key, val) in index_db.iter(read_options).filter(|(k, _)| k.key.first().map(|v| v == &0x74).unwrap_or(false)).take(100) {
        println!("Key: {:x?}\nVal: {:x?}\n", key.key, val);
    }*/
    let mut out = std::collections::HashSet::new();
    let mut counter = 0;
    for (key, val) in index_db.iter(read_options).take(1000) {
        out.insert(key.key[0]);
        counter += 1;
    }
    println!("Counter: {:?}", counter);
    println!("Set: {:?}, {:x?}", out, out);
}

fn main() {
    let blocks = bitcoin_parser::BlockCollection::new(std::path::PathBuf::from(
        "/Users/christopher/Documents/bitcoin-core/blocks/",
    ));

    for t in blocks.transaction_iter().take(10) {
        println!("Hash: {:x?}\t Time: {:?}\t Coinbase: {:?}\t Input count: {:?}\t Output count: {:?}", t.hash, t.timestamp, t.is_coinbase, t.inputs.len(), t.outputs.len());
        // for o in t.outputs {
        //     println!("Script: {:?}", o.script.expect("Failed to parse script.").opcodes);
        // }
    }

    let start = Instant::now();

    let rates = exchange_rates::ExchangeRates::new();

    // let values = blocks
    //     .par_iter()
    //     .map(|b| {
    //         b.transactions
    //             .iter()
    //             .map(|t| t.outputs.iter().map(|o| o.value).collect::<Vec<_>>())
    //             .flatten()
    //             .collect::<Vec<_>>()
    //     })
    //     .flatten()
    //     .collect::<Vec<_>>();

    let value_histogram: Mutex<HashMap<u32, u32>> = Mutex::new(HashMap::new());
    /*blocks
        .transaction_par_iter()
        .filter(|t| !t.is_coinbase)
        .flat_map(|t| t.value_usd(&rates))
        .for_each(|v| {
            let order;
            if v == 0f64 {
                order = 0;
            } else {
                order = (v.log10() * 100f64).floor() as u32;
            }
            *value_histogram
                .lock()
                .expect("Poisoned mutex.")
                .entry(order)
                .or_insert(0) += 1;
            // *value_histogram.entry(order).or_insert(0) += 1;
        });*/

    println!(
        "{:?}\tFinished getting transaction values.",
        start.elapsed()
    );

    // for v in values {
    //     let order;
    //     if v <= 0f32 {
    //         order = 0;
    //     } else {
    //         order = ((v as f64).log10() * 100f64).floor() as u32;
    //     }
    //     *value_histogram.entry(order).or_insert(0) += 1;
    // }

    println!("{:?}\tFinished creating histogram.", start.elapsed());

    let output_file = std::fs::File::create("/Users/christopher/Downloads/test4.json")
        .expect("Could not create output file.");
    // serde_json::to_writer(output_file, &value_histogram).expect("Failed to serialize to JSON.");

    // // Sequential
    // let start = Instant::now();
    // let mut block_count = 0;
    // for _ in blocks.iter() {
    //     block_count += 1;
    //     if block_count % 10000 == 0 {
    //         println!("Block: {:?}\tTime: {:?}", block_count, start.elapsed());
    //     }
    // }
    // println!(
    //     "-- Sequential --\nBlock count: {:?}\nTime: {:?}",
    //     block_count,
    //     start.elapsed()
    // );

    // // Parallel
    // let start = Instant::now();
    // let block_count = std::sync::atomic::AtomicUsize::from(0);
    // blocks.par_iter().for_each(|_| {
    //     block_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    //     let static_block_count = block_count.load(std::sync::atomic::Ordering::Relaxed);
    //     if static_block_count % 10000 == 0 {
    //         println!(
    //             "Block: {:?}\tTime: {:?}",
    //             static_block_count,
    //             start.elapsed()
    //         );
    //     }
    // });
    // println!(
    //     "-- Parallel --\nBlock count: {:?}\nTime: {:?}",
    //     block_count,
    //     start.elapsed()
    // );
}
