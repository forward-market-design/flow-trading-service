use crate::Db;
use crate::types::ProductId;
use fts_core::{models::ProductRecord, ports::ProductRepository};

impl<ProductData: Send + Unpin + 'static + serde::Serialize + serde::de::DeserializeOwned>
    ProductRepository<ProductData> for Db
{
    async fn create_product(
        &self,
        product_id: Self::ProductId,
        app_data: ProductData,
        as_of: Self::DateTime,
    ) -> Result<(), Self::Error> {
        // the triggers manage all the details of updating the product tree
        // TODO: errors about temporary value being dropped if we do not
        //       explicitly bind Json(app_data) to a variable first.
        //       research this!
        let app_data = sqlx::types::Json(app_data);
        sqlx::query!(
            r#"
            insert into
                product (id, as_of, app_data)
            values
                ($1, $2, jsonb($3))
            "#,
            product_id,
            as_of,
            app_data,
        )
        .execute(&self.writer)
        .await?;

        Ok(())
    }

    async fn partition_product<T: Send + IntoIterator<Item = (Self::ProductId, ProductData, f64)>>(
        &self,
        product_id: Self::ProductId,
        children: T,
        as_of: Self::DateTime,
    ) -> Result<usize, Self::Error>
    where
        T::IntoIter: Send + ExactSizeIterator,
    {
        let children = children.into_iter();
        if children.len() == 0 {
            return Ok(0);
        }

        let mut query_builder = sqlx::QueryBuilder::new(
            "insert into product (id, as_of, app_data, parent_id, parent_ratio) ",
        );
        // TODO: we should partition the iterator to play nice with DB limits
        query_builder.push_values(children, |mut b, (child_id, app_data, child_ratio)| {
            b.push_bind(child_id)
                .push_bind(as_of)
                .push("jsonb(")
                .push_bind_unseparated(sqlx::types::Json(app_data))
                .push_unseparated(")")
                .push_bind(product_id)
                .push_bind(child_ratio);
        });

        let result = query_builder.build().execute(&self.writer).await?;

        Ok(result.rows_affected() as usize)
    }

    async fn get_product(
        &self,
        product_id: Self::ProductId,
        as_of: Self::DateTime,
    ) -> Result<Option<ProductRecord<Self::ProductId, ProductData>>, Self::Error> {
        let product_data = sqlx::query_scalar!(
            r#"
            select
                json(app_data) as "data!: sqlx::types::Json::<ProductData>"
            from
                product
            where
                id = $1
            "#,
            product_id
        )
        .fetch_optional(&self.reader)
        .await?
        .map(|x| x.0);

        let parent = sqlx::query!(
            r#"
            select
                src_id as "parent_id!: ProductId",
                ratio as "ratio!: f64"
            from
                product_tree
            where
                dst_id = $1
            and
                depth = 1
            and
                valid_from <= $2
            and
                ($2 < valid_until or valid_until is null)
            "#,
            product_id,
            as_of,
        )
        .fetch_optional(&self.reader)
        .await?
        .map(|record| (record.parent_id, record.ratio));

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
                depth = 1
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

        // TODO: Chaining these queries is probably okay in SQLite,
        //       but we should be better.
        Ok(product_data.map(|app_data| ProductRecord {
            id: product_id,
            app_data,
            parent,
            children,
        }))
    }
}
