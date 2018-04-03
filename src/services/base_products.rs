//! Base product service
use std::collections::{HashMap, HashSet};

use futures::future::*;
use futures_cpupool::CpuPool;
use diesel::Connection;
use diesel::connection::AnsiTransactionManager;
use diesel::pg::Pg;
use r2d2::{ManageConnection, Pool};

use models::*;
use elastic::{ProductsElastic, ProductsElasticImpl};
use super::types::ServiceFuture;
use repos::types::RepoResult;
use repos::ReposFactory;
use super::error::ServiceError;
use repos::error::RepoError;

use stq_http::client::ClientHandle;

const MAX_PRODUCTS_SEARCH_COUNT: i64 = 1000;

pub trait BaseProductsService {
    /// Find product by name limited by `count` and `offset` parameters
    fn search_by_name(&self, prod: SearchProductsByName, count: i64, offset: i64) -> ServiceFuture<Vec<BaseProductWithVariants>>;
    /// Find product by views limited by `count` and `offset` parameters
    fn search_most_viewed(&self, prod: MostViewedProducts, count: i64, offset: i64) -> ServiceFuture<Vec<BaseProductWithVariants>>;
    /// Find product by dicount pattern limited by `count` and `offset` parameters
    fn search_most_discount(&self, prod: MostDiscountProducts, count: i64, offset: i64) -> ServiceFuture<Vec<BaseProductWithVariants>>;
    /// auto complete limited by `count` and `offset` parameters
    fn auto_complete(&self, name: String, count: i64, offset: i64) -> ServiceFuture<Vec<String>>;
    /// search filters
    fn search_filters(&self, name: String) -> ServiceFuture<SearchOptions>;
    /// Returns product by ID
    fn get(&self, product_id: i32) -> ServiceFuture<BaseProduct>;
    /// Returns product by ID
    fn get_with_variants(&self, product_id: i32) -> ServiceFuture<BaseProductWithVariants>;
    /// Deactivates specific product
    fn deactivate(&self, product_id: i32) -> ServiceFuture<BaseProduct>;
    /// Creates base product
    fn create(&self, payload: NewBaseProduct) -> ServiceFuture<BaseProduct>;
    /// Lists base products limited by `from` and `count` parameters
    fn list(&self, from: i32, count: i64) -> ServiceFuture<Vec<BaseProduct>>;
    /// Returns list of base_products by store id and exclude base_product_id_arg, limited by 10
    fn list_with_variants(&self, store_id: i32, base_product_id: i32) -> ServiceFuture<Vec<BaseProductWithVariants>>;
    /// Updates base product
    fn update(&self, product_id: i32, payload: UpdateBaseProduct) -> ServiceFuture<BaseProduct>;
}

/// Products services, responsible for Product-related CRUD operations
pub struct BaseProductsServiceImpl<
    T: Connection<Backend = Pg, TransactionManager = AnsiTransactionManager> + 'static,
    M: ManageConnection<Connection = T>,
    F: ReposFactory<T>,
> {
    pub db_pool: Pool<M>,
    pub cpu_pool: CpuPool,
    pub user_id: Option<i32>,
    pub client_handle: ClientHandle,
    pub elastic_address: String,
    pub repo_factory: F,
}

impl<
    T: Connection<Backend = Pg, TransactionManager = AnsiTransactionManager> + 'static,
    M: ManageConnection<Connection = T>,
    F: ReposFactory<T>,
> BaseProductsServiceImpl<T, M, F>
{
    pub fn new(
        db_pool: Pool<M>,
        cpu_pool: CpuPool,
        user_id: Option<i32>,
        client_handle: ClientHandle,
        elastic_address: String,
        repo_factory: F,
    ) -> Self {
        Self {
            db_pool,
            cpu_pool,
            user_id,
            client_handle,
            elastic_address,
            repo_factory,
        }
    }
}

impl<
    T: Connection<Backend = Pg, TransactionManager = AnsiTransactionManager> + 'static,
    M: ManageConnection<Connection = T>,
    F: ReposFactory<T>,
