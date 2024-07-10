use utoipa::{
    openapi::security::{Http, SecurityScheme},
    Modify, OpenApi,
};

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::signup,
        crate::login,
        crate::whoami
    ),
    components(
        schemas(
            crate::UserCredentials,
        )
    ),
    modifiers(&SecurityAddon),
    tags(
      (name = "User Management", description = "User authentication and management"),  
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
