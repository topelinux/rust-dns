extern crate c_ares;
extern crate c_ares_resolver;
extern crate futures;
extern crate tokio;

use std::error::Error;

use c_ares_resolver::{CAresFuture, FutureResolver, Options};
use futures::future::Future;
use futures::stream::FuturesUnordered;
use futures::Stream;
use std::net::Ipv4Addr;

fn gen_future_resolve(server: &str) -> CAresFuture<c_ares::AResults> {
    let mut option = Options::new();
    option.set_timeout(2000);
    let resolver = FutureResolver::with_options(option).expect("Failed to create resolver");
    resolver.set_servers(&[server]).expect("Fail to set server");
    resolver.query_a("baidu.com")
}

fn main() {
    // Create resolver and make a query.
    let servers = ["8.8.8.8:53", "192.168.1.1:53"];

    let mut future_set = FuturesUnordered::<CAresFuture<c_ares::AResults>>::new();

    servers
        .iter()
        .for_each(|server| future_set.push(gen_future_resolve(server)));

    let future = future_set
        .map_err(|e| {
            println!("dns lookup failed with error '{}'", e.description());
        })
        .collect();

    let task = future.map(|items| {
        let result: Vec<c_ares::AResult> = items.iter().flat_map(|item| item.into_iter()).collect();

        let mut to_show = result.into_iter().map(|results| {
            results.ipv4()
        }).collect::<Vec<Ipv4Addr>>();

        to_show.sort();
        to_show.dedup();
        println!("to show is {:?}", to_show);
    });
    tokio::run(task);
}