> BaseProductsService for BaseProductsServiceImpl<T, M, F>
{
    fn search_by_name(&self, search_product: SearchProductsByName, count: i64, offset: i64) -> ServiceFuture<Vec<BaseProductWithVariants>> {
        let products = {
            let client_handle = self.client_handle.clone();
            let address = self.elastic_address.clone();
            let products_el = ProductsElasticImpl::new(client_handle, address);
            products_el
                .search_by_name(search_product, count, offset)
                .map_err(ServiceError::from)
        };

        Box::new(products.and_then({
            let cpu_pool = self.cpu_pool.clone();
            let db_pool = self.db_pool.clone();
            let user_id = self.user_id;
            let repo_factory = self.repo_factory.clone();
            move |el_products| {
                cpu_pool.spawn_fn(move || {
                    db_pool
                        .get()
                        .map_err(|e| {
                            error!(
                                "Could not get connection to db from pool! {}",
                                e.to_string()
                            );
                            ServiceError::Connection(e.into())
                        })
                        .and_then(move |conn| {
                            el_products
                                .into_iter()
                                .map(|el_product| {
                                    let base_products_repo = repo_factory.create_base_product_repo(&*conn, user_id);
                                    let products_repo = repo_factory.create_product_repo(&*conn, user_id);
                                    let attr_prod_repo = repo_factory.create_product_attrs_repo(&*conn, user_id);
                                    base_products_repo
                                        .find(el_product.id)
                                        .and_then(move |base_product| {
                                            products_repo
                                                .find_with_base_id(base_product.id)
                                                .map(|products| (base_product, products))
                                                .and_then(move |(base_product, products)| {
                                                    products
                                                        .into_iter()
                                                        .map(|product| {
                                                            attr_prod_repo
                                                                .find_all_attributes(product.id)
                                                                .map(|attrs| {
                                                                    attrs
                                                                        .into_iter()
                                                                        .map(|attr| attr.into())
                                                                        .collect::<Vec<AttrValue>>()
                                                                })
                                                                .map(|attrs| VariantsWithAttributes::new(product, attrs))
                                                        })
                                                        .collect::<RepoResult<Vec<VariantsWithAttributes>>>()
                                                        .and_then(|var| Ok(BaseProductWithVariants::new(base_product, var)))
                                                })
                                        })
                                        .map_err(ServiceError::from)
                                })
                                .collect()
                        })
                })
            }
        }))
    }

    /// Find product by views limited by `count` and `offset` parameters
    fn search_most_viewed(&self, prod: MostViewedProducts, count: i64, offset: i64) -> ServiceFuture<Vec<BaseProductWithVariants>> {
        let products = {
            let client_handle = self.client_handle.clone();
            let address = self.elastic_address.clone();
            let products_el = ProductsElasticImpl::new(client_handle, address);
            products_el
                .search_most_viewed(prod, count, offset)
                .map_err(ServiceError::from)
        };

        Box::new(products.and_then({
            let cpu_pool = self.cpu_pool.clone();
            let db_pool = self.db_pool.clone();
            let user_id = self.user_id;
            let repo_factory = self.repo_factory.clone();
            move |el_products| {
                cpu_pool.spawn_fn(move || {
                    db_pool
                        .get()
                        .map_err(|e| {
                            error!(
                                "Could not get connection to db from pool! {}",
                                e.to_string()
                            );
                            ServiceError::Connection(e.into())
                        })
                        .and_then(move |conn| {
                            el_products
                                .into_iter()
                                .map(|el_product| {
                                    let base_products_repo = repo_factory.create_base_product_repo(&*conn, user_id);
                                    let products_repo = repo_factory.create_product_repo(&*conn, user_id);
                                    let attr_prod_repo = repo_factory.create_product_attrs_repo(&*conn, user_id);
                                    base_products_repo
                                        .find(el_product.id)
                                        .and_then(move |base_product| {
                                            products_repo
                                                .find_with_base_id(base_product.id)
                                                .and_then(move |products| {
                                                    products
                                                        .into_iter()
                                                        .nth(0)
                                                        .ok_or(RepoError::NotFound)
                                                        .and_then(|prod| {
                                                            attr_prod_repo
                                                                .find_all_attributes(prod.id)
                                                                .map(|attrs| {
                                                                    attrs
                                                                        .into_iter()
                                                                        .map(|attr| attr.into())
                                                                        .collect::<Vec<AttrValue>>()
                                                                })
                                                                .map(|attrs| VariantsWithAttributes::new(prod, attrs))
                                                        })
                                                })
                                                .and_then(|var| Ok(BaseProductWithVariants::new(base_product, vec![var])))
                                        })
                                        .map_err(ServiceError::from)
                                })
                                .collect()
                        })
                })
            }
        }))
    }

    /// Find product by dicount pattern limited by `count` and `offset` parameters
    fn search_most_discount(&self, prod: MostDiscountProducts, count: i64, offset: i64) -> ServiceFuture<Vec<BaseProductWithVariants>> {
        let products = {
            let client_handle = self.client_handle.clone();
            let address = self.elastic_address.clone();
            let products_el = ProductsElasticImpl::new(client_handle, address);
            products_el
                .search_most_discount(prod, count, offset)
                .map_err(ServiceError::from)
        };

        Box::new(products.and_then({
            let cpu_pool = self.cpu_pool.clone();
            let db_pool = self.db_pool.clone();
            let user_id = self.user_id;
            let repo_factory = self.repo_factory.clone();
            move |el_products| {
                cpu_pool.spawn_fn(move || {
                    db_pool
                        .get()
                        .map_err(|e| {
                            error!(
                                "Could not get connection to db from pool! {}",
                                e.to_string()
                            );
                            ServiceError::Connection(e.into())
                        })
                        .and_then(move |conn| {
                            el_products
                                .into_iter()
                                .map(|el_product| {
                                    let base_products_repo = repo_factory.create_base_product_repo(&*conn, user_id);
                                    let products_repo = repo_factory.create_product_repo(&*conn, user_id);
                                    let attr_prod_repo = repo_factory.create_product_attrs_repo(&*conn, user_id);
                                    base_products_repo
                                        .find(el_product.id)
                                        .and_then(move |base_product| {
                                            products_repo
                                                .find_with_base_id(base_product.id)
                                                .and_then(move |products| {
                                                    products
                                                        .into_iter()
                                                        .filter_map(|p| {
                                                            if let Some(_) = p.discount {
                                                                Some(p)
                                                            } else {
                                                                None
                                                            }
                                                        })
                                                        .max_by_key(|p| (p.discount.unwrap() * 1000f64).round() as i64)
                                                        .ok_or(RepoError::NotFound)
                                                        .and_then(|prod| {
                                                            attr_prod_repo
                                                                .find_all_attributes(prod.id)
                                                                .map(|attrs| {
                                                                    attrs
                                                                        .into_iter()
                                                                        .map(|attr| attr.into())
                                                                        .collect::<Vec<AttrValue>>()
                                                                })
                                                                .map(|attrs| VariantsWithAttributes::new(prod, attrs))
                                                        })
                                                })
                                                .and_then(|var| Ok(BaseProductWithVariants::new(base_product, vec![var])))
                                        })
                                        .map_err(ServiceError::from)
                                })
                                .collect()
                        })
                })
            }
        }))
    }

    fn auto_complete(&self, name: String, count: i64, offset: i64) -> ServiceFuture<Vec<String>> {
        let client_handle = self.client_handle.clone();
        let address = self.elastic_address.clone();
        let products_names = {
            let products_el = ProductsElasticImpl::new(client_handle, address);
            products_el
                .auto_complete(name, count, offset)
                .map_err(ServiceError::from)
        };

        Box::new(products_names)
    }

    fn search_filters(&self, name: String) -> ServiceFuture<SearchOptions> {
        let client_handle = self.client_handle.clone();
        let address = self.elastic_address.clone();
        let mut search_prod = SearchProductsByName::default();
        search_prod.name = name;
        let search_filters = {
            let products_el = ProductsElasticImpl::new(client_handle, address);
            products_el
                .search_by_name(search_prod, MAX_PRODUCTS_SEARCH_COUNT, 0)
                .map_err(ServiceError::from)
        };

        Box::new(search_filters.and_then({
            move |el_products| {
                let mut cats = HashSet::<i32>::default();
                let mut equal_attrs = HashMap::<i32, HashSet<String>>::default();
                let mut range_attrs = HashMap::<i32, RangeFilter>::default();
                let mut price_filters = RangeFilter::default();

                for product in el_products.into_iter() {
                    cats.insert(product.category_id);
                    for variant in product.variants.into_iter() {
                        price_filters.add_value(variant.price);
                        for attr_value in variant.attrs.into_iter() {
                            if let Some(value) = attr_value.str_val {
                                let hash_with_values = equal_attrs
                                    .entry(attr_value.attr_id)
                                    .or_insert(HashSet::<String>::default());
                                hash_with_values.insert(value);
                            }
                            if let Some(value) = attr_value.float_val {
                                let range = range_attrs
                                    .entry(attr_value.attr_id)
                                    .or_insert(RangeFilter::default());
                                range.add_value(value);
                            }
                        }
                    }
                }

                let eq_filters = equal_attrs.into_iter().map(|(k, v)| AttributeFilter {
                    id: k,
                    equal: Some(EqualFilter {
                        values: v.iter().map(|s| s.clone()).collect(),
                    }),
                    range: None,
                });

                let range_filters = range_attrs.into_iter().map(|(k, v)| AttributeFilter {
                    id: k,
                    equal: None,
                    range: Some(v),
                });

                Ok(SearchOptions {
                    categories_ids: cats.into_iter().collect(),
                    attr_filters: eq_filters.chain(range_filters).collect(),
                    price_filter: Some(price_filters),
                })
            }
        }))
    }

    /// Returns product by ID
    fn get(&self, product_id: i32) -> ServiceFuture<BaseProduct> {
        let db_pool = self.db_pool.clone();
        let user_id = self.user_id;
        let repo_factory = self.repo_factory.clone();

        Box::new(self.cpu_pool.spawn_fn(move || {
            db_pool
                .get()
                .map_err(|e| {
                    error!(
                        "Could not get connection to db from pool! {}",
                        e.to_string()
                    );
                    ServiceError::Connection(e.into())
                })
                .and_then(move |conn| {
                    let base_products_repo = repo_factory.create_base_product_repo(&*conn, user_id);
                    base_products_repo
                        .find(product_id)
                        .map_err(ServiceError::from)
                })
        }))
    }

    /// Returns product by ID
    fn get_with_variants(&self, base_product_id: i32) -> ServiceFuture<BaseProductWithVariants> {
        let db_pool = self.db_pool.clone();
        let user_id = self.user_id;
        let repo_factory = self.repo_factory.clone();

        Box::new(self.cpu_pool.spawn_fn(move || {
            db_pool
                .get()
                .map_err(|e| {
                    error!(
                        "Could not get connection to db from pool! {}",
                        e.to_string()
                    );
                    ServiceError::Connection(e.into())
                })
                .and_then(move |conn| {
                    let base_products_repo = repo_factory.create_base_product_repo(&*conn, user_id);
                    let products_repo = repo_factory.create_product_repo(&*conn, user_id);
                    let attr_prod_repo = repo_factory.create_product_attrs_repo(&*conn, user_id);
                    base_products_repo
                        .find(base_product_id)
                        .and_then(move |base_product| {
                            products_repo
                                .find_with_base_id(base_product.id)
                                .map(|products| (base_product, products))
                                .and_then(move |(base_product, products)| {
                                    products
                                        .into_iter()
                                        .map(|product| {
                                            attr_prod_repo
                                                .find_all_attributes(product.id)
                                                .map(|attrs| {
                                                    attrs
                                                        .into_iter()
                                                        .map(|attr| attr.into())
                                                        .collect::<Vec<AttrValue>>()
                                                })
                                                .map(|attrs| VariantsWithAttributes::new(product, attrs))
                                        })
                                        .collect::<RepoResult<Vec<VariantsWithAttributes>>>()
                                        .and_then(|var| Ok(BaseProductWithVariants::new(base_product, var)))
                                })
                        })
                        .map_err(ServiceError::from)
                })
        }))
    }

    /// Deactivates specific base product
    fn deactivate(&self, product_id: i32) -> ServiceFuture<BaseProduct> {
        let db_pool = self.db_pool.clone();
        let user_id = self.user_id;
        let repo_factory = self.repo_factory.clone();

        Box::new(self.cpu_pool.spawn_fn(move || {
            db_pool
                .get()
                .map_err(|e| {
                    error!(
                        "Could not get connection to db from pool! {}",
                        e.to_string()
                    );
                    ServiceError::Connection(e.into())
                })
                .and_then(move |conn| {
                    let base_products_repo = repo_factory.create_base_product_repo(&*conn, user_id);
                    base_products_repo
                        .deactivate(product_id)
                        .map_err(ServiceError::from)
                })
        }))
    }

    /// Lists base products limited by `from` and `count` parameters
    fn list(&self, from: i32, count: i64) -> ServiceFuture<Vec<BaseProduct>> {
        let db_pool = self.db_pool.clone();
        let user_id = self.user_id;
        let repo_factory = self.repo_factory.clone();

        Box::new(self.cpu_pool.spawn_fn(move || {
            db_pool
                .get()
                .map_err(|e| {
                    error!(
                        "Could not get connection to db from pool! {}",
                        e.to_string()
                    );
                    ServiceError::Connection(e.into())
                })
                .and_then(move |conn| {
                    let base_products_repo = repo_factory.create_base_product_repo(&*conn, user_id);
                    base_products_repo
                        .list(from, count)
                        .map_err(ServiceError::from)
                })
        }))
    }

    /// Returns list of base_products by store id and exclude base_product_id_arg, limited by 10
    fn list_with_variants(&self, store_id: i32, base_product_id: i32) -> ServiceFuture<Vec<BaseProductWithVariants>> {
        let db_pool = self.db_pool.clone();
        let user_id = self.user_id;
        let repo_factory = self.repo_factory.clone();

        Box::new(self.cpu_pool.spawn_fn(move || {
            db_pool
                .get()
                .map_err(|e| {
                    error!(
                        "Could not get connection to db from pool! {}",
                        e.to_string()
                    );
                    ServiceError::Connection(e.into())
                })
                .and_then(move |conn| {
                    let base_products_repo = repo_factory.create_base_product_repo(&*conn, user_id);
                    let products_repo = repo_factory.create_product_repo(&*conn, user_id);
                    let attr_prod_repo = repo_factory.create_product_attrs_repo(&*conn, user_id);
                    base_products_repo
                        .list_by_store(store_id, base_product_id)
                        .and_then(move |base_products| {
                            base_products
                                .into_iter()
                                .map(|base_product| {
                                    products_repo
                                        .find_with_base_id(base_product.id)
                                        .map(|products| (base_product, products))
                                        .and_then(|(base_product, products)| {
                                            products
                                                .into_iter()
                                                .map(|product| {
                                                    attr_prod_repo
                                                        .find_all_attributes(product.id)
                                                        .map(|attrs| {
                                                            attrs
                                                                .into_iter()
                                                                .map(|attr| attr.into())
                                                                .collect::<Vec<AttrValue>>()
                                                        })
                                                        .map(|attrs| VariantsWithAttributes::new(product, attrs))
                                                })
                                                .collect::<RepoResult<Vec<VariantsWithAttributes>>>()
                                                .and_then(|var| Ok(BaseProductWithVariants::new(base_product, var)))
                                        })
                                })
                                .collect::<RepoResult<Vec<BaseProductWithVariants>>>()
                        })
                        .map_err(ServiceError::from)
                })
        }))
    }

    /// Creates new product
    fn create(&self, payload: NewBaseProduct) -> ServiceFuture<BaseProduct> {
        let db_pool = self.db_pool.clone();
        let user_id = self.user_id;
        let cpu_pool = self.cpu_pool.clone();
        let repo_factory = self.repo_factory.clone();
        Box::new(cpu_pool.spawn_fn(move || {
            db_pool
                .get()
                .map_err(|e| {
                    error!(
                        "Could not get connection to db from pool! {}",
                        e.to_string()
                    );
                    ServiceError::Connection(e.into())
                })
                .and_then(move |conn| {
                    let base_products_repo = repo_factory.create_base_product_repo(&*conn, user_id);
                    conn.transaction::<(BaseProduct), ServiceError, _>(move || {
                        base_products_repo
                            .create(payload)
                            .map_err(ServiceError::from)
                    })
                })
        }))
    }

    /// Updates specific product
    fn update(&self, product_id: i32, payload: UpdateBaseProduct) -> ServiceFuture<BaseProduct> {
        let db_pool = self.db_pool.clone();
        let user_id = self.user_id;
        let cpu_pool = self.cpu_pool.clone();
        let repo_factory = self.repo_factory.clone();

        Box::new(cpu_pool.spawn_fn(move || {
            db_pool
                .get()
                .map_err(|e| {
                    error!(
                        "Could not get connection to db from pool! {}",
                        e.to_string()
                    );
                    ServiceError::Connection(e.into())
                })
                .and_then(move |conn| {
                    let base_products_repo = repo_factory.create_base_product_repo(&*conn, user_id);
                    conn.transaction::<(BaseProduct), ServiceError, _>(move || {
                        base_products_repo
                            .update(product_id, payload)
                            .map_err(ServiceError::from)
                    })
                })
        }))
    }
}

