use diesel;
use diesel::connection::AnsiTransactionManager;
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::query_dsl::LoadQuery;
use diesel::query_dsl::RunQueryDsl;
use diesel::Connection;
use failure::Error as FailureError;

use stq_static_resources::Currency;
use stq_types::{BaseProductId, ProductId, UserId};

use models::{BaseProduct, NewProduct, Product, Store, UpdateProduct};
use repos::legacy_acl::*;
use schema::base_products::dsl as BaseProducts;
use schema::products::dsl::*;
use schema::stores::dsl as Stores;

use super::acl;
use super::types::RepoResult;
use models::authorization::*;

/// Products repository, responsible for handling products
pub struct ProductsRepoImpl<'a, T: Connection<Backend = Pg, TransactionManager = AnsiTransactionManager> + 'static> {
    pub db_conn: &'a T,
    pub acl: Box<Acl<Resource, Action, Scope, FailureError, Product>>,
}

pub trait ProductsRepo {
    /// Find specific product by ID
    fn find(&self, product_id: ProductId) -> RepoResult<Option<Product>>;

    /// Find specific product by IDs
    fn find_many(&self, product_ids: Vec<ProductId>) -> RepoResult<Vec<Product>>;

    /// Returns list of products, limited by `from` and `count` parameters
    fn list(&self, from: i32, count: i32) -> RepoResult<Vec<Product>>;

    /// Returns list of products with base id
    fn find_with_base_id(&self, base_id: BaseProductId) -> RepoResult<Vec<Product>>;

    /// Creates new product
    fn create(&self, payload: NewProduct) -> RepoResult<Product>;

    /// Updates specific product
    fn update(&self, product_id: ProductId, payload: UpdateProduct) -> RepoResult<Product>;

    /// Deactivates specific product
    fn deactivate(&self, product_id: ProductId) -> RepoResult<Product>;

    /// Update currency on all prodouct with base_product_id
    fn update_currency(&self, currency: Currency, base_product_id: BaseProductId) -> RepoResult<usize>;
}

impl<'a, T: Connection<Backend = Pg, TransactionManager = AnsiTransactionManager> + 'static> ProductsRepoImpl<'a, T> {
    pub fn new(db_conn: &'a T, acl: Box<Acl<Resource, Action, Scope, FailureError, Product>>) -> Self {
        Self { db_conn, acl }
    }

    fn execute_query<Ty: Send + 'static, U: LoadQuery<T, Ty> + Send + 'static>(&self, query: U) -> RepoResult<Ty> {
        query.get_result::<Ty>(self.db_conn).map_err(From::from)
    }
}

