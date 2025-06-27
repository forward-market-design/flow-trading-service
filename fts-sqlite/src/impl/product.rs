use crate::Db;
use fts_core::ports::ProductRepository;

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
        _as_of: Self::DateTime,
    ) -> Result<Option<ProductData>, Self::Error> {
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
        .await?;

        // TODO: We're not presently using as_of, but it should be used
        //       to gather some sort of tree-relationships.
        // To research:
        // Either as part of this function or as a dedicated, separate function,
        // we want all paths ending in self and starting at self.
        // The former is linear, the latter is a tree -- maybe there is a
        // nice infix traversal that, coupled with `depth`, can be used to (de)serialize the trees uniquely?

        Ok(product_data.map(|x| x.0))
    }
}