#[cfg(test)]
pub mod tests {
    use std::sync::Arc;

    use futures_cpupool::CpuPool;
    use tokio_core::reactor::Handle;
    use tokio_core::reactor::Core;
    use r2d2;
    use serde_json;

    use stq_http::client::Config as HttpConfig;
    use stq_http;

    use repos::repo_factory::tests::*;
    use services::*;
    use models::*;
    use config::Config;

    #[allow(unused)]
    fn create_base_product_service(
        user_id: Option<i32>,
        handle: Arc<Handle>,
    ) -> BaseProductsServiceImpl<MockConnection, MockConnectionManager, ReposFactoryMock> {
        let manager = MockConnectionManager::default();
        let db_pool = r2d2::Pool::builder()
            .build(manager)
            .expect("Failed to create connection pool");
        let cpu_pool = CpuPool::new(1);

        let config = Config::new().unwrap();
        let http_config = HttpConfig {
            http_client_retries: config.client.http_client_retries,
            http_client_buffer_size: config.client.http_client_buffer_size,
        };
        let client = stq_http::client::Client::new(&http_config, &handle);
        let client_handle = client.handle();

        BaseProductsServiceImpl {
            db_pool: db_pool,
            cpu_pool: cpu_pool,
            user_id: user_id,
            client_handle: client_handle,
            elastic_address: "".to_string(),
            repo_factory: MOCK_REPO_FACTORY,
        }
    }

