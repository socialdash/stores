//! Products Services, presents CRUD operations with product
use std::collections::HashMap;

use diesel::connection::AnsiTransactionManager;
use diesel::pg::Pg;
use diesel::Connection;
use failure::Error as FailureError;
use r2d2::ManageConnection;

use stq_static_resources::{Currency, ModerationStatus};
use stq_types::{AttributeId, AttributeValue, BaseProductId, ExchangeRate, ProductId, ProductSellerPrice, StoreId};

use super::types::ServiceFuture;
use errors::Error;
use models::*;
use repos::{AttributesRepo, CustomAttributesRepo, ProductAttrsRepo, ReposFactory, StoresRepo};
use services::Service;

pub trait ProductsService {
    /// Returns product by ID
    fn get_product(&self, product_id: ProductId) -> ServiceFuture<Option<ProductWithCurrency>>;
    /// Returns product seller price by ID
    fn get_product_seller_price(&self, product_id: ProductId) -> ServiceFuture<Option<ProductSellerPrice>>;
    /// Returns store_id by ID
    fn get_product_store_id(&self, product_id: ProductId) -> ServiceFuture<Option<StoreId>>;
    /// Deactivates specific product
    fn deactivate_product(&self, product_id: ProductId) -> ServiceFuture<ProductWithCurrency>;
    /// Creates base product
    fn create_product(&self, payload: NewProductWithAttributes) -> ServiceFuture<ProductWithCurrency>;
    /// Lists product variants limited by `from` and `count` parameters
    fn list_products(&self, from: i32, count: i32) -> ServiceFuture<Vec<ProductWithCurrency>>;
    /// Updates  product
    fn update_product(&self, product_id: ProductId, payload: UpdateProductWithAttributes) -> ServiceFuture<ProductWithCurrency>;
    /// Get by base product id
    fn find_products_with_base_id(&self, base_product_id: BaseProductId) -> ServiceFuture<Vec<ProductWithCurrency>>;
    /// Get by base product id
    fn find_products_attributes(&self, product_id: ProductId) -> ServiceFuture<Vec<AttrValue>>;
}

