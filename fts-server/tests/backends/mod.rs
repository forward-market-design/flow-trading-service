#![allow(unused_macros)]

use std::{any::Any, marker::PhantomData};

use fts_core::ports::MarketRepository;
use fts_demo::{Config as simple_config, db::Database as simple_database};
use rstest_reuse::template;
// use pg_backend::db::initialize_db;
// use testcontainers_modules::{
//     postgres::Postgres,
//     testcontainers::{runners::AsyncRunner, ImageExt},
// };

// pub async fn launch_pg_backend() -> (impl MarketRepository, Box<dyn Any>) {
//     let node = Postgres::default().with_tag("17").start().await.unwrap();
//     let connection_string = &format!(
//         "postgres://postgres:postgres@localhost:{}/postgres",
//         node.get_host_port_ipv4(5432).await.unwrap()
//     );
//     let db = initialize_db(&connection_string)
//         .await
//         .expect("could not initialize database");

//     (db, Box::new(node))
// }

pub async fn launch_simple_backend() -> (impl MarketRepository, Box<dyn Any>) {
    let db = simple_database::open(
        None,
        Some(&simple_config {
            trade_rate: std::time::Duration::from_secs(60 * 60),
        }),
    )
    .expect("could not open in-memory db");

    (db, Box::new(PhantomData::<()>))
}

// This creates a testing "template" to allow for the injection of each backend
// implementation

#[template]
#[rstest]
//#[case::pg(launch_pg_backend())]
#[case::sqlite(launch_simple_backend())]
#[tokio::test]
pub async fn all_backends(
    #[case] backend: impl Future<Output = (impl MarketRepository, Box<dyn Any>)>,
) -> () {
}