    pub fn create_new_base_product(name: &str) -> NewBaseProduct {
        NewBaseProduct {
            name: serde_json::from_str(name).unwrap(),
            store_id: 1,
            short_description: serde_json::from_str("{}").unwrap(),
            long_description: None,
            seo_title: None,
            seo_description: None,
            currency_id: 1,
            category_id: 1,
        }
    }

    pub fn create_update_base_product(name: &str) -> UpdateBaseProduct {
        UpdateBaseProduct {
            name: Some(serde_json::from_str(name).unwrap()),
            short_description: Some(serde_json::from_str("{}").unwrap()),
            long_description: None,
            seo_title: None,
            seo_description: None,
            currency_id: Some(1),
            category_id: Some(1),
        }
    }

    #[test]
    fn test_get_base_product() {
        let mut core = Core::new().unwrap();
        let handle = Arc::new(core.handle());
        let service = create_base_product_service(Some(MOCK_USER_ID), handle);
        let work = service.get(1);
        let result = core.run(work).unwrap();
        assert_eq!(result.id, 1);
    }

    #[test]
    fn test_list() {
        let mut core = Core::new().unwrap();
        let handle = Arc::new(core.handle());
        let service = create_base_product_service(Some(MOCK_USER_ID), handle);
        let work = service.list(1, 5);
        let result = core.run(work).unwrap();
        assert_eq!(result.len(), 5);
    }

