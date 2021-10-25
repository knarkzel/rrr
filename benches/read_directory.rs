use std::path::PathBuf;
use criterion::{criterion_group, criterion_main, Criterion};
use rrr::state::Context;

fn read_directory() {
    let mut context = Context::new().unwrap();
    context.current_dir = PathBuf::from("/bin");
    context.read_directory().unwrap();
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("read_directory", |b| b.iter(|| read_directory()));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
