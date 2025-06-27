mod common;

use common::TestApp;
use fts_core::{
    models::Portfolio,
    ports::{Application, PortfolioRepository, ProductRepository as _},
};
use fts_sqlite::{
    Db,
    config::SqliteConfig,
    types::{BidderId, DateTime},
};

#[tokio::test]
async fn test_product_expansion() -> anyhow::Result<()> {
    let now = time::OffsetDateTime::now_utc();

    let database = Db::open(&SqliteConfig::default(), now.into()).await?;
    let app = TestApp(database);

    let db = app.database();

    let bidder_id = BidderId(uuid::Uuid::new_v4());

    let food = app.generate_product_id(&());
    let fruit = app.generate_product_id(&());
    let apple = app.generate_product_id(&());
    let banana = app.generate_product_id(&());
    let vegetable = app.generate_product_id(&());
    let carrot = app.generate_product_id(&());
    let daikon = app.generate_product_id(&());

    db.create_product(food, (), now.into()).await?;

    let portfolio_id = app.generate_portfolio_id(&());
    db.create_portfolio(
        portfolio_id,
        bidder_id,
        (),
        Default::default(),
        std::iter::once((food, 1.0)).into_iter().collect(),
        (now + std::time::Duration::from_secs(1)).into(),
    )
    .await?;

    db.partition_product(
        food,
        vec![(fruit, (), 2.0), (vegetable, (), 3.0)],
        (now + std::time::Duration::from_secs(2)).into(),
    )
    .await?;

    db.partition_product(
        fruit,
        vec![(apple, (), 5.0), (banana, (), 7.0)],
        (now + std::time::Duration::from_secs(4)).into(),
    )
    .await?;

    db.partition_product(
        vegetable,
        vec![(carrot, (), 11.0), (daikon, (), 13.0)],
        (now + std::time::Duration::from_secs(6)).into(),
    )
    .await?;

    // Test at different time points
    for i in 0u64..=7 {
        let as_of = DateTime::from(now + std::time::Duration::from_secs(i));
        let Portfolio { product_group, .. } =
            <Db as PortfolioRepository<()>>::get_portfolio(db, portfolio_id, as_of)
                .await?
                .unwrap();

        // Additional assertions based on the time point
        match i {
            0 => {
                // Before any product expansion
                assert_eq!(product_group.len(), 0);
            }
            1 => {
                // After portfolio creation but before partition
                assert_eq!(product_group.len(), 1);
                assert!(product_group.contains_key(&food));
                assert_eq!(product_group.get(&food), Some(&1.0));
            }
            2 | 3 => {
                // After first partition
                assert_eq!(product_group.len(), 2);
                assert!(product_group.contains_key(&fruit));
                assert!(product_group.contains_key(&vegetable));
                assert_eq!(product_group.get(&fruit), Some(&2.0));
                assert_eq!(product_group.get(&vegetable), Some(&3.0));
            }
            4 | 5 => {
                // After fruit partition
                assert_eq!(product_group.len(), 3);
                assert!(product_group.contains_key(&vegetable));
                assert!(product_group.contains_key(&apple));
                assert!(product_group.contains_key(&banana));
                assert_eq!(product_group.get(&vegetable), Some(&3.0));
                assert_eq!(product_group.get(&apple), Some(&10.0));
                assert_eq!(product_group.get(&banana), Some(&14.0));
            }
            6 | 7 => {
                // After vegetable partition
                assert_eq!(product_group.len(), 4);
                assert!(product_group.contains_key(&apple));
                assert!(product_group.contains_key(&banana));
                assert!(product_group.contains_key(&carrot));
                assert!(product_group.contains_key(&daikon));
                assert_eq!(product_group.get(&apple), Some(&10.0));
                assert_eq!(product_group.get(&banana), Some(&14.0));
                assert_eq!(product_group.get(&carrot), Some(&33.0));
                assert_eq!(product_group.get(&daikon), Some(&39.0));
            }
            _ => {}
        }
    }

    Ok(())
}
