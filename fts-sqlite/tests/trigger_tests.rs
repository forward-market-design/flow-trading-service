mod common;

use common::TestApp;
use fts_core::{
    models::{ConstantCurve, DateTimeRangeQuery, DemandCurve, Map, Point, PwlCurve},
    ports::{Application, DemandRepository, PortfolioRepository, ProductRepository},
};
use fts_sqlite::{Db, config::SqliteConfig, types::BidderId};

#[tokio::test]
async fn test_demand_curve_triggers() -> anyhow::Result<()> {
    let now = time::OffsetDateTime::now_utc();
    let database = Db::open(&SqliteConfig::default(), now.into()).await?;
    let app = TestApp(database);
    let db = app.database();

    let bidder_id = BidderId(uuid::Uuid::new_v4());
    let demand_id = app.generate_demand_id(&());

    // Create demand with initial curve using safe constructor
    let initial_curve: DemandCurve = PwlCurve::new(vec![
        Point {
            rate: 0.0,
            price: 10.0,
        },
        Point {
            rate: 5.0,
            price: 8.0,
        },
        Point {
            rate: 10.0,
            price: 0.0,
        },
    ])?
    .into();

    db.create_demand(
        demand_id,
        bidder_id,
        (),
        Some(initial_curve.clone()),
        now.into(),
    )
    .await?;

    // Verify demand exists and has the curve
    let demand = <Db as DemandRepository<()>>::get_demand(db, demand_id, now.into())
        .await?
        .unwrap();
    assert!(demand.curve_data.unwrap().clone().points() == initial_curve.clone().points());

    // Update demand with new curve
    let updated_curve: DemandCurve = PwlCurve::new(vec![
        fts_core::models::Point {
            rate: 0.0,
            price: 15.0,
        },
        fts_core::models::Point {
            rate: 7.0,
            price: 10.0,
        },
        fts_core::models::Point {
            rate: 12.0,
            price: 0.0,
        },
    ])?
    .into();
    let update_time = now + std::time::Duration::from_secs(5);
    let updated = <Db as DemandRepository<()>>::update_demand(
        db,
        demand_id,
        Some(updated_curve.clone()),
        update_time.into(),
    )
    .await?;
    assert!(updated);

    // Verify demand has the new curve
    let updated_demand =
        <Db as DemandRepository<()>>::get_demand(db, demand_id, update_time.into())
            .await?
            .unwrap();
    assert!(updated_demand.curve_data.unwrap().clone().points() == updated_curve.clone().points());

    // Verify we can see history
    let history = <Db as DemandRepository<()>>::get_demand_history(
        db,
        demand_id,
        DateTimeRangeQuery {
            before: None,
            after: None,
        },
        10,
    )
    .await?;

    assert_eq!(history.results.len(), 2);

    // History should be in descending order
    assert!(history.results[0].valid_from > history.results[1].valid_from);
    assert!(history.results[0].value.clone().points() == updated_curve.points());
    assert!(history.results[1].value.clone().points() == initial_curve.points());

    Ok(())
}

