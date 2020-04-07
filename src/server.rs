use std::collections::HashSet;
use std::convert::Infallible;
use std::sync::Arc;

use chrono::{NaiveDateTime, TimeZone, Utc};
use hyper::{service::make_service_fn, Server};
use serde::{Deserialize, Serialize};
use warp::Filter;

use crate::ccn::Ccn;
use crate::store::CcnStore;
use crate::tcn::Tcn;
use crate::utils::{api_reply, response_format};

#[derive(Debug, Clone)]
pub struct BackendState {
    store: Arc<CcnStore>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CheckRequest {
    contacts: HashSet<Tcn>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CcnStoreRequest {
    ccn: Ccn,
}

pub async fn serve() {
    let backend_state = Arc::new(BackendState {
        store: Arc::new(CcnStore::open("db").unwrap()),
    });

    macro_rules! pass_state {
        () => {
            warp::any().map({
                let backend_state = backend_state.clone();
                move || backend_state.clone()
            })
        };
    }

    let make_svc = make_service_fn(move |_| {
        let fetch = warp::path("fetch")
            .and(warp::path::param())
            .and(pass_state!())
            .map(|ts: u64, state: Arc<BackendState>| {
                let ts = Utc.from_utc_datetime(&NaiveDateTime::from_timestamp(ts as i64, 0));
                state.store.fetch_buckets(ts).unwrap()
            })
            .map(api_reply);

        let check = warp::path("check")
            .and(warp::body::json())
            .and(pass_state!())
            .map(|data: CheckRequest, state: Arc<BackendState>| {
                for ccn0 in state.store.fetch_active_buckets().unwrap().into_iter() {
                    for ccn in ccn0.generate_ccns().take(14) {
                        for tcn in ccn.generate_tcns().take(1440) {
                            if data.contacts.contains(&tcn) {
                                return true;
                            }
                        }
                    }
                }
                false
            })
            .map(api_reply);

        let submit = warp::path("submit")
            .and(warp::body::json())
            .and(pass_state!())
            .map(|data: CcnStoreRequest, state: Arc<BackendState>| {
                state.store.add_ccn(data.ccn).unwrap()
            })
            .map(api_reply);

        let routes = response_format().and(fetch.or(check).or(submit));
        let svc = warp::service(routes);
        async move { Ok::<_, Infallible>(svc) }
    });

    let mut listenfd = listenfd::ListenFd::from_env();
    let server = if let Some(l) = listenfd.take_tcp_listener(0).unwrap() {
        Server::from_tcp(l).unwrap()
    } else {
        Server::bind(&([127, 0, 0, 1], 5000).into())
    };
    server.serve(make_svc).await.unwrap();
}
