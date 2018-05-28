use std::convert::From;

use diesel;
use diesel::connection::AnsiTransactionManager;
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::query_dsl::RunQueryDsl;
use diesel::Connection;

use stq_acl::*;

use super::acl;
use super::types::RepoResult;
use models::attribute_product::prod_attr_values::dsl::*;
use models::authorization::*;
use models::base_product::base_products::dsl as BaseProducts;
use models::store::stores::dsl as Stores;
use models::{BaseProduct, NewProdAttr, ProdAttr, Store, UpdateProdAttr};
use repos::error::RepoError as Error;

/// ProductAttrs repository, responsible for handling prod_attr_values
pub struct ProductAttrsRepoImpl<'a, T: Connection<Backend = Pg, TransactionManager = AnsiTransactionManager> + 'static> {
    pub db_conn: &'a T,
    pub acl: Box<Acl<Resource, Action, Scope, Error, ProdAttr>>,
}

pub trait ProductAttrsRepo {
    /// Find product attributes by product ID
    fn find_all_attributes(&self, product_id_arg: i32) -> RepoResult<Vec<ProdAttr>>;

    /// Creates new product_attribute
    fn create(&self, payload: NewProdAttr) -> RepoResult<ProdAttr>;

    /// Updates specific product_attribute
    fn update(&self, payload: UpdateProdAttr) -> RepoResult<ProdAttr>;

    /// Delete all attributes values from product
    fn delete_all_attributes(&self, product_id_arg: i32) -> RepoResult<Vec<ProdAttr>>;

    /// Delete all attributes values from product not in the list
    fn delete_all_attributes_not_in_list(&self, product_id_arg: i32, attr_values: Vec<i32>) -> RepoResult<Vec<ProdAttr>>;

    /// Delete attribute value
    fn delete(&self, id_arg: i32) -> RepoResult<ProdAttr>;
}

impl<'a, T: Connection<Backend = Pg, TransactionManager = AnsiTransactionManager> + 'static> ProductAttrsRepoImpl<'a, T> {
    pub fn new(db_conn: &'a T, acl: Box<Acl<Resource, Action, Scope, Error, ProdAttr>>) -> Self {
        Self { db_conn, acl }
    }
}

impl<'a, T: Connection<Backend = Pg, TransactionManager = AnsiTransactionManager> + 'static> ProductAttrsRepo
    for ProductAttrsRepoImpl<'a, T>
{
    /// Find specific product_attributes by product ID
    fn find_all_attributes(&self, product_id_arg: i32) -> RepoResult<Vec<ProdAttr>> {
        debug!("Find all attributes of product id {}.", product_id_arg);
        let query = prod_attr_values.filter(prod_id.eq(product_id_arg)).order(id);

        query
            .get_results(self.db_conn)
            .map_err(Error::from)
            .and_then(|prod_attrs_res: Vec<ProdAttr>| {
                for prod_attr in &prod_attrs_res {
                    acl::check(&*self.acl, &Resource::ProductAttrs, &Action::Read, self, Some(&prod_attr))?;
                }
                Ok(prod_attrs_res.clone())
            })
    }

    /// Creates new product_attribute
    fn create(&self, payload: NewProdAttr) -> RepoResult<ProdAttr> {
        debug!("Create new product attribute {:?}.", payload);
        let query_product_attribute = diesel::insert_into(prod_attr_values).values(&payload);
        query_product_attribute
            .get_result::<ProdAttr>(self.db_conn)
            .map_err(Error::from)
            .and_then(|prod_attr| {
                acl::check(&*self.acl, &Resource::ProductAttrs, &Action::Create, self, Some(&prod_attr)).and_then(|_| Ok(prod_attr))
            })
    }

    fn update(&self, payload: UpdateProdAttr) -> RepoResult<ProdAttr> {
        debug!("Updating product attribute with payload {:?}.", payload);
        let query = prod_attr_values
            .filter(prod_id.eq(payload.prod_id))
            .filter(attr_id.eq(payload.attr_id));

        query
            .first::<ProdAttr>(self.db_conn)
            .map_err(Error::from)
            .and_then(|prod_attr: ProdAttr| acl::check(&*self.acl, &Resource::ProductAttrs, &Action::Update, self, Some(&prod_attr)))
            .and_then(|_| {
                let filter = prod_attr_values
                    .filter(prod_id.eq(payload.prod_id))
                    .filter(attr_id.eq(payload.attr_id));

                let query = diesel::update(filter).set(&payload);
                query.get_result::<ProdAttr>(self.db_conn).map_err(Error::from)
            })
    }

    /// Delete all attributes values from product
    fn delete_all_attributes(&self, product_id_arg: i32) -> RepoResult<Vec<ProdAttr>> {
        debug!("Delete all attributes of product id {}.", product_id_arg);
        let filtered = prod_attr_values.filter(prod_id.eq(product_id_arg));

        let query = diesel::delete(filtered);
        query
            .get_results(self.db_conn)
            .map_err(Error::from)
            .and_then(|prod_attrs_res: Vec<ProdAttr>| {
                for prod_attr in &prod_attrs_res {
                    acl::check(&*self.acl, &Resource::ProductAttrs, &Action::Delete, self, Some(&prod_attr))?;
                }
                Ok(prod_attrs_res.clone())
            })
    }

    /// Delete all attributes values from product not in the list
    fn delete_all_attributes_not_in_list(&self, product_id_arg: i32, attr_values: Vec<i32>) -> RepoResult<Vec<ProdAttr>> {
        debug!(
            "Delete all attributes of product id {} not in the list {:?}.",
            product_id_arg, attr_values
        );
        let filtered = prod_attr_values.filter(prod_id.eq(product_id_arg)).filter(id.ne_all(attr_values));

        let query = diesel::delete(filtered);
        query
            .get_results(self.db_conn)
            .map_err(Error::from)
            .and_then(|prod_attrs_res: Vec<ProdAttr>| {
                for prod_attr in &prod_attrs_res {
                    acl::check(&*self.acl, &Resource::ProductAttrs, &Action::Delete, self, Some(&prod_attr))?;
                }
                Ok(prod_attrs_res.clone())
            })
    }

    /// Delete attribute value
    fn delete(&self, id_arg: i32) -> RepoResult<ProdAttr> {
        debug!("Delete attribute value by id {}.", id_arg);
        let filtered = prod_attr_values.filter(id.eq(id_arg));

        let query = diesel::delete(filtered);
        query.get_result(self.db_conn).map_err(Error::from).and_then(|prod_attr: ProdAttr| {
            acl::check(&*self.acl, &Resource::ProductAttrs, &Action::Delete, self, Some(&prod_attr))?;
            Ok(prod_attr)
        })
    }
}

impl<'a, T: Connection<Backend = Pg, TransactionManager = AnsiTransactionManager> + 'static> CheckScope<Scope, ProdAttr>
    for ProductAttrsRepoImpl<'a, T>
{
    fn is_in_scope(&self, user_id: i32, scope: &Scope, obj: Option<&ProdAttr>) -> bool {
        match *scope {
            Scope::All => true,
            Scope::Owned => {
                if let Some(prod_attr) = obj {
                    BaseProducts::base_products
                        .find(prod_attr.base_prod_id)
                        .get_result::<BaseProduct>(self.db_conn)
                        .and_then(|base_prod: BaseProduct| {
                            Stores::stores
                                .find(base_prod.store_id)
                                .get_result::<Store>(self.db_conn)
                                .and_then(|store: Store| Ok(store.user_id == user_id))
                        })
                        .ok()
                        .unwrap_or(false)
                } else {
                    false
                }
            }
        }
    }
}
