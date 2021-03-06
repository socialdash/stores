use std::str::FromStr;

use hyper::header::{Authorization, ContentLength, ContentType};
use hyper::Uri;
use hyper::{Method, Request};

use stq_http::request_util::read_body;
use stq_http::request_util::Currency as CurrencyHeader;
use stq_static_resources::*;
use stq_types::*;

use futures::Future;
use rand::Rng;

use common::*;
use stores_lib::models::*;

pub fn create_new_base_product(name: &str, short_description: &str) -> NewBaseProduct {
    NewBaseProduct {
        name: serde_json::from_str(name).unwrap(),
        store_id: StoreId(1),
        short_description: serde_json::from_str(short_description).unwrap(),
        long_description: None,
        seo_title: None,
        seo_description: None,
        currency: Currency::STQ,
        category_id: CategoryId(12),
        slug: Some(rand::thread_rng().gen_ascii_chars().take(10).collect::<String>().to_lowercase()),
        uuid: uuid::Uuid::new_v4(),
        length_cm: Some(60),
        width_cm: Some(40),
        height_cm: Some(20),
        weight_g: Some(100),
        store_status: Some(ModerationStatus::Moderation),
    }
}

pub fn create_update_base_product(name: &str, short_description: &str) -> UpdateBaseProduct {
    UpdateBaseProduct {
        name: Some(serde_json::from_str(name).unwrap()),
        short_description: Some(serde_json::from_str(short_description).unwrap()),
        long_description: None,
        seo_title: None,
        seo_description: None,
        currency: Some(Currency::STQ),
        category_id: Some(CategoryId(12)),
        slug: None,
        length_cm: Some(60),
        width_cm: Some(40),
        height_cm: Some(20),
        weight_g: Some(100),
    }
}

static MOCK_BASE_PRODUCT_NAME_JSON: &'static str = r##"[{"lang": "en","text": "Base Product"}]"##;
static MOCK_SHORT_DESCRIPTION_JSON: &'static str = r##"[{"lang": "en","text": "Short Description"}]"##;

#[ignore]
#[test]
fn base_products_crud() {
    let mut context = setup();

    //create
    let mut url = Uri::from_str(&format!("{}/base_products", context.base_url)).unwrap();

    let new_base_product = create_new_base_product(MOCK_BASE_PRODUCT_NAME_JSON, MOCK_SHORT_DESCRIPTION_JSON);
    let mut body: String = serde_json::to_string(&new_base_product).unwrap().to_string();

    let mut req = Request::new(Method::Post, url.clone());
    req.headers_mut().set(ContentType::json());
    req.headers_mut().set(ContentLength(body.len() as u64));
    req.headers_mut().set(Authorization("1".to_string()));
    req.headers_mut().set(CurrencyHeader("STQ".to_string()));
    req.set_body(body);

    let mut code = context
        .core
        .run(context.client.request(req).and_then(|res| read_body(res.body())))
        .unwrap();
    let value = serde_json::from_str::<BaseProduct>(&code);
    assert!(value.is_ok(), format!("{:?}", value));

    let id = value.unwrap().id;

    //read
    url = Uri::from_str(&format!("{}/base_products/{}", context.base_url, id)).unwrap();

    req = Request::new(Method::Get, url.clone());
    code = context
        .core
        .run(context.client.request(req).and_then(|res| read_body(res.body())))
        .unwrap();
    let value = serde_json::from_str::<BaseProduct>(&code);
    assert!(value.is_ok(), format!("{:?}", value));

    //update
    url = Uri::from_str(&format!("{}/base_products/{}", context.base_url, id)).unwrap();

    let update_base_product = create_update_base_product(MOCK_BASE_PRODUCT_NAME_JSON, MOCK_SHORT_DESCRIPTION_JSON);
    body = serde_json::to_string(&update_base_product).unwrap().to_string();

    req = Request::new(Method::Put, url.clone());
    req.headers_mut().set(ContentType::json());
    req.headers_mut().set(ContentLength(body.len() as u64));
    req.headers_mut().set(Authorization("1".to_string()));
    req.set_body(body);

    code = context
        .core
        .run(context.client.request(req).and_then(|res| read_body(res.body())))
        .unwrap();
    let value = serde_json::from_str::<BaseProduct>(&code);
    assert!(value.is_ok(), format!("{:?}", value));
    verify_base_product_updated_values(value.expect("Base product update failed"));

    //delete
    url = Uri::from_str(&format!("{}/base_products/{}", context.base_url, id)).unwrap();

    req = Request::new(Method::Delete, url.clone());
    req.headers_mut().set(Authorization("1".to_string()));
    code = context
        .core
        .run(context.client.request(req).and_then(|res| read_body(res.body())))
        .unwrap();
    let value = serde_json::from_str::<BaseProduct>(&code);
    assert!(value.is_ok(), format!("{:?}", value));
}

fn verify_base_product_updated_values(base_product: BaseProduct) {
    assert_eq!(base_product.store_status, ModerationStatus::Published);
}
