extern crate c_ares;
extern crate c_ares_resolver;
extern crate futures;
extern crate getopts;
extern crate indicatif;
extern crate tokio;
extern crate yaml_rust;

use c_ares::Error;
use c_ares_resolver::{CAresFuture, FutureResolver, Options};
use futures::future::Future;
use futures::stream::FuturesUnordered;
use futures::Stream;
use getopts::Options as AppOptions;
use indicatif::ProgressBar;
use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::net::Ipv4Addr;
use std::sync::{Arc, Mutex};
use yaml_rust::YamlLoader;
use std::collections::HashMap;

struct ProgressState {
    server_num: usize,
    response_num: usize,
    timeout_num: usize,
    connect_refused_num: usize,
    pb: ProgressBar,
}

impl ProgressState {
    fn new(server_num: usize) -> Self {
        ProgressState {
            server_num,
            response_num: 0,
            timeout_num: 0,
            connect_refused_num: 0,
            pb: ProgressBar::new(server_num as u64),
        }
    }
}

fn gen_future_resolve(server: &str, domain: &str) -> CAresFuture<c_ares::AResults> {
    let mut option = Options::new();
    option.set_timeout(2000);
    let resolver = FutureResolver::with_options(option).expect("Failed to create resolver");
    resolver.set_servers(&[server]).expect("Fail to set server");
    resolver.query_a(domain)
}

fn usage(opts: AppOptions) {
    let brief = ("Usage: rust-dns <domain>").to_string();
    print!("{}", opts.usage(&brief));
}

fn main() {
    let mut opts = AppOptions::new();

    opts.optflag("h", "help", "Print this help menu");
    let matches = match opts.parse(env::args().skip(1)) {
        Ok(m) => m,
        Err(f) => panic!(f.to_string()),
    };

    if matches.opt_present("h") {
        usage(opts);
        return;
    }

    let domain = if !matches.free.is_empty() {
        matches.free[0].clone()
    } else {
        usage(opts);
        return;
    };

    let mut servers = Vec::new();
    let mut f = File::open("resolver-list.yml").unwrap();
    let mut s = String::new();
    f.read_to_string(&mut s).unwrap();

    let mut docs = YamlLoader::load_from_str(&s).unwrap().into_iter();

    docs.next().unwrap().into_iter().for_each(|item| {
        let ip = String::from(item["ip"].as_str().unwrap());
        servers.push(ip);
    });

    let mut future_set = FuturesUnordered::<CAresFuture<c_ares::AResults>>::new();

    let count = servers.len();
    let progress_state = Arc::new(Mutex::new(ProgressState::new(count)));
    let processs_copy = Arc::clone(&progress_state);

    println!("Will Query {} servers for domain {}", servers.len(), domain);
    servers
        .iter()
        .for_each(|server| future_set.push(gen_future_resolve(server, &domain)));

    let future = future_set
        .then(move |ret| {
            let mut state = processs_copy.lock().unwrap();

            state.pb.inc(1);
            match ret {
                Ok(item) => {
                    state.response_num += 1;
                    Ok(Some(item))
                }
                Err(err) => {
                    match err {
                        Error::ETIMEOUT => {
                            state.timeout_num += 1;
                        }
                        Error::ECONNREFUSED => {
                            state.connect_refused_num += 1;
                        }
                        _ => {}
                    }
                    //println!("err is {}", _e);
                    Ok(None)
                }
            }
        })
        .filter_map(|item| item)
        .collect();

    let task = future.map(move |items| {
        let state = progress_state.lock().unwrap();

        state.pb.finish();
        let result: Vec<c_ares::AResult> = items.iter().flat_map(|item| item.into_iter()).collect();

        let to_show = result
            .into_iter()
            .map(|results| results.ipv4())
            .collect::<Vec<Ipv4Addr>>();

        let frequencies = to_show.iter().fold(HashMap::new(), |mut freqs, value| {
            *freqs.entry(value).or_insert(1) += 1;
            freqs
        });

        let mut ips = frequencies
            .into_iter()
            .collect::<Vec<(&Ipv4Addr, i32)>>();

        ips.sort_by(|(_, count_a), (_, count_b)| count_b.cmp(count_a));

        println!("Query {} servers", state.server_num);
        println!("Repsonse servers: {}", state.response_num);
        println!("Timeout servers: {}", state.timeout_num);
        println!("Connect refused servers: {}", state.connect_refused_num);
        println!("IPs List: IpAddr\t count");
        ips.iter().for_each(|(ip, num)| {
            println!("\t{}\t {}", ip, num);
        });
    });
    tokio::run(task);
}