impl<'a, T: Connection<Backend = Pg, TransactionManager = AnsiTransactionManager> + 'static> ProductsRepo for ProductsRepoImpl<'a, T> {
    /// Find specific product by ID
    fn find(&self, product_id_arg: ProductId) -> RepoResult<Option<Product>> {
        debug!("Find in products with id {}.", product_id_arg);
        let query = products.find(product_id_arg).filter(is_active.eq(true));
        query
            .get_result(self.db_conn)
            .optional()
            .map_err(From::from)
            .and_then(|product: Option<Product>| {
                if let Some(ref product) = product {
                    acl::check(&*self.acl, Resource::Products, Action::Read, self, Some(product))?;
                };
                Ok(product)
            }).map_err(|e: FailureError| e.context(format!("Find product with id: {} error occured", product_id_arg)).into())
    }

    /// Find specific product by IDs
    fn find_many(&self, product_ids: Vec<ProductId>) -> RepoResult<Vec<Product>> {
        debug!("Find in products {:?}.", product_ids);
        let query = products.filter(id.eq_any(product_ids.clone())).filter(is_active.eq(true));

        query
            .get_results(self.db_conn)
            .map_err(From::from)
            .and_then(|products_res: Vec<Product>| {
                for product in &products_res {
                    acl::check(&*self.acl, Resource::Products, Action::Read, self, Some(&product))?;
                }
                Ok(products_res.clone())
            }).map_err(move |e: FailureError| e.context(format!("Find in products {:?} error occured.", product_ids)).into())
    }

    /// Creates new product
    fn create(&self, payload: NewProduct) -> RepoResult<Product> {
        debug!("Create products {:?}.", payload);
        let query_product = diesel::insert_into(products).values(&payload);
        query_product
            .get_result::<Product>(self.db_conn)
            .map_err(From::from)
            .and_then(|prod| acl::check(&*self.acl, Resource::Products, Action::Create, self, Some(&prod)).and_then(|_| Ok(prod)))
            .map_err(|e: FailureError| e.context(format!("Create products {:?} error occured.", payload)).into())
    }

    /// Returns list of products, limited by `from` and `count` parameters
    fn list(&self, from: i32, count: i32) -> RepoResult<Vec<Product>> {
        debug!("Find in products from {} count {}.", from, count);
        let query = products
            .filter(is_active.eq(true))
            .filter(id.ge(from))
            .order(id)
            .limit(count.into());

        query
            .get_results(self.db_conn)
            .map_err(From::from)
            .and_then(|products_res: Vec<Product>| {
                for product in &products_res {
                    acl::check(&*self.acl, Resource::Products, Action::Read, self, Some(&product))?;
                }
                Ok(products_res.clone())
            }).map_err(|e: FailureError| {
                e.context(format!("Find in products from {} count {} error occured.", from, count))
                    .into()
            })
    }

    /// Returns list of products with base id
    fn find_with_base_id(&self, base_id_arg: BaseProductId) -> RepoResult<Vec<Product>> {
        debug!("Find in products with id {}.", base_id_arg);
        let query = products
            .filter(base_product_id.eq(base_id_arg))
            .filter(is_active.eq(true))
            .order_by(id.desc());

        query
            .get_results(self.db_conn)
            .map_err(From::from)
            .and_then(|products_res: Vec<Product>| {
                for product in &products_res {
                    acl::check(&*self.acl, Resource::Products, Action::Read, self, Some(&product))?;
                }
                Ok(products_res.clone())
            }).map_err(|e: FailureError| e.context(format!("Find in products with id {} error occured.", base_id_arg)).into())
    }

    /// Updates specific product
    fn update(&self, product_id_arg: ProductId, payload: UpdateProduct) -> RepoResult<Product> {
        debug!("Updating product with id {} and payload {:?}.", product_id_arg, payload);
        self.execute_query(products.find(product_id_arg))
            .and_then(|product: Product| acl::check(&*self.acl, Resource::Products, Action::Update, self, Some(&product)))
            .and_then(|_| {
                let filter = products.filter(id.eq(product_id_arg)).filter(is_active.eq(true));

                let query = diesel::update(filter).set(&payload);
                query.get_result::<Product>(self.db_conn).map_err(From::from)
            }).map_err(|e: FailureError| {
                e.context(format!(
                    "Updating product with id {} and payload {:?} error occured.",
                    product_id_arg, payload
                )).into()
            })
    }

    /// Deactivates specific product
    fn deactivate(&self, product_id_arg: ProductId) -> RepoResult<Product> {
        debug!("Deactivate product with id {}.", product_id_arg);
        self.execute_query(products.find(product_id_arg))
            .and_then(|product: Product| acl::check(&*self.acl, Resource::Products, Action::Delete, self, Some(&product)))
            .and_then(|_| {
                let filter = products.filter(id.eq(product_id_arg)).filter(is_active.eq(true));
                let query = diesel::update(filter).set(is_active.eq(false));
                self.execute_query(query)
            }).map_err(|e: FailureError| {
                e.context(format!("Deactivate product with id {} error occured.", product_id_arg))
                    .into()
            })
    }

    /// Update currency on all product with base_product_id
    fn update_currency(&self, currency_arg: Currency, base_product_id_arg: BaseProductId) -> RepoResult<usize> {
        debug!(
            "Setting currency {} on all product with base_product_id {}.",
            currency_arg, base_product_id_arg
        );

        let query = products.filter(base_product_id.eq(base_product_id_arg)).filter(is_active.eq(true));

        query
            .get_results(self.db_conn)
            .map_err(From::from)
            .and_then(|products_res: Vec<Product>| {
                for product in &products_res {
                    acl::check(&*self.acl, Resource::Products, Action::Read, self, Some(&product))?;
                }
                Ok(())
            }).and_then(|_| {
                diesel::update(products)
                    .filter(base_product_id.eq(base_product_id_arg))
                    .filter(is_active.eq(true))
                    .set(currency.eq(currency_arg))
                    .execute(self.db_conn)
                    .map_err(From::from)
            }).map_err(|e: FailureError| {
                e.context(format!(
                    "Setting currency {} on all product with base_product_id {} error occured.",
                    currency_arg, base_product_id_arg
                )).into()
            })
    }
}

impl<'a, T: Connection<Backend = Pg, TransactionManager = AnsiTransactionManager> + 'static> CheckScope<Scope, Product>
    for ProductsRepoImpl<'a, T>
{
    fn is_in_scope(&self, user_id: UserId, scope: &Scope, obj: Option<&Product>) -> bool {
        match *scope {
            Scope::All => true,
            Scope::Owned => {
                if let Some(product) = obj {
                    BaseProducts::base_products
                        .find(product.base_product_id)
                        .get_result::<BaseProduct>(self.db_conn)
                        .and_then(|base_prod: BaseProduct| {
                            Stores::stores
                                .find(base_prod.store_id)
                                .get_result::<Store>(self.db_conn)
                                .and_then(|store: Store| Ok(store.user_id == user_id))
                        }).ok()
                        .unwrap_or(false)
                } else {
                    false
                }
            }
        }
    }
}
