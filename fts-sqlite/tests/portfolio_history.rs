mod common;

use common::TestApp;
use fts_core::{
    models::{DateTimeRangeQuery, DemandCurve, DemandGroup, Basis},
    ports::{Application, DemandRepository as _, PortfolioRepository, ProductRepository as _},
};
use fts_sqlite::{Db, config::SqliteConfig, types::BidderId};

#[tokio::test]
async fn test_portfolio_history() -> anyhow::Result<()> {
    let now = time::OffsetDateTime::now_utc();
    let database = Db::open(&SqliteConfig::default(), now.into()).await?;
    let app = TestApp(database);

    let db = app.database();

    let bidder_id = BidderId(uuid::Uuid::new_v4());
    let portfolio_id = app.generate_portfolio_id(&()).0;

    // Create some products and demands for testing
    let product1 = app.generate_product_id(&()).0;
    let product2 = app.generate_product_id(&()).0;
    let demand1 = app.generate_demand_id(&()).0;
    let demand2 = app.generate_demand_id(&()).0;

    // Create products and demands
    db.create_product(product1, (), now.into()).await?;
    db.create_product(
        product2,
        (),
        (now + std::time::Duration::from_secs(1)).into(),
    )
    .await?;

    db.create_demand(demand1, bidder_id, (), DemandCurve::None, now.into())
        .await?;

    db.create_demand(
        demand2,
        bidder_id,
        (),
        DemandCurve::None,
        (now + std::time::Duration::from_secs(1)).into(),
    )
    .await?;

    // Create initial portfolio
    let mut initial_basis = Basis::default();
    initial_basis.insert(product1, 1.0);

    let mut initial_demand_group = DemandGroup::default();
    initial_demand_group.insert(demand1, 0.5);

    db.create_portfolio(
        portfolio_id,
        bidder_id,
        (),
        initial_demand_group,
        initial_basis,
        (now + std::time::Duration::from_secs(2)).into(),
    )
    .await?;

    // Update portfolio to include more products and demands
    let mut updated_basis = Basis::default();
    updated_basis.insert(product1, 0.7);
    updated_basis.insert(product2, 1.5);

    let mut updated_demand_group = DemandGroup::default();
    updated_demand_group.insert(demand1, 0.3);
    updated_demand_group.insert(demand2, 0.8);

    <Db as PortfolioRepository<()>>::update_portfolio_groups(
        db,
        portfolio_id,
        updated_demand_group,
        updated_basis,
        (now + std::time::Duration::from_secs(4)).into(),
    )
    .await?;

    // Test demand history
    let demand_history = <Db as PortfolioRepository<()>>::get_portfolio_demand_history(
        db,
        portfolio_id,
        DateTimeRangeQuery {
            before: None,
            after: None,
        },
        10,
    )
    .await?;

    assert_eq!(
        demand_history.results.len(),
        2,
        "Should have 2 demand history records"
    );

    // Verify the records are in descending order
    let first_record = &demand_history.results[0];
    let second_record = &demand_history.results[1];

    // First record should be the most recent update
    assert_eq!(
        first_record.value.len(),
        2,
        "Latest demand group should have 2 demands"
    );
    assert_eq!(first_record.value.get(&demand1), Some(&0.3));
    assert_eq!(first_record.value.get(&demand2), Some(&0.8));

    // Second record should be the initial creation
    assert_eq!(
        second_record.value.len(),
        1,
        "Initial demand group should have 1 demand"
    );
    assert_eq!(second_record.value.get(&demand1), Some(&0.5));

    // Test product history
    let product_history = <Db as PortfolioRepository<()>>::get_portfolio_product_history(
        db,
        portfolio_id,
        DateTimeRangeQuery {
            before: None,
            after: None,
        },
        10,
    )
    .await?;

    assert_eq!(
        product_history.results.len(),
        2,
        "Should have 2 product history records"
    );

    // Verify the records are in descending order
    let first_product_record = &product_history.results[0];
    let second_product_record = &product_history.results[1];

    // First record should be the most recent update
    assert_eq!(
        first_product_record.value.len(),
        2,
        "Latest product group should have 2 products"
    );
    assert_eq!(first_product_record.value.get(&product1), Some(&0.7));
    assert_eq!(first_product_record.value.get(&product2), Some(&1.5));

    // Second record should be the initial creation
    assert_eq!(
        second_product_record.value.len(),
        1,
        "Initial product group should have 1 product"
    );
    assert_eq!(second_product_record.value.get(&product1), Some(&1.0));

    // Test pagination
    let limited_history = <Db as PortfolioRepository<()>>::get_portfolio_demand_history(
        db,
        portfolio_id,
        DateTimeRangeQuery {
            before: None,
            after: None,
        },
        1,
    )
    .await?;

    assert_eq!(
        limited_history.results.len(),
        1,
        "Should return only 1 record with limit=1"
    );
    assert!(
        limited_history.more.is_some(),
        "Should have more records available"
    );

    Ok(())
}
