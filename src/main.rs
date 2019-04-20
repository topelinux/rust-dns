extern crate c_ares;
extern crate c_ares_resolver;
extern crate futures;
extern crate tokio;
extern crate yaml_rust;

use std::error::Error;

use std::fs::File;
use std::net::Ipv4Addr;
use std::io::prelude::*;
use c_ares_resolver::{CAresFuture, FutureResolver, Options};
use futures::future::Future;
use futures::stream::FuturesUnordered;
use futures::Stream;
use std::ops::Deref;
use yaml_rust::{YamlLoader};

fn gen_future_resolve(servers: Vec<String>) -> CAresFuture<c_ares::AResults> {
    let inner: Vec<&str> = servers.iter().map(|item| item.deref()).collect::<Vec<&str>>();
    let mut option = Options::new();
    option.set_timeout(2000);
    let resolver = FutureResolver::with_options(option).expect("Failed to create resolver");
    resolver.set_servers(&inner[..]).expect("Fail to set server");
    resolver.search_a("github.com")
}

fn main() {
    let mut servers = Vec::new();
    let mut f = File::open("resolver-list.yml").unwrap();
    let mut s = String::new();
    f.read_to_string(&mut s).unwrap();

    let mut docs = YamlLoader::load_from_str(&s).unwrap().into_iter();

    docs.next().unwrap().into_iter().for_each(|item|{
        servers.push(String::from(item["ip"].as_str().unwrap()));
    });

    //let mut future_set = FuturesUnordered::<CAresFuture<c_ares::AResults>>::new();

    //println!("{:?}", servers);
    //servers
    //    .iter()
    //    .for_each(|server| future_set.push(gen_future_resolve(server)));
    let future_set = gen_future_resolve(servers);
    //let mut option = Options::new();
    //option.set_timeout(2000);
    //let resolver = FutureResolver::with_options(option).expect("Failed to create resolver");
    //resolver.set_servers(&servers[..]).expect("Fail to set server");
    //resolver.search_a("baidu.com")

    let future = future_set
        .map_err(|e| {
            println!("dns lookup failed with error '{}'", e.description());
            ()
        });
        //.collect();

    let task = future.map(|items| {
        for item in items.iter() {
            println!("result is {}", item);
        }
        //let result: Vec<c_ares::AResult> = items.iter().flat_map(|item| item.into_iter()).collect();

        //let mut to_show = result.into_iter().map(|results| {
        //    results.ipv4()
        //}).collect::<Vec<Ipv4Addr>>();

        //to_show.sort();
        //to_show.dedup();
        //println!("to show is {:?}", to_show);
    });
    tokio::run(task);
}
