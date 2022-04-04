use rayon::iter::ParallelIterator;
use serde_json;
use std::collections::HashMap;
use std::time::Instant;

use std::sync::Mutex;

mod bitcoin_parser;
mod exchange_rates;

fn main() {
    let blocks = bitcoin_parser::BlockCollection::new(std::path::PathBuf::from(
        "/Users/christopher/Documents/bitcoin-core/blocks/",
    ));

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
    blocks.transaction_par_iter().flat_map(|t| t.value_usd(&rates)).for_each(|v| {
        let order;
        if v == 0f64 {
            order = 0;
        } else {
            order = (v.log10() * 100f64).floor() as u32;
        }
        *value_histogram.lock().expect("Poisoned mutex.").entry(order).or_insert(0) += 1;
        // *value_histogram.entry(order).or_insert(0) += 1;
    });

    println!("{:?}\tFinished getting transaction values.", start.elapsed());

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

    let output_file = std::fs::File::create("/Users/christopher/Downloads/test3.json")
        .expect("Could not create output file.");
    serde_json::to_writer(output_file, &value_histogram).expect("Failed to serialize to JSON.");

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
