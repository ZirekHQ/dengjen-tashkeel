use criterion::{criterion_group, criterion_main, Criterion};
use dengjen_tashkeel::bench_internal::{tokenize, NullEngine};
use dengjen_tashkeel::do_tashkeel;
use std::hint::black_box;

const SHORT_TEXT: &str = "بسم الله الرحمن الرحيم";
const LONG_TEXT: &str = "من ذا يقارن حسنك المغري بصيف قد تجلى وفنون سحرك قد بدت في ناظري أسمى وأغلى تجني الرياح العاتيات على البراعم وهي جذلى والصيف يمضي مسرعا إذ عقده المحدود ولى ستعانقين العصر في شعري، وفيك أقول";

fn bench_tokenize(c: &mut Criterion) {
    let mut group = c.benchmark_group("tokenize");
    group.bench_function("short", |b| b.iter(|| tokenize(black_box(SHORT_TEXT))));
    group.bench_function("long", |b| b.iter(|| tokenize(black_box(LONG_TEXT))));
    group.finish();
}

fn bench_do_tashkeel_preprocessed(c: &mut Criterion) {
    let engine = NullEngine;
    let mut group = c.benchmark_group("do_tashkeel_preprocessed");
    group.bench_function("short", |b| {
        b.iter(|| do_tashkeel(&engine, black_box(SHORT_TEXT), None, true))
    });
    group.bench_function("long", |b| {
        b.iter(|| do_tashkeel(&engine, black_box(LONG_TEXT), None, true))
    });
    group.finish();
}

criterion_group!(benches, bench_tokenize, bench_do_tashkeel_preprocessed);
criterion_main!(benches);
