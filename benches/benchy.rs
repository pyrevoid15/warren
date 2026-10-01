
use criterion::{criterion_group, criterion_main, Criterion};

use warren::{Warren, warren::GenerationGuard};

fn timing_warren(c: &mut Criterion) {

    c.bench_function("Warren -- Insert 1 million u64 values.", 
        |b| b.iter(|| { 
            let mut warren1: Warren<u64> = Warren::with_region_count(1);
            (0..1000000).for_each(|i| {
                warren1.insert(i);
            });
        })
    );

    c.bench_function("Warren -- Insert 1 million [u64; 8] arrays.", 
        |b| b.iter(|| { 
            let mut warren2: Warren<[u64; 8]> = Warren::with_region_count(1);
            (0..1000000).for_each(|i| {
                warren2.insert([i; 8]);
            });
        })
    );

    c.bench_function("Warren -- Insert 1 million u64 values with capacity.", 
        |b| b.iter(|| { 
            let mut warren1: Warren<u64> = Warren::with_capacity(1000000);
            (0..1000000).for_each(|i| {
                warren1.insert(i);
            });
        })
    );

    c.bench_function("Warren -- Insert 1 million [u64; 8] arrays with capacity.", 
        |b| b.iter(|| { 
            let mut warren2: Warren<[u64; 8]> = Warren::with_capacity(1000000);
            (0..1000000).for_each(|i| {
                warren2.insert([i; 8]);
            });
        })
    );

    c.bench_function("Warren -- Insert and remove 1 million u64 values with capacity.", 
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

    c.bench_function("Warren -- Insert and remove 1 million [u64; 8] arrays with capacity.", 
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

    c.bench_function("Warren -- Iterate 1 million u64 elements.", 
        |b| b.iter(|| { 
            warren1.iter().for_each(|_x| { std::hint::black_box(0); });
        })
    );

    c.bench_function("Warren -- Iterate 1 million [u64; 8] arrays.", 
        |b| b.iter(|| { 
            warren2.iter().for_each(|_x| { std::hint::black_box(0); });
        })
    );

    c.bench_function("Warren -- Iterate with mutability 1 million u64 elements.", 
        |b| b.iter(|| { 
            warren1.iter_mut().for_each(|_x| { std::hint::black_box(0); });
        })
    );

    c.bench_function("Warren -- Iterate with mutability 1 million [u64; 8] arrays.", 
        |b| b.iter(|| { 
            warren2.iter_mut().for_each(|_x| { std::hint::black_box(0); });
        })
    );
}

fn timing_warren_with_generations(c: &mut Criterion) {

    c.bench_function("Warren w/ GenerationGuard -- Insert 1 million u64 values.", 
        |b| b.iter(|| { 
            let mut warren1: Warren<u64, GenerationGuard> = Warren::with_region_count(1);
            (0..1000000).for_each(|i| {
                warren1.insert(i);
            });
        })
    );

    c.bench_function("Warren w/ GenerationGuard -- Insert 1 million [u64; 8] arrays.", 
        |b| b.iter(|| { 
            let mut warren2: Warren<[u64; 8], GenerationGuard> = Warren::with_region_count(1);
            (0..1000000).for_each(|i| {
                warren2.insert([i; 8]);
            });
        })
    );

    c.bench_function("Warren w/ GenerationGuard -- Insert 1 million u64 values with capacity.", 
        |b| b.iter(|| { 
            let mut warren1: Warren<u64, GenerationGuard> = Warren::with_capacity(1000000);
            (0..1000000).for_each(|i| {
                warren1.insert(i);
            });
        })
    );

    c.bench_function("Warren w/ GenerationGuard -- Insert 1 million [u64; 8] arrays with capacity.", 
        |b| b.iter(|| { 
            let mut warren2: Warren<[u64; 8], GenerationGuard> = Warren::with_capacity(1000000);
            (0..1000000).for_each(|i| {
                warren2.insert([i; 8]);
            });
        })
    );

    c.bench_function("Warren w/ GenerationGuard -- Insert and remove 1 million u64 values with capacity.", 
        |b| b.iter(|| { 
            let mut warren1: Warren<u64, GenerationGuard> = Warren::with_capacity(1000000);
            (0..1000000).for_each(|i| {
                warren1.insert(i);
            });
            (0..1000000).for_each(|i| {
                warren1.remove((i, 1));
            });
        })
    );

    c.bench_function("Warren w/ GenerationGuard -- Insert and remove 1 million [u64; 8] arrays with capacity.", 
        |b| b.iter(|| { 
            let mut warren2: Warren<[u64; 8], GenerationGuard> = Warren::with_capacity(1000000);
            (0..1000000).for_each(|i| {
                warren2.insert([i; 8]);
            });
            (0..1000000).for_each(|i| {
                warren2.remove((i, 1));
            });
        })
    );

    let mut warren1: Warren<u64, GenerationGuard> = Warren::with_region_count(1);
    (0..1000000).for_each(|i| {
        warren1.insert(i);
    });

    let mut warren2: Warren<[u64; 8], GenerationGuard> = Warren::with_region_count(1);
     (0..1000000).for_each(|i| {
        warren2.insert([i; 8]);
    });    

    c.bench_function("Warren w/ GenerationGuard -- Iterate 1 million u64 elements.", 
        |b| b.iter(|| { 
            warren1.iter().for_each(|_x| { std::hint::black_box(0); });
        })
    );

    c.bench_function("Warren w/ GenerationGuard -- Iterate 1 million [u64; 8] arrays.", 
        |b| b.iter(|| { 
            warren2.iter().for_each(|_x| { std::hint::black_box(0); });
        })
    );

    c.bench_function("Warren w/ GenerationGuard -- Iterate with mutability 1 million u64 elements.", 
        |b| b.iter(|| { 
            warren1.iter_mut().for_each(|_x| { std::hint::black_box(0); });
        })
    );

    c.bench_function("Warren w/ GenerationGuard -- Iterate with mutability 1 million [u64; 8] arrays.", 
        |b| b.iter(|| { 
            warren2.iter_mut().for_each(|_x| { std::hint::black_box(0); });
        })
    );
}

fn times_for_comparison_vec(c: &mut Criterion) {
    
    c.bench_function("Vec -- Insert 1 million u64 values.", 
        |b| b.iter(|| { 
            let mut v: Vec<u64> = Vec::with_capacity(64);
            (0..1000000).for_each(|i| {
                v.push(i);
            });
        })
    );

    c.bench_function("Vec -- Insert 1 million u64 values with capacity.", 
        |b| b.iter(|| { 
            let mut v: Vec<u64> = Vec::with_capacity(1000000);
            (0..1000000).for_each(|i| {
                v.push(i);
            });
        })
    );

    // Trust me, this takes a *while*.
    // c.bench_function("Vec -- Insert and remove 1 million u64 values with capacity.", 
    //     |b| b.iter(|| { 
    //         let mut v: Vec<u64> = Vec::with_capacity(1000000);
    //         (0..1000000).for_each(|i| {
    //             v.push(i);
    //         });
    //         (0..1000000).for_each(|_i| {
    //             v.remove(0);
    //         });
    //     })
    // );

    let mut v: Vec<u64> = Vec::with_capacity(64);
    (0..1000000).for_each(|i| {
        v.push(i);
    });

    c.bench_function("Vec -- Iterate 1 million u64 elements.", 
        |b| b.iter(|| { 
            v.iter().for_each(|_x| { std::hint::black_box(0); });
        })
    );

    c.bench_function("Vec -- Iterate with mutability 1 million u64 elements.", 
        |b| b.iter(|| { 
            v.iter_mut().for_each(|_x| { std::hint::black_box(0); });
        })
    );
}

criterion_group!(benches, timing_warren, timing_warren_with_generations, times_for_comparison_vec);
criterion_main!(benches);