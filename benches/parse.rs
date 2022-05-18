use bencher::benchmark_group;
use bencher::benchmark_main;
use bencher::Bencher;

use http::header::{HeaderMap, HeaderValue, ACCEPT_ENCODING};

fn single_encoding(bench: &mut Bencher) {
    let mut headers = HeaderMap::new();
    headers.insert(ACCEPT_ENCODING, HeaderValue::from_str("gzip").unwrap());

    bench.iter(|| {
        let _ = fly_accept_encoding::parse(&headers);
    })
}

fn multi_encoding(bench: &mut Bencher) {
    let mut headers = HeaderMap::new();
    headers.insert(
        ACCEPT_ENCODING,
        HeaderValue::from_str("gzip, deflate, br").unwrap(),
    );

    bench.iter(|| {
        let _ = fly_accept_encoding::parse(&headers);
    })
}

benchmark_group!(benches, single_encoding, multi_encoding);
benchmark_main!(benches);