impl<
        T: Connection<Backend = Pg, TransactionManager = AnsiTransactionManager> + 'static,
        M: ManageConnection<Connection = T>,
        F: ReposFactory<T>,
    > ProductsService for Service<T, M, F>
{
    /// Returns product by ID
    fn get_product(&self, product_id: ProductId) -> ServiceFuture<Option<ProductWithCurrency>> {
        let user_id = self.dynamic_context.user_id;
        let repo_factory = self.static_context.repo_factory.clone();
        let currency = self.dynamic_context.currency;

        self.spawn_on_pool(move |conn| {
            {
                let products_repo = repo_factory.create_product_repo(&*conn, user_id);
                let currency_exchange = repo_factory.create_currency_exchange_repo(&*conn, user_id);
                let product = products_repo.find(product_id)?.map(ProductWithCurrency::from);
                if let Some(mut product) = product {
                    let currencies_map = currency_exchange.get_exchange_for_currency(currency)?;
                    recalc_currencies(&mut product, &currencies_map, currency);
                    Ok(Some(product))
                } else {
                    Ok(None)
                }
            }.map_err(|e: FailureError| e.context("Service Product, get endpoint error occurred.").into())
        })
    }

    /// Returns product seller price by ID
    fn get_product_seller_price(&self, product_id: ProductId) -> ServiceFuture<Option<ProductSellerPrice>> {
        let user_id = self.dynamic_context.user_id;
        let repo_factory = self.static_context.repo_factory.clone();

        self.spawn_on_pool(move |conn| {
            {
                let products_repo = repo_factory.create_product_repo(&*conn, user_id);
                let product = products_repo.find(product_id)?;
                if let Some(product) = product {
                    Ok(Some(ProductSellerPrice {
                        price: product.price,
                        currency: product.currency,
                    }))
                } else {
                    Ok(None)
                }
            }.map_err(|e: FailureError| e.context("Service Product, get endpoint error occurred.").into())
        })
    }

    /// Returns store_id by ID
    fn get_product_store_id(&self, product_id: ProductId) -> ServiceFuture<Option<StoreId>> {
        let user_id = self.dynamic_context.user_id;
        let repo_factory = self.static_context.repo_factory.clone();

        self.spawn_on_pool(move |conn| {
            {
                let products_repo = repo_factory.create_product_repo(&*conn, user_id);
                let base_products_repo = repo_factory.create_base_product_repo(&*conn, user_id);
                let product = products_repo.find(product_id)?;
                if let Some(product) = product {
                    let base_product = base_products_repo.find(product.base_product_id)?;
                    if let Some(base_product) = base_product {
                        Ok(Some(base_product.store_id))
                    } else {
                        Ok(None)
                    }
                } else {
                    Ok(None)
                }
            }.map_err(|e: FailureError| e.context("Service Product, get_store_id endpoint error occurred.").into())
        })
    }

    /// Deactivates specific product
    fn deactivate_product(&self, product_id: ProductId) -> ServiceFuture<ProductWithCurrency> {
        let user_id = self.dynamic_context.user_id;
        let repo_factory = self.static_context.repo_factory.clone();

        self.spawn_on_pool(move |conn| {
            let products_repo = repo_factory.create_product_repo(&*conn, user_id);
            let prod_attr_repo = repo_factory.create_product_attrs_repo(&*conn, user_id);
            conn.transaction::<ProductWithCurrency, FailureError, _>(move || {
                let result_product: ProductWithCurrency = products_repo.deactivate(product_id)?.into();
                prod_attr_repo.delete_all_attributes(result_product.product.id)?;
                Ok(result_product)
            }).map_err(|e| e.context("Service Product, deactivate endpoint error occurred.").into())
        })
    }

    /// Lists users limited by `from` and `count` parameters
    fn list_products(&self, from: i32, count: i32) -> ServiceFuture<Vec<ProductWithCurrency>> {
        let user_id = self.dynamic_context.user_id;
        let repo_factory = self.static_context.repo_factory.clone();
        let currency = self.dynamic_context.currency;

        self.spawn_on_pool(move |conn| {
            {
                let products_repo = repo_factory.create_product_repo(&*conn, user_id);
                let currency_exchange = repo_factory.create_currency_exchange_repo(&*conn, user_id);
                let mut products = products_repo
                    .list(from, count)?
                    .into_iter()
                    .map(ProductWithCurrency::from)
                    .collect::<Vec<ProductWithCurrency>>();
                let currencies_map = currency_exchange.get_exchange_for_currency(currency)?;
                products
                    .iter_mut()
                    .for_each(|mut product| recalc_currencies(&mut product, &currencies_map, currency));
                Ok(products)
            }.map_err(|e: FailureError| e.context("Service Product, list endpoint error occurred.").into())
        })
    }

    /// Creates new product
    fn create_product(&self, payload: NewProductWithAttributes) -> ServiceFuture<ProductWithCurrency> {
        let user_id = self.dynamic_context.user_id;
        let repo_factory = self.static_context.repo_factory.clone();

        self.spawn_on_pool(move |conn| {
            let base_products_repo = repo_factory.create_base_product_repo(&*conn, user_id);
            let products_repo = repo_factory.create_product_repo(&*conn, user_id);
            let prod_attr_repo = repo_factory.create_product_attrs_repo(&*conn, user_id);
            let attr_repo = repo_factory.create_attributes_repo(&*conn, user_id);
            let custom_attributes_repo = repo_factory.create_custom_attributes_repo(&*conn, user_id);
            let stores_repo = repo_factory.create_stores_repo(&*conn, user_id);

            let NewProductWithAttributes { mut product, attributes } = payload;

            conn.transaction::<ProductWithCurrency, FailureError, _>(move || {
                // fill currency id taken from base_product first
                let base_product_id = product
                    .base_product_id
                    .ok_or(format_err!("Base product id not set.").context(Error::NotFound))?;

                let base_product = base_products_repo.find(base_product_id)?;
                let base_product =
                    base_product.ok_or(format_err!("Base product with id {} not found.", base_product_id).context(Error::NotFound))?;

                product.base_product_id = Some(base_product_id);

                check_vendor_code(&*stores_repo, base_product.store_id, &product.vendor_code)?;

                let result_product: ProductWithCurrency = products_repo.create((product, base_product.currency).into())?.into();

                create_product_attributes_values(
                    &*prod_attr_repo,
                    &*attr_repo,
                    &*custom_attributes_repo,
                    &result_product.product,
                    base_product.id,
                    attributes,
                )?;

                Ok(result_product)
            }).map_err(|e| e.context("Service Product, create endpoint error occurred.").into())
        })
    }

    /// Updates specific product
    fn update_product(&self, product_id: ProductId, payload: UpdateProductWithAttributes) -> ServiceFuture<ProductWithCurrency> {
        let user_id = self.dynamic_context.user_id;
        let repo_factory = self.static_context.repo_factory.clone();

        self.spawn_on_pool(move |conn| {
            let base_products_repo = repo_factory.create_base_product_repo(&*conn, user_id);
            let products_repo = repo_factory.create_product_repo(&*conn, user_id);
            let prod_attr_repo = repo_factory.create_product_attrs_repo(&*conn, user_id);
            let attr_repo = repo_factory.create_attributes_repo(&*conn, user_id);
            let custom_attributes_repo = repo_factory.create_custom_attributes_repo(&*conn, user_id);
            let stores_repo = repo_factory.create_stores_repo(&*conn, user_id);

            conn.transaction::<ProductWithCurrency, FailureError, _>(move || {
                let original_product = products_repo
                    .find(product_id)?
                    .ok_or(format_err!("Not found such product id: {}", product_id).context(Error::NotFound))?;

                let product = if let Some(product) = payload.product {
                    if let Some(vendor_code) = &product.vendor_code {
                        let BaseProduct { store_id, .. } = base_products_repo.find(original_product.base_product_id)?.ok_or(
                            format_err!("Base product with id {} not found.", original_product.base_product_id).context(Error::NotFound),
                        )?;

                        if *original_product.vendor_code.as_str() != *vendor_code {
                            check_vendor_code(&*stores_repo, store_id, &vendor_code)?;
                        }
                    };

                    let reset_moderation = product.reset_moderation_status_needed();
                    let updated_product = products_repo.update(product_id, product)?;
                    // reset moderation if needed
                    if reset_moderation {
                        let update_base_product = UpdateBaseProduct::update_status(ModerationStatus::Draft);
                        base_products_repo.update(updated_product.base_product_id, update_base_product)?;
                    }
                    updated_product
                } else {
                    original_product
                };

                let result_product: ProductWithCurrency = product.into();

                if let Some(attributes) = payload.attributes {
                    create_product_attributes_values(
                        &*prod_attr_repo,
                        &*attr_repo,
                        &*custom_attributes_repo,
                        &result_product.product,
                        result_product.product.base_product_id,
                        attributes,
                    )?;
                }

                Ok(result_product)
            }).map_err(|e| e.context("Service Product, update endpoint error occurred.").into())
        })
    }

    /// Get by base product id
    fn find_products_with_base_id(&self, base_product_id: BaseProductId) -> ServiceFuture<Vec<ProductWithCurrency>> {
        let user_id = self.dynamic_context.user_id;
        let repo_factory = self.static_context.repo_factory.clone();
        let currency = self.dynamic_context.currency;

        self.spawn_on_pool(move |conn| {
            {
                let products_repo = repo_factory.create_product_repo(&*conn, user_id);
                let currency_exchange = repo_factory.create_currency_exchange_repo(&*conn, user_id);
                let mut products = products_repo
                    .find_with_base_id(base_product_id)?
                    .into_iter()
                    .map(ProductWithCurrency::from)
                    .collect::<Vec<ProductWithCurrency>>();
                let currencies_map = currency_exchange.get_exchange_for_currency(currency)?;
                products
                    .iter_mut()
                    .for_each(|mut product| recalc_currencies(&mut product, &currencies_map, currency));
                Ok(products)
            }.map_err(|e: FailureError| e.context("Service Product, find_with_base_id endpoint error occurred.").into())
        })
    }

    /// Get by base product id
    fn find_products_attributes(&self, product_id: ProductId) -> ServiceFuture<Vec<AttrValue>> {
        let user_id = self.dynamic_context.user_id;
        let repo_factory = self.static_context.repo_factory.clone();

        self.spawn_on_pool(move |conn| {
            let prod_attr_repo = repo_factory.create_product_attrs_repo(&*conn, user_id);
            prod_attr_repo
                .find_all_attributes(product_id)
                .map(|pr_attrs| pr_attrs.into_iter().map(|pr_attr| pr_attr.into()).collect())
                .map_err(|e| e.context("Service Product, find_attributes endpoint error occurred.").into())
        })
    }
}

