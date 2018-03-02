//! Repo for user_roles table. UserRole is an entity that connects
//! users and roles. I.e. this table is for user has-many roles
//! relationship

use diesel;
use diesel::prelude::*;
use diesel::query_dsl::RunQueryDsl;

use models::user_role::user_roles::dsl::*;
use models::{NewUserRole, OldUserRole, UserRole};
use repos::acl::Acl;
use super::error::RepoError as Error;
use super::types::{DbConnection, RepoResult};

/// UserRoles repository for handling UserRoles
pub trait UserRolesRepo {
    /// Returns list of user_roles for a specific user
    fn list_for_user(&self, user_id: i32) -> RepoResult<Vec<UserRole>>;

    /// Create a new user role
    fn create(&self, payload: NewUserRole) -> RepoResult<UserRole>;

    /// Delete role of a user
    fn delete(&self, payload: OldUserRole) -> RepoResult<()>;
}

/// Implementation of UserRoles trait
pub struct UserRolesRepoImpl<'a> {
    pub db_conn: &'a DbConnection,
    pub acl: &'a Acl,
}

impl<'a> UserRolesRepoImpl<'a> {
    pub fn new(db_conn: &'a DbConnection, acl: &'a Acl) -> Self {
        Self { db_conn, acl }
    }
}

impl<'a> UserRolesRepo for UserRolesRepoImpl<'a> {
    fn list_for_user(&self, user_id_value: i32) -> RepoResult<Vec<UserRole>> {
        let query = user_roles.filter(user_id.eq(user_id_value));
        query.get_results(&**self.db_conn).map_err(Error::from)
    }

    fn create(&self, payload: NewUserRole) -> RepoResult<UserRole> {
        let query = diesel::insert_into(user_roles).values(&payload);
        query.get_result(&**self.db_conn).map_err(Error::from)
    }

    fn delete(&self, payload: OldUserRole) -> RepoResult<()> {
        let filtered = user_roles
            .filter(user_id.eq(payload.user_id))
            .filter(role.eq(payload.role));
        let query = diesel::delete(filtered);
        query
            .execute(&**self.db_conn)
            .map_err(Error::from)
            .and_then(|_| Ok(()))
    }
}
