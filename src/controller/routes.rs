use stq_router::RouteParser;
use stq_types::*;

/// List of all routes with params for the app
#[derive(Clone, Debug, PartialEq)]
pub enum Route {
    Healthcheck,
    Attributes,
    Attribute(i32),
    BaseProducts,
    BaseProductWithVariants,
    BaseProductsSearch,
    BaseProductsAutoComplete,
    BaseProductsMostViewed,
    BaseProductsMostDiscount,
    BaseProductsSearchFiltersPrice,
    BaseProductsSearchFiltersCategory,
    BaseProductsSearchFiltersAttributes,
    BaseProductsSearchFiltersCount,
    BaseProduct(BaseProductId),
    BaseProductWithViewsUpdate(BaseProductId),
    BaseProductByProduct(ProductId),
    BaseProductWithVariant(BaseProductId),
    Categories,
    Category(i32),
    CategoryAttrs,
    CategoryAttr(i32),
    CurrencyExchange,
    ModeratorProductComments,
    ModeratorBaseProductComment(BaseProductId),
    ModeratorStoreComments,
    ModeratorStoreComment(StoreId),
    Products,
    ProductStoreId,
    Product(ProductId),
    ProductAttributes(ProductId),
    ProductsByBaseProduct(BaseProductId),
    Stores,
    StoresSearch,
    StoresAutoComplete,
    StoresSearchFiltersCount,
    StoresSearchFiltersCountry,
    StoresSearchFiltersCategory,
    StoresCart,
    Store(StoreId),
    StoreByUser(UserId),
    StoreProducts(StoreId),
    StoreProductsCount(StoreId),
    UserRoles,
    UserRole(UserId),
    DefaultRole(UserId),
    WizardStores,
}

