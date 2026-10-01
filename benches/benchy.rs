
use criterion::{criterion_group, criterion_main, Criterion};

use warren::Warren;

fn benchmark_insertion(c: &mut Criterion) {

    c.bench_function("Insert 1 million u64 values.", 
        |b| b.iter(|| { 
            let mut warren1: Warren<u64> = Warren::with_region_count(1);
            (0..1000000).for_each(|i| {
                warren1.insert(i);
            });
        })
    );

    c.bench_function("Insert 1 million [u64; 8] arrays.", 
        |b| b.iter(|| { 
            let mut warren2: Warren<[u64; 8]> = Warren::with_region_count(1);
            (0..1000000).for_each(|i| {
                warren2.insert([i; 8]);
            });
        })
    );

    c.bench_function("Insert 1 million u64 values with capacity.", 
        |b| b.iter(|| { 
            let mut warren1: Warren<u64> = Warren::with_capacity(1000000);
            (0..1000000).for_each(|i| {
                warren1.insert(i);
            });
        })
    );

    c.bench_function("Insert 1 million [u64; 8] arrays with capacity.", 
        |b| b.iter(|| { 
            let mut warren2: Warren<[u64; 8]> = Warren::with_capacity(1000000);
            (0..1000000).for_each(|i| {
                warren2.insert([i; 8]);
            });
        })
    );

    c.bench_function("Insert and remove 1 million u64 values with capacity.", 
        |b| b.iter(|| { 
            let mut warren1: Warren<u64> = Warren::with_capacity(1000000);
            (0..1000000).for_each(|i| {
                warren1.insert(i);
            });
            (0..1000000).for_each(|i| {
                warren1.remove(i);
            });
        })
    );

    c.bench_function("Insert and remove 1 million [u64; 8] arrays with capacity.", 
        |b| b.iter(|| { 
            let mut warren2: Warren<[u64; 8]> = Warren::with_capacity(1000000);
            (0..1000000).for_each(|i| {
                warren2.insert([i; 8]);
            });
            (0..1000000).for_each(|i| {
                warren2.remove(i);
            });
        })
    );

    let mut warren1: Warren<u64> = Warren::with_region_count(1);
    (0..1000000).for_each(|i| {
        warren1.insert(i);
    });

    let mut warren2: Warren<[u64; 8]> = Warren::with_region_count(1);
     (0..1000000).for_each(|i| {
        warren2.insert([i; 8]);
    });    

    c.bench_function("Iterate 1 million u64 elements.", 
        |b| b.iter(|| { 
            warren1.iter().for_each(|_x| { std::hint::black_box(0); });
        })
    );

    c.bench_function("Iterate 1 million [u64; 8] arrays.", 
        |b| b.iter(|| { 
            warren2.iter().for_each(|_x| { std::hint::black_box(0); });
        })
    );

    c.bench_function("Iterate with mutability 1 million u64 elements.", 
        |b| b.iter(|| { 
            warren1.iter_mut().for_each(|_x| { std::hint::black_box(0); });
        })
    );

    c.bench_function("Iterate with mutability 1 million [u64; 8] arrays.", 
        |b| b.iter(|| { 
            warren2.iter_mut().for_each(|_x| { std::hint::black_box(0); });
        })
    );
}


criterion_group!(benches, benchmark_insertion);
criterion_main!(benches);