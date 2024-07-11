use utoipa::{
    openapi::security::{Http, SecurityScheme},
    Modify, OpenApi,
};

use crate::{
    balance::DepositAmount,
    transaction::{Transaction, TransactionRequest},
    user::UserCredentials,
};

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::user::signup,
        crate::user::login,
        crate::user::whoami,
        crate::transaction::create_transaction,
        crate::transaction::get_transaction_by_id,
        crate::transaction::transactions_list,
        crate::balance::deposit,
        crate::balance::get_balance,
    ),
    components(
        schemas(
            UserCredentials,
            DepositAmount,
            TransactionRequest,
            Transaction,
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