fn recalc_currencies(product_arg: &mut ProductWithCurrency, currencies_map: &Option<HashMap<Currency, ExchangeRate>>, currency: Currency) {
    if let Some(currency_map) = currencies_map {
        product_arg.product.price.0 *= currency_map[&product_arg.product.currency].0;
        product_arg.product.currency = currency;
    }
}

pub fn create_product_attributes_values(
    prod_attr_repo: &ProductAttrsRepo,
    attr_repo: &AttributesRepo,
    custom_attributes_repo: &CustomAttributesRepo,
    product_arg: &Product,
    base_product_arg: BaseProductId,
    attributes_values: Vec<AttrValue>,
) -> Result<(), FailureError> {
    // deleting old attributes for this product
    prod_attr_repo.delete_all_attributes(product_arg.id)?;
    // searching for existed product with such attribute values
    let base_attrs = prod_attr_repo.find_all_attributes_by_base(base_product_arg)?;
    // get available attributes
    let available_attributes = custom_attributes_repo
        .find_all_attributes(base_product_arg)?
        .into_iter()
        .map(|v| (v.attribute_id, String::default().into()))
        .collect::<HashMap<AttributeId, AttributeValue>>();

    check_attributes_values_exist(base_attrs, attributes_values.clone(), available_attributes)?;

    for attr_value in attributes_values {
        let attr = attr_repo.find(attr_value.attr_id)?;
        let attr = attr.ok_or(format_err!("Not found such attribute id : {}", attr_value.attr_id).context(Error::NotFound))?;
        let new_prod_attr = NewProdAttr::new(
            product_arg.id,
            base_product_arg,
            attr_value.attr_id,
            attr_value.value,
            attr.value_type,
            attr_value.meta_field,
        );
        prod_attr_repo.create(new_prod_attr)?;
    }

    Ok(())
}