pub fn create_route_parser() -> RouteParser<Route> {
    let mut router = RouteParser::default();

    // Healthcheck
    router.add_route(r"^/healthcheck$", || Route::Healthcheck);

    // Stores Routes
    router.add_route(r"^/stores$", || Route::Stores);

    // Stores/:id route
    router.add_route_with_params(r"^/stores/(\d+)$", |params| {
        params
            .get(0)
            .and_then(|string_id| string_id.parse::<i32>().ok())
            .map(StoreId)
            .map(Route::Store)
    });

    // Stores/by_user_id/:id route
    router.add_route_with_params(r"^/stores/by_user_id/(\d+)$", |params| {
        params
            .get(0)
            .and_then(|string_id| string_id.parse::<i32>().ok())
            .map(UserId)
            .map(Route::StoreByUser)
    });

    // Stores/:id/products route
    router.add_route_with_params(r"^/stores/(\d+)/products$", |params| {
        params
            .get(0)
            .and_then(|string_id| string_id.parse::<i32>().ok())
            .map(StoreId)
            .map(Route::StoreProducts)
    });

    // Stores/:id/products/count route
    router.add_route_with_params(r"^/stores/(\d+)/products/count$", |params| {
        params
            .get(0)
            .and_then(|string_id| string_id.parse::<i32>().ok())
            .map(StoreId)
            .map(Route::StoreProductsCount)
    });

    // Stores Cart route
    router.add_route(r"^/stores/cart$", || Route::StoresCart);

    // Stores Search route
    router.add_route(r"^/stores/search$", || Route::StoresSearch);

    // Stores Search filter count route
    router.add_route(r"^/stores/search/filters/count$", || Route::StoresSearchFiltersCount);

    // Stores Search filter country route
    router.add_route(r"^/stores/search/filters/country$", || Route::StoresSearchFiltersCountry);

    // Stores Search filter  route
    router.add_route(r"^/stores/search/filters/category$", || Route::StoresSearchFiltersCategory);

    // Stores auto complete route
    router.add_route(r"^/stores/auto_complete$", || Route::StoresAutoComplete);

    // Products Routes
    router.add_route(r"^/products$", || Route::Products);

    // Product Store Id Routes
    router.add_route(r"^/products/store_id$", || Route::ProductStoreId);

    // Products/:id route
    router.add_route_with_params(r"^/products/(\d+)$", |params| {
        params
            .get(0)
            .and_then(|string_id| string_id.parse::<i32>().ok())
            .map(ProductId)
            .map(Route::Product)
    });

    // Products/by_base_product/:id route
    router.add_route_with_params(r"^/products/by_base_product/(\d+)$", |params| {
        params
            .get(0)
            .and_then(|string_id| string_id.parse::<i32>().ok())
            .map(BaseProductId)
            .map(Route::ProductsByBaseProduct)
    });

    // Products/:id/attributes route
    router.add_route_with_params(r"^/products/(\d+)/attributes$", |params| {
        params
            .get(0)
            .and_then(|string_id| string_id.parse::<i32>().ok())
            .map(ProductId)
            .map(Route::ProductAttributes)
    });

    // Base products routes
    router.add_route(r"^/base_products$", || Route::BaseProducts);

    // Base products/:id route
    router.add_route_with_params(r"^/base_products/(\d+)$", |params| {
        params
            .get(0)
            .and_then(|string_id| string_id.parse::<i32>().ok())
            .map(BaseProductId)
            .map(Route::BaseProduct)
    });

    // Base products/:id/update_view route
    router.add_route_with_params(r"^/base_products/(\d+)/update_view$", |params| {
        params
            .get(0)
            .and_then(|string_id| string_id.parse::<i32>().ok())
            .map(BaseProductId)
            .map(Route::BaseProductWithViewsUpdate)
    });

    // Base products with variants routes
    router.add_route(r"^/base_products/with_variants$", || Route::BaseProductWithVariants);

    // Base products/:id/with_variants route
    router.add_route_with_params(r"^/base_products/(\d+)/with_variants$", |params| {
        params
            .get(0)
            .and_then(|string_id| string_id.parse::<i32>().ok())
            .map(BaseProductId)
            .map(Route::BaseProductWithVariant)
    });

    // base_products/by_product/:id route
    router.add_route_with_params(r"^/base_products/by_product/(\d+)$", |params| {
        params
            .get(0)
            .and_then(|string_id| string_id.parse::<i32>().ok())
            .map(ProductId)
            .map(Route::BaseProductByProduct)
    });

    // BaseProducts Search route
    router.add_route(r"^/base_products/search$", || Route::BaseProductsSearch);

    // BaseProducts auto complete route
    router.add_route(r"^/base_products/auto_complete$", || Route::BaseProductsAutoComplete);

    // BaseProducts with most discount
    router.add_route(r"^/base_products/most_discount$", || Route::BaseProductsMostDiscount);

    // BaseProducts with most viewed
    router.add_route(r"^/base_products/most_viewed$", || Route::BaseProductsMostViewed);

    // BaseProducts search filters price route
    router.add_route(r"^/base_products/search/filters/price$", || Route::BaseProductsSearchFiltersPrice);

    // BaseProducts search filters category route
    router.add_route(r"^/base_products/search/filters/category", || {
        Route::BaseProductsSearchFiltersCategory
    });

    // BaseProducts search filters attribute route
    router.add_route(r"^/base_products/search/filters/attributes", || {
        Route::BaseProductsSearchFiltersAttributes
    });

    // BaseProducts search filters count route
    router.add_route(r"^/base_products/search/filters/count", || Route::BaseProductsSearchFiltersCount);

    // User_roles Routes
    router.add_route(r"^/user_roles$", || Route::UserRoles);

    // User_roles/:id route
    router.add_route_with_params(r"^/user_roles/(\d+)$", |params| {
        params
            .get(0)
            .and_then(|string_id| string_id.parse::<i32>().ok())
            .map(UserId)
            .map(Route::UserRole)
    });

    // Attributes Routes
    router.add_route(r"^/attributes$", || Route::Attributes);

    // Attributes/:id route
    router.add_route_with_params(r"^/attributes/(\d+)$", |params| {
        params
            .get(0)
            .and_then(|string_id| string_id.parse::<i32>().ok())
            .map(Route::Attribute)
    });

    // Categories Routes
    router.add_route(r"^/categories$", || Route::Categories);

    // Categories/:id route
    router.add_route_with_params(r"^/categories/(\d+)$", |params| {
        params
            .get(0)
            .and_then(|string_id| string_id.parse::<i32>().ok())
            .map(Route::Category)
    });

    // roles/default/:id route
    router.add_route_with_params(r"^/roles/default/(\d+)$", |params| {
        params
            .get(0)
            .and_then(|string_id| string_id.parse::<i32>().ok())
            .map(UserId)
            .map(Route::DefaultRole)
    });

    // Categories Attributes Routes
    router.add_route(r"^/categories/attributes$", || Route::CategoryAttrs);

    // Categories Attributes/:id route
    router.add_route_with_params(r"^/categories/(\d+)/attributes$", |params| {
        params
            .get(0)
            .and_then(|string_id| string_id.parse::<i32>().ok())
            .map(Route::CategoryAttr)
    });

    // Currency exchange Routes
    router.add_route(r"^/currency_exchange$", || Route::CurrencyExchange);

    // Wizard store Routes
    router.add_route(r"^/wizard_stores$", || Route::WizardStores);

    // Moderator Product Comments Routes
    router.add_route(r"^/moderator_product_comments$", || Route::ModeratorProductComments);

    // Moderator Product Comment/:base_product_id Route
    router.add_route_with_params(r"^/moderator_product_comments/(\d+)$", |params| {
        params
            .get(0)
            .and_then(|string_id| string_id.parse::<i32>().ok())
            .map(BaseProductId)
            .map(Route::ModeratorBaseProductComment)
    });

    // Moderator Store Comments Routes
    router.add_route(r"^/moderator_store_comments$", || Route::ModeratorStoreComments);

    // Moderator Store Comment/:store_id Route
    router.add_route_with_params(r"^/moderator_store_comments/(\d+)$", |params| {
        params
            .get(0)
            .and_then(|string_id| string_id.parse::<i32>().ok())
            .map(StoreId)
            .map(Route::ModeratorStoreComment)
    });

    router
}
