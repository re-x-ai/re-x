//! Benchmarks for regex engine performance

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use regex::Regex;

fn bench_simple_pattern(c: &mut Criterion) {
    let pattern = r"\d+";
    let input = "The answer is 42 and the question is 6 times 7";
    let re = Regex::new(pattern).unwrap();

    c.bench_function("simple_pattern_match", |b| {
        b.iter(|| {
            let count = re.find_iter(black_box(input)).count();
            black_box(count)
        })
    });
}

fn bench_email_pattern(c: &mut Criterion) {
    let pattern = r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b";
    let input = "Contact us at support@example.com or sales@company.org for more info.";
    let re = Regex::new(pattern).unwrap();

    c.bench_function("email_pattern_match", |b| {
        b.iter(|| {
            let count = re.find_iter(black_box(input)).count();
            black_box(count)
        })
    });
}

fn bench_capture_groups(c: &mut Criterion) {
    let pattern = r"(\d{4})-(\d{2})-(\d{2})";
    let input = "Dates: 2024-01-15, 2024-02-20, 2024-03-25, 2024-04-30";
    let re = Regex::new(pattern).unwrap();

    c.bench_function("capture_groups", |b| {
        b.iter(|| {
            let caps: Vec<_> = re.captures_iter(black_box(input)).collect();
            black_box(caps)
        })
    });
}

fn bench_large_input(c: &mut Criterion) {
    let pattern = r"\b\w+\b";
    let input = "word ".repeat(10000);
    let re = Regex::new(pattern).unwrap();

    c.bench_function("large_input_10k_words", |b| {
        b.iter(|| {
            let count = re.find_iter(black_box(&input)).count();
            black_box(count)
        })
    });
}

fn bench_pattern_compilation(c: &mut Criterion) {
    let patterns = vec![
        r"\d+",
        r"\b[A-Za-z]+\b",
        r"(\d{4})-(\d{2})-(\d{2})",
        r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}",
    ];

    let mut group = c.benchmark_group("pattern_compilation");
    for pattern in patterns {
        group.bench_with_input(BenchmarkId::new("compile", pattern), pattern, |b, p| {
            b.iter(|| {
                let re = Regex::new(black_box(p)).unwrap();
                black_box(re)
            })
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_simple_pattern,
    bench_email_pattern,
    bench_capture_groups,
    bench_large_input,
    bench_pattern_compilation,
);

criterion_main!(benches);
