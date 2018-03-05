use std::convert::From;

use diesel;
use diesel::prelude::*;
use diesel::query_dsl::RunQueryDsl;

use models::{Attribute, NewAttribute, UpdateAttribute};
use models::attribute::attributes::dsl::*;
use repos::error::RepoError as Error;

use super::types::{DbConnection, RepoResult};
use models::authorization::*;
use super::acl;
use super::acl::BoxedAcl;

/// Attributes repository, responsible for handling attribute_values
pub struct AttributesRepoImpl<'a> {
    pub db_conn: &'a DbConnection,
    pub acl: BoxedAcl,
}

pub trait AttributesRepo {
    /// Find specific attribute by name
    fn find(&self, name: String) -> RepoResult<Attribute>;

    /// Creates new attribute
    fn create(&self, payload: NewAttribute) -> RepoResult<Attribute>;

    /// Updates specific attribute
    fn update(&self, attribute_id_arg: i32, payload: UpdateAttribute) -> RepoResult<Attribute>;
}

impl<'a> AttributesRepoImpl<'a> {
    pub fn new(db_conn: &'a DbConnection, acl: BoxedAcl) -> Self {
        Self { db_conn, acl }
    }
}

impl<'a> AttributesRepo for AttributesRepoImpl<'a> {
    /// Find specific attribute by name
    fn find(&self, name_arg: String) -> RepoResult<Attribute> {
        let query = attributes.filter(name.eq(name_arg));

        query
            .first::<Attribute>(&**self.db_conn)
            .map_err(Error::from)
            .and_then(|attribute: Attribute| {
                acl::check(
                    &*self.acl,
                    &Resource::Attributes,
                    &Action::Read,
                    &[],
                    Some(self.db_conn),
                ).and_then(|_| Ok(attribute))
            })
    }

    /// Creates new attribute
    fn create(&self, payload: NewAttribute) -> RepoResult<Attribute> {
        acl::check(
            &*self.acl,
            &Resource::Attributes,
            &Action::Create,
            &[],
            Some(self.db_conn),
        ).and_then(|_| {
            let query_attribute = diesel::insert_into(attributes).values(&payload);
            query_attribute
                .get_result::<Attribute>(&**self.db_conn)
                .map_err(Error::from)
        })
    }

    /// Updates specific attribute
    fn update(&self, attribute_id_arg: i32, payload: UpdateAttribute) -> RepoResult<Attribute> {
        let query = attributes.find(attribute_id_arg);

        query
            .first::<Attribute>(&**self.db_conn)
            .map_err(Error::from)
            .and_then(|_| {
                acl::check(
                    &*self.acl,
                    &Resource::Attributes,
                    &Action::Update,
                    &[],
                    Some(self.db_conn),
                )
            })
            .and_then(|_| {
                let filter = attributes.filter(id.eq(attribute_id_arg));

                let query = diesel::update(filter).set(&payload);
                query
                    .get_result::<Attribute>(&**self.db_conn)
                    .map_err(Error::from)
            })
    }
}