    #[test]
    fn test_create_base_product() {
        let mut core = Core::new().unwrap();
        let handle = Arc::new(core.handle());
        let service = create_base_product_service(Some(MOCK_USER_ID), handle);
        let new_base_product = create_new_base_product(MOCK_BASE_PRODUCT_NAME_JSON);
        let work = service.create(new_base_product);
        let result = core.run(work).unwrap();
        assert_eq!(result.id, MOCK_BASE_PRODUCT_ID);
    }

    #[test]
    fn test_update() {
        let mut core = Core::new().unwrap();
        let handle = Arc::new(core.handle());
        let service = create_base_product_service(Some(MOCK_USER_ID), handle);
        let new_base_product = create_update_base_product(MOCK_BASE_PRODUCT_NAME_JSON);
        let work = service.update(1, new_base_product);
        let result = core.run(work).unwrap();
        assert_eq!(result.id, 1);
        assert_eq!(result.id, MOCK_BASE_PRODUCT_ID);
    }

    #[test]
    fn test_deactivate() {
        let mut core = Core::new().unwrap();
        let handle = Arc::new(core.handle());
        let service = create_base_product_service(Some(MOCK_USER_ID), handle);
        let work = service.deactivate(1);
        let result = core.run(work).unwrap();
        assert_eq!(result.id, 1);
        assert_eq!(result.is_active, false);
    }

}
