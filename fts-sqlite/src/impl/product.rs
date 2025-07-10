use crate::Db;
use crate::types::{ProductId, ProductRow};
use fts_core::{
    models::{ProductGroup, ProductRecord},
    ports::ProductRepository,
};

impl<ProductData: Send + Unpin + 'static + serde::Serialize + serde::de::DeserializeOwned>
    ProductRepository<ProductData> for Db
{
    async fn create_product(
        &self,
        product_id: Self::ProductId,
        app_data: ProductData,
        as_of: Self::DateTime,
    ) -> Result<ProductRecord<Self, ProductData>, Self::Error> {
        // the triggers manage all the details of updating the product tree
        // TODO: errors about temporary value being dropped if we do not
        //       explicitly bind Json(app_data) to a variable first.
        //       research this!
        let app_data = sqlx::types::Json(app_data);
        let new_product = sqlx::query_as!(
            ProductRow,
            r#"
            insert into
                product (id, as_of, app_data)
            values
                ($1, $2, jsonb($3))
            returning
                id as "id!: ProductId",
                json(app_data) as "app_data!: sqlx::types::Json<ProductData>",
                null as "parent?: sqlx::types::Json<(ProductId, f64)>",
                null as "basis?: sqlx::types::Json<ProductGroup<ProductId>>"
            "#,
            product_id,
            as_of,
            app_data,
        )
        .fetch_one(&self.writer)
        .await?;

        Ok(new_product.into())
    }

    async fn partition_product<T: Send + IntoIterator<Item = (Self::ProductId, ProductData, f64)>>(
        &self,
        product_id: Self::ProductId,
        children: T,
        as_of: Self::DateTime,
    ) -> Result<Option<Vec<ProductRecord<Self, ProductData>>>, Self::Error>
    where
        T::IntoIter: Send + ExactSizeIterator,
    {
        let npaths = sqlx::query_scalar!(
            "select count(*) from product_tree where src_id = $1 and valid_from <= $2 and ($2 < valid_until or valid_until is null)",
            product_id,
            as_of
        )
        .fetch_one(&self.reader)
        .await?;

        if npaths == 0 {
            return Ok(None);
        } else if npaths > 1 {
            return Ok(Some(Vec::new()));
        }

        let children = children.into_iter();
        if children.len() == 0 {
            return Ok(Some(Vec::new()));
        }

        let mut query_builder = sqlx::QueryBuilder::new(
            "insert into product (id, as_of, app_data, parent_id, parent_ratio) ",
        );
        // TODO: we should partition the iterator to play nice with DB limits
        query_builder
            .push_values(children, |mut b, (child_id, app_data, child_ratio)| {
                b.push_bind(child_id)
                    .push_bind(as_of)
                    .push("jsonb(")
                    .push_bind_unseparated(sqlx::types::Json(app_data))
                    .push_unseparated(")")
                    .push_bind(product_id)
                    .push_bind(child_ratio);
            })
            .push(" returning id, json(app_data), parent_id, parent_ratio");

        let result: Vec<(ProductId, sqlx::types::Json<ProductData>, ProductId, f64)> =
            query_builder
                .build_query_as()
                .fetch_all(&self.writer)
                .await?;

        Ok(Some(
            result
                .into_iter()
                .map(|(id, app_data, parent_id, parent_ratio)| ProductRecord {
                    id,
                    app_data: app_data.0,
                    parent: Some((parent_id, parent_ratio)),
                    basis: Default::default(),
                })
                .collect(),
        ))
    }

    async fn get_product(
        &self,
        product_id: Self::ProductId,
        as_of: Self::DateTime,
    ) -> Result<Option<ProductRecord<Self, ProductData>>, Self::Error> {
        let record: Option<ProductRecord<Self, ProductData>> = sqlx::query_as!(
            ProductRow,
            r#"
            select
                id as "id!: ProductId",
                json(app_data) as "app_data!: sqlx::types::Json<ProductData>",
                case
                    when
                        parent_id is null
                    then
                        null
                    else
                        json_array(parent_id, parent_ratio)
                    end as "parent?: sqlx::types::Json<(ProductId, f64)>",
                null as "basis?: sqlx::types::Json<ProductGroup<ProductId>>"
            from
                product
            where
                id = $1
            "#,
            product_id
        )
        .fetch_optional(&self.reader)
        .await?
        .map(Into::into);

        if let Some(mut record) = record {
            let children = sqlx::query!(
                r#"
                select
                    dst_id as "child_id!: ProductId",
                    ratio as "ratio!: f64"
                from
                    product_tree
                where
                    src_id = $1
                and
                    valid_from <= $2
                and
                    ($2 < valid_until or valid_until is null)
                "#,
                product_id,
                as_of
            )
            .fetch_all(&self.reader)
            .await?
            .iter()
            .map(|record| (record.child_id, record.ratio))
            .collect();

            record.basis = children;
            Ok(Some(record))
        } else {
            Ok(None)
        }
    }
}
