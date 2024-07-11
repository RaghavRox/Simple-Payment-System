use utoipa::{
    openapi::security::{Http, SecurityScheme},
    Modify, OpenApi,
};

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::signup,
        crate::login,
        crate::whoami,
        crate::create_transaction,
        crate::deposit,
        crate::get_balance,
        crate::get_transaction_by_id,
        crate::transactions_list,
    ),
    components(
        schemas(
            crate::UserCredentials,
            crate::DepositAmount,
            crate::TransactionRequest,
            crate::Transaction,
        )
    ),
    modifiers(&SecurityAddon),
    tags(
      (name = "User Management", description = "User authentication and management"),  
      (name = "Account Balance Management", description = "Account Balances Management"),  
      (name = "Transactions" ),  
    ),
)]
pub(crate) struct ApiDoc;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "USER_JWT",
                SecurityScheme::Http(Http::new(utoipa::openapi::security::HttpAuthScheme::Bearer)),
            )
        }
    }
}
