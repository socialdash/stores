use std::convert::From;

use diesel;
use diesel::prelude::*;
use diesel::query_dsl::RunQueryDsl;

use models::{ProdAttr, NewProdAttr, UpdateProdAttr};
use models::attribute_product::prod_attr_values::dsl::*;
use super::error::Error;
use super::types::{DbConnection, RepoResult};
use repos::acl::Acl;
use models::authorization::*;

/// ProductAttrs repository, responsible for handling prod_attr_values
pub struct ProductAttrsRepoImpl<'a> {
    pub db_conn: &'a DbConnection,
    pub acl: Box<Acl>,
}

pub trait ProductAttrsRepo {
    /// Find specific product_attribute by product_id
    fn find(&mut self, product_id: i32) -> RepoResult<Vec<ProdAttr>>;

    /// Creates new product_attribute
    fn create(&mut self, payload: NewProdAttr) -> RepoResult<ProdAttr>;

    /// Updates specific product_attribute
    fn update(&mut self, payload: UpdateProdAttr) -> RepoResult<ProdAttr>;
}

impl<'a> ProductAttrsRepoImpl<'a> {
    pub fn new(db_conn: &'a DbConnection, acl: Box<Acl>) -> Self {
        Self { db_conn, acl }
    }
}

impl<'a> ProductAttrsRepo for ProductAttrsRepoImpl<'a> {
    /// Find specific product_attribute by ID
    fn find(&mut self, product_id_arg: i32) -> RepoResult<Vec<ProdAttr>>{
        let query = prod_attr_values
            .filter(prod_id.eq(product_id_arg))
            .order(id);

        query
            .get_results(&**self.db_conn)
            .map_err(|e| Error::from(e))
            .and_then(|prod_attrs_res: Vec<ProdAttr>| {
                let resources = prod_attrs_res
                    .iter()
                    .map(|prod_attr| (prod_attr as &WithScope))
                    .collect();
                acl!(
                    resources,
                    self.acl,
                    Resource::ProductAttrs,
                    Action::Read,
                    Some(self.db_conn)
                ).and_then(|_| Ok(prod_attrs_res.clone()))
            })
    }

    /// Creates new product_attribute
    fn create(&mut self, payload: NewProdAttr) -> RepoResult<ProdAttr> {
        acl!(
            [payload],
            self.acl,
            Resource::ProductAttrs,
            Action::Create,
            Some(self.db_conn)
        ).and_then(|_| {
            let query_product_attribute = diesel::insert_into(prod_attr_values).values(&payload);
            query_product_attribute
                .get_result::<ProdAttr>(&**self.db_conn)
                .map_err(Error::from)
        })
    }

    /// Updates specific product_attribute
    fn update(&mut self, payload: UpdateProdAttr) -> RepoResult<ProdAttr> {
        let query = prod_attr_values
            .filter(prod_id.eq(payload.prod_id))
            .filter(attr_id.eq(payload.attr_id));
        query
            .first::<ProdAttr>(&**self.db_conn)
            .map_err(|e| Error::from(e))
            .and_then(|prod_attr: ProdAttr| {
                acl!(
                    [prod_attr],
                    self.acl,
                    Resource::ProductAttrs,
                    Action::Update,
                    Some(self.db_conn)
                )
            })
            .and_then(|_| {
                let filter = prod_attr_values
                    .filter(prod_id.eq(payload.prod_id))
                    .filter(attr_id.eq(payload.attr_id));

                let query = diesel::update(filter).set(&payload);
                query
                    .get_result::<ProdAttr>(&**self.db_conn)
                    .map_err(|e| Error::from(e))
            })
    }
}
