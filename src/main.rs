use rayon::iter::ParallelIterator;
use std::time::Instant;
mod bitcoin_parser;

fn main() {
    let blocks = bitcoin_parser::BlockCollection::new(std::path::PathBuf::from("/Users/christopher/Documents/bitcoin-core/blocks/"));

    // Sequential
    let start = Instant::now();
    let mut block_count = 0;
    for _ in blocks.iter() {
        block_count += 1;
        if block_count % 10000 == 0 {
            println!("Block: {:?}\tTime: {:?}", block_count, start.elapsed());
        }
    }
    println!("-- Sequential --\nBlock count: {:?}\nTime: {:?}", block_count, start.elapsed());

    // Parallel
    let start = Instant::now();
    let block_count = std::sync::atomic::AtomicUsize::from(0);
    blocks.par_iter().for_each(|_| {
        block_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let static_block_count = block_count.load(std::sync::atomic::Ordering::Relaxed);
        if static_block_count % 10000 == 0 {
            println!("Block: {:?}\tTime: {:?}", static_block_count, start.elapsed());
        }
    });
    println!("-- Parallel --\nBlock count: {:?}\nTime: {:?}", block_count, start.elapsed());
}
