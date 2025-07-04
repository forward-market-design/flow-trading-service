use crate::models::ProductRecord;

/// Repository interface for product hierarchy management.
///
/// This trait encapsulates the functionality related to defining and maintaining
/// a hierarchical tree of products. Products can be partitioned into child products,
/// enabling fine-grained control over tradeable assets.
///
/// # Product Hierarchy
///
/// Products form a tree structure where:
/// - Root products have no parent
/// - Products can be partitioned into weighted children
/// - Child weights represent the proportion of the parent product
pub trait ProductRepository<ProductData>: super::Repository {
    /// Define a new root product with no parent.
    fn create_product(
        &self,
        product_id: Self::ProductId,
        app_data: ProductData,
        as_of: Self::DateTime,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;

    /// Partition an existing product into new weighted children.
    ///
    /// This operation creates child products that represent portions of the parent.
    /// The weights determine how allocations to the parent are distributed to children.
    ///
    /// # Returns
    ///
    /// Ok(n) if successful, where n is the number of products created.
    ///
    /// # Errors
    ///
    /// Should fail if the product_id does not already exist.
    fn partition_product<T: Send + IntoIterator<Item = (Self::ProductId, ProductData, f64)>>(
        &self,
        product_id: Self::ProductId,
        children: T,
        as_of: Self::DateTime,
    ) -> impl Future<Output = Result<usize, Self::Error>> + Send
    where
        T::IntoIter: Send + ExactSizeIterator;

    /// Get the data associated with a product at a specific time.
    ///
    /// # Returns
    ///
    /// The product data if it exists at the specified time, None otherwise.
    fn get_product(
        &self,
        product_id: Self::ProductId,
        as_of: Self::DateTime,
    ) -> impl Future<
        Output = Result<Option<ProductRecord<Self::ProductId, ProductData>>, Self::Error>,
    > + Send;
}