fn check_attributes_values_exist(
    base_attrs: Vec<ProdAttr>,
    attributes: Vec<AttrValue>,
    available_attributes: HashMap<AttributeId, AttributeValue>,
) -> Result<(), FailureError> {
    let mut hash = HashMap::<ProductId, HashMap<AttributeId, AttributeValue>>::default();
    for attr in base_attrs {
        let mut prod_attrs = hash.entry(attr.prod_id).or_insert_with(|| available_attributes.clone());
        prod_attrs.insert(attr.attr_id, attr.value);
    }

    let result = hash.into_iter().any(|(_, prod_attrs)| {
        attributes.iter().all(|attr| {
            if let Some(value) = prod_attrs.get(&attr.attr_id) {
                value == &attr.value
            } else {
                false
            }
        })
    });

    if result {
        Err(format_err!("Product with attributes {:?} already exists", attributes)
            .context(Error::Validate(
                validation_errors!({"attributes": ["attributes" => "Product with this attributes already exists"]}),
            )).into())
    } else {
        Ok(())
    }
}

fn check_vendor_code(stores_repo: &StoresRepo, store_id: StoreId, vendor_code: &str) -> Result<(), FailureError> {
    let vendor_code_exists = stores_repo
        .vendor_code_exists(store_id, vendor_code)?
        .ok_or(format_err!("Store with id {} not found.", store_id).context(Error::NotFound))?;

    if vendor_code_exists {
        Err(
            format_err!("Vendor code '{}' already exists for store with id {}.", vendor_code, store_id)
                .context(Error::Validate(
                    validation_errors!({"vendor_code": ["vendor_code" => "Vendor code already exists."]}),
                )).into(),
        )
    } else {
        Ok(())
    }
}