#[tokio::test]
async fn test_portfolio_triggers_empty_groups() -> anyhow::Result<()> {
    let now = time::OffsetDateTime::now_utc();
    let database = Db::open(&SqliteConfig::default(), now.into()).await?;
    let app = TestApp(database);
    let db = app.database();

    let bidder_id = BidderId(uuid::Uuid::new_v4());
    let portfolio_id = app.generate_portfolio_id(&());

    // Create portfolio with empty groups
    db.create_portfolio(
        portfolio_id,
        bidder_id,
        (),
        Map::default(), // empty demand group
        Map::default(), // empty product group
        now.into(),
    )
    .await?;

    // Verify portfolio exists
    let portfolio =
        <Db as PortfolioRepository<()>>::get_portfolio(db, portfolio_id, now.into()).await?;
    assert!(portfolio.is_some());
    let portfolio = portfolio.unwrap();

    assert!(portfolio.demand_group.is_empty());
    assert!(portfolio.product_group.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_portfolio_triggers_partial_updates() -> anyhow::Result<()> {
    let now = time::OffsetDateTime::now_utc();
    let database = Db::open(&SqliteConfig::default(), now.into()).await?;
    let app = TestApp(database);
    let db = app.database();

    let bidder_id = BidderId(uuid::Uuid::new_v4());
    let portfolio_id = app.generate_portfolio_id(&());
    let demand_id = app.generate_demand_id(&());
    let product_id = app.generate_product_id(&());

    // Create some entities first
    db.create_demand(demand_id, bidder_id, (), None, now.into())
        .await?;
    db.create_product(product_id, (), now.into()).await?;

    // Create portfolio with initial groups
    let mut initial_demand_group = Map::default();
    initial_demand_group.insert(demand_id, 1.0);
    let mut initial_product_group = Map::default();
    initial_product_group.insert(product_id, 2.0);

    db.create_portfolio(
        portfolio_id,
        bidder_id,
        (),
        initial_demand_group,
        initial_product_group,
        now.into(),
    )
    .await?;

    // Verify initial state
    let initial_portfolio =
        <Db as PortfolioRepository<()>>::get_portfolio(db, portfolio_id, now.into()).await?;
    assert!(initial_portfolio.is_some());
    let initial_portfolio = initial_portfolio.unwrap();
    assert_eq!(initial_portfolio.demand_group.get(&demand_id), Some(&1.0));
    assert_eq!(initial_portfolio.product_group.get(&product_id), Some(&2.0));

    // Update only demand group
    let mut updated_demand_group = Map::default();
    updated_demand_group.insert(demand_id, 1.5);
    let update_time = now + std::time::Duration::from_secs(5);

    let updated = <Db as PortfolioRepository<()>>::update_portfolio(
        db,
        portfolio_id,
        Some(updated_demand_group),
        None, // Don't update product group
        update_time.into(),
    )
    .await?;
    assert!(updated);

    // Verify demand group was updated but product group unchanged
    let updated_portfolio =
        <Db as PortfolioRepository<()>>::get_portfolio(db, portfolio_id, update_time.into())
            .await?;
    assert!(updated_portfolio.is_some());
    let updated_portfolio = updated_portfolio.unwrap();
    assert_eq!(updated_portfolio.demand_group.get(&demand_id), Some(&1.5));
    assert_eq!(updated_portfolio.product_group.get(&product_id), Some(&2.0));

    // Verify we can see demand history
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

    assert_eq!(demand_history.results.len(), 2);
    assert_eq!(demand_history.results[0].value.get(&demand_id), Some(&1.5));
    assert_eq!(demand_history.results[1].value.get(&demand_id), Some(&1.0));

    // Verify product history shows no change
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

    assert_eq!(product_history.results.len(), 1);
    assert_eq!(
        product_history.results[0].value.get(&product_id),
        Some(&2.0)
    );

    // Now update only product group
    let mut updated_product_group = Map::default();
    updated_product_group.insert(product_id, 3.0);
    let product_update_time = now + std::time::Duration::from_secs(10);
    let updated_product = <Db as PortfolioRepository<()>>::update_portfolio(
        db,
        portfolio_id,
        None, // Don't update demand group
        Some(updated_product_group),
        product_update_time.into(),
    )
    .await?;

    assert!(updated_product);

    // Verify product group was updated but demand group unchanged
    let final_portfolio = <Db as PortfolioRepository<()>>::get_portfolio(
        db,
        portfolio_id,
        product_update_time.into(),
    )
    .await?
    .unwrap();

    assert_eq!(final_portfolio.demand_group.get(&demand_id), Some(&1.5));
    assert_eq!(final_portfolio.product_group.get(&product_id), Some(&3.0));

    // Verify product history shows the update
    let final_product_history = <Db as PortfolioRepository<()>>::get_portfolio_product_history(
        db,
        portfolio_id,
        DateTimeRangeQuery {
            before: None,
            after: None,
        },
        10,
    )
    .await?;
    assert_eq!(final_product_history.results.len(), 2);
    assert_eq!(
        final_product_history.results[0].value.get(&product_id),
        Some(&3.0)
    );
    assert_eq!(
        final_product_history.results[1].value.get(&product_id),
        Some(&2.0)
    );
    // Demand history should remain unchanged
    let final_demand_history = <Db as PortfolioRepository<()>>::get_portfolio_demand_history(
        db,
        portfolio_id,
        DateTimeRangeQuery {
            before: None,
            after: None,
        },
        10,
    )
    .await?;
    assert_eq!(final_demand_history.results.len(), 2);
    assert_eq!(
        final_demand_history.results[0].value.get(&demand_id),
        Some(&1.5)
    );
    assert_eq!(
        final_demand_history.results[1].value.get(&demand_id),
        Some(&1.0)
    );

    Ok(())
}

#[tokio::test]
async fn test_portfolio_triggers_multiple_items() -> anyhow::Result<()> {
    let now = time::OffsetDateTime::now_utc();
    let database = Db::open(&SqliteConfig::default(), now.into()).await?;
    let app = TestApp(database);
    let db = app.database();

    let bidder_id = BidderId(uuid::Uuid::new_v4());
    let portfolio_id = app.generate_portfolio_id(&());

    // Create multiple demands and products
    let demand1 = app.generate_demand_id(&());
    let demand2 = app.generate_demand_id(&());
    let demand3 = app.generate_demand_id(&());
    let product1 = app.generate_product_id(&());
    let product2 = app.generate_product_id(&());

    for &demand_id in &[demand1, demand2, demand3] {
        db.create_demand(demand_id, bidder_id, (), None, now.into())
            .await?;
    }
    for &product_id in &[product1, product2] {
        db.create_product(product_id, (), now.into()).await?;
    }

    // Create portfolio with multiple items in each group
    let mut demand_group = Map::default();
    demand_group.insert(demand1, 1.0);
    demand_group.insert(demand2, 2.0);
    demand_group.insert(demand3, 3.0);

    let mut product_group = Map::default();
    product_group.insert(product1, 4.0);
    product_group.insert(product2, 5.0);

    db.create_portfolio(
        portfolio_id,
        bidder_id,
        (),
        demand_group,
        product_group,
        now.into(),
    )
    .await?;

    // Verify all items were inserted
    let portfolio = <Db as PortfolioRepository<()>>::get_portfolio(db, portfolio_id, now.into())
        .await?
        .unwrap();

    assert_eq!(portfolio.demand_group.len(), 3);
    assert_eq!(portfolio.product_group.len(), 2);
    assert_eq!(portfolio.demand_group.get(&demand1), Some(&1.0));
    assert_eq!(portfolio.demand_group.get(&demand2), Some(&2.0));
    assert_eq!(portfolio.demand_group.get(&demand3), Some(&3.0));
    assert_eq!(portfolio.product_group.get(&product1), Some(&4.0));
    assert_eq!(portfolio.product_group.get(&product2), Some(&5.0));

    // Update to remove some items and modify others
    let mut updated_demand_group = Map::default();
    updated_demand_group.insert(demand1, 1.5); // modified
    updated_demand_group.insert(demand3, 3.5); // modified
    // demand2 removed

    let mut updated_product_group = Map::default();
    updated_product_group.insert(product1, 4.5); // modified
    // product2 removed

    let update_time = now + std::time::Duration::from_secs(10);
    <Db as PortfolioRepository<()>>::update_portfolio(
        db,
        portfolio_id,
        Some(updated_demand_group.clone()),
        Some(updated_product_group.clone()),
        update_time.into(),
    )
    .await?;

    // Verify current state matches expectations
    let updated_portfolio =
        <Db as PortfolioRepository<()>>::get_portfolio(db, portfolio_id, update_time.into())
            .await?;
    assert!(updated_portfolio.is_some());
    let updated_portfolio = updated_portfolio.unwrap();

    assert_eq!(updated_portfolio.demand_group.len(), 2);
    assert_eq!(updated_portfolio.product_group.len(), 1);
    assert_eq!(updated_portfolio.demand_group.get(&demand1), Some(&1.5));
    assert_eq!(updated_portfolio.demand_group.get(&demand3), Some(&3.5));
    assert!(!updated_portfolio.demand_group.contains_key(&demand2));
    assert_eq!(updated_portfolio.product_group.get(&product1), Some(&4.5));
    assert!(!updated_portfolio.product_group.contains_key(&product2));

    // Verify history shows all changes
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

    assert_eq!(demand_history.results.len(), 2);
    // first
    assert_eq!(demand_history.results[0].value.len(), 2); // demand2 not present
    assert_eq!(demand_history.results[0].value.get(&demand1), Some(&1.5));
    assert_eq!(demand_history.results[0].value.get(&demand3), Some(&3.5));
    // second -- all items present
    assert_eq!(demand_history.results[1].value.len(), 3);
    assert_eq!(demand_history.results[1].value.get(&demand1), Some(&1.0));
    assert_eq!(demand_history.results[1].value.get(&demand2), Some(&2.0));
    assert_eq!(demand_history.results[1].value.get(&demand3), Some(&3.0));

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

    assert_eq!(product_history.results.len(), 2);
    // first
    assert_eq!(product_history.results[0].value.len(), 1); // product 2 not present
    assert_eq!(product_history.results[0].value.get(&product1), Some(&4.5));
    // second -- all items present
    assert_eq!(product_history.results[1].value.get(&product1), Some(&4.0));
    assert_eq!(product_history.results[1].value.get(&product2), Some(&5.0));

    Ok(())
}

#[tokio::test]
async fn test_demand_trigger_null_curve() -> anyhow::Result<()> {
    let now = time::OffsetDateTime::now_utc();
    let database = Db::open(&SqliteConfig::default(), now.into()).await?;
    let app = TestApp(database);
    let db = app.database();

    let bidder_id = BidderId(uuid::Uuid::new_v4());
    let demand_id = app.generate_demand_id(&());

    // Create demand with null curve
    db.create_demand(demand_id, bidder_id, (), None, now.into())
        .await?;

    // Verify demand exists with null curve
    let demand = <Db as DemandRepository<()>>::get_demand(db, demand_id, now.into())
        .await?
        .unwrap();
    assert!(demand.curve_data.is_none());

    // Update to non-null curve
    let curve: DemandCurve = ConstantCurve::new(Some(-10.0), Some(10.0), 5.0)?.into();
    let update_time = now + std::time::Duration::from_secs(5);
    let updated = <Db as DemandRepository<()>>::update_demand(
        db,
        demand_id,
        Some(curve.clone()),
        update_time.into(),
    )
    .await?;
    assert!(updated);

    // Verify curve is now set
    let updated_demand =
        <Db as DemandRepository<()>>::get_demand(db, demand_id, update_time.into())
            .await?
            .unwrap();
    assert!(updated_demand.curve_data.unwrap().clone().points() == curve.clone().points());

    // Update back to null
    let null_time = now + std::time::Duration::from_secs(10);
    let updated_again =
        <Db as DemandRepository<()>>::update_demand(db, demand_id, None, null_time.into()).await?;
    assert!(updated_again);

    // Verify curve is null again
    let final_demand = <Db as DemandRepository<()>>::get_demand(db, demand_id, null_time.into())
        .await?
        .unwrap();
    assert!(final_demand.curve_data.is_none());

    // Verify history shows all transitions
    let history = <Db as DemandRepository<()>>::get_demand_history(
        db,
        demand_id,
        DateTimeRangeQuery {
            before: None,
            after: None,
        },
        10,
    )
    .await?;

    assert_eq!(history.results.len(), 1);
    // Results should be in descending order
    assert!(history.results[0].valid_until.is_some()); // latest: null
    assert!(history.results[0].value.clone().points() == curve.points());

    Ok(())
}

#[tokio::test]
async fn test_product_tree_trigger_zero_ratio() -> anyhow::Result<()> {
    let now = time::OffsetDateTime::now_utc();
    let database = Db::open(&SqliteConfig::default(), now.into()).await?;
    let app = TestApp(database);
    let db = app.database();

    let parent = app.generate_product_id(&());
    let child = app.generate_product_id(&());

    // Create parent
    db.create_product(parent, (), now.into()).await?;

    // Create child with zero ratio
    db.partition_product(
        parent,
        vec![(child, (), 0.0)],
        (now + std::time::Duration::from_secs(1)).into(),
    )
    .await?;

    // Verify both products exist
    assert!(
        <Db as ProductRepository<()>>::get_product(db, parent, now.into())
            .await?
            .is_some()
    );
    assert!(
        <Db as ProductRepository<()>>::get_product(
            db,
            child,
            (now + std::time::Duration::from_secs(1)).into()
        )
        .await?
        .is_some()
    );

    // Create a portfolio with the parent to verify zero ratio is handled correctly
    let bidder_id = BidderId(uuid::Uuid::new_v4());
    let portfolio_id = app.generate_portfolio_id(&());

    let mut product_group = Map::default();
    product_group.insert(parent, 1.0);

    db.create_portfolio(
        portfolio_id,
        bidder_id,
        (),
        Map::default(),
        product_group,
        (now + std::time::Duration::from_secs(2)).into(),
    )
    .await?;

    let portfolio = <Db as PortfolioRepository<()>>::get_portfolio(
        db,
        portfolio_id,
        (now + std::time::Duration::from_secs(2)).into(),
    )
    .await?;
    assert!(portfolio.is_some());
    let portfolio = portfolio.unwrap();

    // The exact behavior depends on how the triggers handle zero ratios
    // At minimum, the portfolio should exist and be queryable
    assert!(!portfolio.product_group.is_empty());

    Ok(())
}
