use models::product::products::dsl as Products;
use models::Product;
use repos::types::DbConnection;
use diesel::prelude::*;
use serde::ser::{Serialize, SerializeStruct, Serializer};
use stq_acl::WithScope;
use models::Scope;

/// diesel table for product attributes
table! {
    prod_attr_values (id) {
        id -> Integer,
        prod_id -> Integer,
        attr_id -> Integer,
        value -> VarChar,
        value_type -> VarChar,
    }
}

/// Payload for querying product attributes
#[derive(Debug, Deserialize, Associations, Queryable, Clone, Identifiable)]
#[table_name = "prod_attr_values"]
pub struct ProdAttr {
    pub id: i32,
    pub prod_id: i32,
    pub attr_id: i32,
    pub value: String,
    pub value_type: AttributeType,
}

/// Payload for creating product attributes
#[derive(Serialize, Deserialize, Insertable, Clone)]
#[table_name = "prod_attr_values"]
pub struct NewProdAttr {
    pub prod_id: i32,
    pub attr_id: i32,
    pub value: String,
    pub value_type: AttributeType,
}

/// Payload for updating product attributes
#[derive(Serialize, Deserialize, Insertable, AsChangeset)]
#[table_name = "prod_attr_values"]
pub struct UpdateProdAttr {
    pub prod_id: i32,
    pub attr_id: i32,
    pub value: String,
    pub value_type: AttributeType,
}

#[derive(Deserialize, Clone)]
pub struct AttrValue {
    pub name: String,
    pub value: String,
    pub value_type: AttributeType,
}

impl Serialize for AttrValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("AttrValue", 2)?;
        state.serialize_field("name", &self.name)?;
        match self.value_type {
            AttributeType::Float => {
                let f = self.value.parse::<f32>().map_err(|e| e.to_string());
                state.serialize_field("float_val", &f)
            }
            AttributeType::Str => state.serialize_field("str_val", &self.value),
        }?;

        state.end()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "attribute_type")]
pub enum AttributeType {
    Str,
    Float,
}

impl WithScope<Scope> for ProdAttr {
    fn is_in_scope(&self, scope: &Scope, user_id: i32, conn: Option<&DbConnection>) -> bool {
        match *scope {
            Scope::All => true,
            Scope::Owned => {
                if let Some(conn) = conn {
                    Products::products
                        .find(self.prod_id)
                        .get_result::<Product>(&**conn)
                        .and_then(|product: Product| Ok(product.is_in_scope(scope, user_id, Some(conn))))
                        .ok()
                        .unwrap_or(false)
                } else {
                    false
                }
            }
        }
    }
}

impl WithScope<Scope> for NewProdAttr {
    fn is_in_scope(&self, scope: &Scope, user_id: i32, conn: Option<&DbConnection>) -> bool {
        match *scope {
            Scope::All => true,
            Scope::Owned => {
                if let Some(conn) = conn {
                    Products::products
                        .find(self.prod_id)
                        .get_result::<Product>(&**conn)
                        .and_then(|product: Product| Ok(product.is_in_scope(scope, user_id, Some(conn))))
                        .ok()
                        .unwrap_or(false)
                } else {
                    false
                }
            }
        }
    }
}

mod diesel_impl {
    use std::error::Error;
    use std::io::Write;
    use std::str;

    use diesel::pg::Pg;
    use diesel::row::Row;
    use diesel::expression::bound::Bound;
    use diesel::expression::AsExpression;
    use diesel::types::{FromSqlRow, IsNull, NotNull, SingleValue, ToSql};
    use diesel::serialize::Output;
    use diesel::deserialize::Queryable;
    use diesel::sql_types::VarChar;

    use super::AttributeType;

    impl NotNull for AttributeType {}
    impl SingleValue for AttributeType {}

    impl FromSqlRow<VarChar, Pg> for AttributeType {
        fn build_from_row<R: Row<Pg>>(row: &mut R) -> Result<Self, Box<Error + Send + Sync>> {
            match row.take() {
                Some(b"str") => Ok(AttributeType::Str),
                Some(b"float") => Ok(AttributeType::Float),
                Some(value) => Err(format!(
                    "Unrecognized enum variant for AttributeType: {}",
                    str::from_utf8(value).unwrap_or("unreadable value")
                ).into()),
                None => Err("Unexpected null for non-null column `role`".into()),
            }
        }
    }

    impl Queryable<VarChar, Pg> for AttributeType {
        type Row = AttributeType;
        fn build(row: Self::Row) -> Self {
            row
        }
    }

    impl ToSql<VarChar, Pg> for AttributeType {
        fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> Result<IsNull, Box<Error + Send + Sync>> {
            match *self {
                AttributeType::Str => out.write_all(b"str")?,
                AttributeType::Float => out.write_all(b"float")?,
            }
            Ok(IsNull::No)
        }
    }

    impl AsExpression<VarChar> for AttributeType {
        type Expression = Bound<VarChar, AttributeType>;
        fn as_expression(self) -> Self::Expression {
            Bound::new(self)
        }
    }

    impl<'a> AsExpression<VarChar> for &'a AttributeType {
        type Expression = Bound<VarChar, &'a AttributeType>;
        fn as_expression(self) -> Self::Expression {
            Bound::new(self)
        }
    }

}