#[cfg(test)]
pub mod tests {
    use std::sync::Arc;
    use std::time::SystemTime;

    use stq_static_resources::Currency;
    use stq_types::*;

    use tokio_core::reactor::Core;

    use models::*;
    use repos::repo_factory::tests::*;
    use services::*;

    pub fn create_product(id: ProductId, base_product_id: BaseProductId) -> Product {
        Product {
            id,
            base_product_id,
            is_active: true,
            discount: None,
            photo_main: None,
            vendor_code: "vendor_code".to_string(),
            cashback: None,
            additional_photos: None,
            price: ProductPrice(0f64),
            currency: Currency::STQ,
            created_at: SystemTime::now(),
            updated_at: SystemTime::now(),
            pre_order: false,
            pre_order_days: 0,
            kafka_update_no: 0,
        }
    }

    pub fn create_new_product_with_attributes(base_product_id: BaseProductId) -> NewProductWithAttributes {
        NewProductWithAttributes {
            product: create_new_product(base_product_id),
            attributes: vec![AttrValue {
                attr_id: AttributeId(1),
                value: AttributeValue("String".to_string()),
                meta_field: None,
            }],
        }
    }

    pub fn create_new_product(base_product_id: BaseProductId) -> NewProductWithoutCurrency {
        NewProductWithoutCurrency {
            base_product_id: Some(base_product_id),
            discount: None,
            photo_main: None,
            vendor_code: "vendor_code".to_string(),
            cashback: None,
            additional_photos: None,
            price: ProductPrice(0f64),
            pre_order: Some(false),
            pre_order_days: Some(0),
        }
    }

    pub fn create_update_product() -> UpdateProduct {
        UpdateProduct {
            discount: None,
            photo_main: None,
            vendor_code: None,
            cashback: None,
            additional_photos: None,
            price: None,
            currency: None,
            pre_order: None,
            pre_order_days: None,
        }
    }

    pub fn create_update_product_with_attributes() -> UpdateProductWithAttributes {
        UpdateProductWithAttributes {
            product: Some(create_update_product()),
            attributes: None,
        }
    }

    #[test]
    fn test_get_product() {
        let mut core = Core::new().unwrap();
        let handle = Arc::new(core.handle());
        let service = create_service(Some(MOCK_USER_ID), handle);
        let work = service.get_product(ProductId(1));
        let result = core.run(work).unwrap();
        assert_eq!(result.unwrap().product.id, ProductId(1));
    }

    #[test]
    fn test_list() {
        let mut core = Core::new().unwrap();
        let handle = Arc::new(core.handle());
        let service = create_service(Some(MOCK_USER_ID), handle);
        let work = service.list_products(1, 5);
        let result = core.run(work).unwrap();
        assert_eq!(result.len(), 5);
    }

    #[test]
    fn test_create_product() {
        let mut core = Core::new().unwrap();
        let handle = Arc::new(core.handle());
        let service = create_service(Some(MOCK_USER_ID), handle);
        let new_product = create_new_product_with_attributes(MOCK_BASE_PRODUCT_ID);
        let work = service.create_product(new_product);
        let result = core.run(work).unwrap();
        assert_eq!(result.product.base_product_id, MOCK_BASE_PRODUCT_ID);
    }

    #[test]
    fn test_update_product() {
        let mut core = Core::new().unwrap();
        let handle = Arc::new(core.handle());
        let service = create_service(Some(MOCK_USER_ID), handle);
        let new_product = create_update_product_with_attributes();
        let work = service.update_product(ProductId(1), new_product);
        let result = core.run(work).unwrap();
        assert_eq!(result.product.id, ProductId(1));
        assert_eq!(result.product.base_product_id, MOCK_BASE_PRODUCT_ID);
    }

    #[test]
    fn test_deactivate_product() {
        let mut core = Core::new().unwrap();
        let handle = Arc::new(core.handle());
        let service = create_service(Some(MOCK_USER_ID), handle);
        let work = service.deactivate_product(ProductId(1));
        let result = core.run(work).unwrap();
        assert_eq!(result.product.id, ProductId(1));
        assert_eq!(result.product.is_active, false);
    }

}
