use crate::analysis::{OperationInfo, SchemaAnalysis};
use crate::generator::CodeGenerator;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

impl CodeGenerator {
    /// Generate axum handler trait and related code
    pub fn generate_axum_handlers(&self, analysis: &SchemaAnalysis) -> crate::Result<String> {
        let trait_definition = self.generate_handler_trait(analysis)?;
        let param_structs = self.generate_parameter_structs(analysis)?;
        let error_type = self.generate_error_type();
        let router_builder = self.generate_router_builder(analysis)?;
        let wrapper_handlers = self.generate_wrapper_handlers(analysis)?;

        let code = quote! {
            use axum::{
                extract::{Path, Query, Json, State},
                http::StatusCode,
                response::{IntoResponse, Response},
                routing::{get, post, put, delete, patch},
                Router,
            };
            use serde::{Deserialize, Serialize};
            use std::future::Future;
            use std::pin::Pin;

            #error_type

            #param_structs

            #trait_definition

            #wrapper_handlers

            #router_builder
        };

        let syntax_tree = syn::parse2::<syn::File>(code)?;
        Ok(prettyplease::unparse(&syntax_tree))
    }

    /// Generate the main handler trait
    fn generate_handler_trait(&self, analysis: &SchemaAnalysis) -> crate::Result<TokenStream> {
        let methods: Vec<TokenStream> = analysis
            .operations
            .values()
            .map(|op| self.generate_trait_method(op))
            .collect();

        Ok(quote! {
            /// Trait defining all API handlers
            ///
            /// Implement this trait to provide business logic for each endpoint.
            /// The generated router will call these methods when routes are matched.
            #[async_trait::async_trait]
            pub trait ApiHandlers: Send + Sync + 'static {
                type Error: std::error::Error + Send + Sync + 'static;

                #(#methods)*
            }
        })
    }

    /// Generate a single trait method for an operation
    fn generate_trait_method(&self, op: &OperationInfo) -> TokenStream {
        let method_name = format_ident!("{}", self.to_rust_field_name(&op.operation_id));
        let doc_comment = self.generate_operation_doc_comment(op);

        // Generate parameters
        let params = self.generate_handler_params(op);

        // Generate return type
        let return_type = self.generate_return_type(op);

        quote! {
            #doc_comment
            async fn #method_name(
                &self,
                #params
            ) -> Result<#return_type, Self::Error>;
        }
    }

    /// Generate handler parameters based on operation parameters and request body
    fn generate_handler_params(&self, op: &OperationInfo) -> TokenStream {
        let mut params = Vec::new();

        // Group parameters by location
        let path_params: Vec<_> = op
            .parameters
            .iter()
            .filter(|p| p.location == "path")
            .collect();
        let query_params: Vec<_> = op
            .parameters
            .iter()
            .filter(|p| p.location == "query")
            .collect();
        let header_params: Vec<_> = op
            .parameters
            .iter()
            .filter(|p| p.location == "header")
            .collect();

        // Generate path parameter struct if needed
        if !path_params.is_empty() {
            let struct_name =
                format_ident!("{}PathParams", self.to_rust_type_name(&op.operation_id));
            params.push(quote! {
                Path(#struct_name): Path<#struct_name>
            });
        }

        // Generate query parameter struct if needed
        if !query_params.is_empty() {
            let struct_name =
                format_ident!("{}QueryParams", self.to_rust_type_name(&op.operation_id));
            params.push(quote! {
                Query(#struct_name): Query<#struct_name>
            });
        }

        // Generate header parameters (using axum's TypedHeader or custom extraction)
        for param in &header_params {
            let param_name = format_ident!("{}", self.to_rust_field_name(&param.name));
            // For simplicity, we'll use String for headers
            // In a more advanced version, we could use TypedHeader
            params.push(quote! {
                #param_name: Option<String>
            });
        }

        // Generate request body parameter if present
        if let Some(ref body) = op.request_body {
            if let Some(schema_name) = body.schema_name() {
                let body_type = format_ident!("{}", self.to_rust_type_name(schema_name));
                params.push(quote! {
                    Json(body): Json<#body_type>
                });
            }
        }

        quote! {
            #(#params),*
        }
    }

    /// Generate return type for handler
    fn generate_return_type(&self, op: &OperationInfo) -> TokenStream {
        // Find the success response type
        let success_response = op
            .response_schemas
            .iter()
            .find(|(code, _)| {
                code.starts_with('2')
                    || code.as_str() == "200"
                    || code.as_str() == "201"
                    || code.as_str() == "202"
            })
            .map(|(_, schema)| schema);

        match success_response {
            Some(schema_name) => {
                let response_type = format_ident!("{}", self.to_rust_type_name(schema_name));
                quote! { Json<#response_type> }
            }
            None => {
                // No response body, return status
                quote! { StatusCode }
            }
        }
    }

    /// Generate parameter structs for all operations
    fn generate_parameter_structs(&self, analysis: &SchemaAnalysis) -> crate::Result<TokenStream> {
        let structs: Vec<TokenStream> = analysis
            .operations
            .values()
            .map(|op| self.generate_operation_parameter_structs(op))
            .collect();

        Ok(quote! {
            #(#structs)*
        })
    }

    /// Generate parameter structs for a single operation
    fn generate_operation_parameter_structs(&self, op: &OperationInfo) -> TokenStream {
        let mut structs = TokenStream::new();

        // Path parameters struct
        let path_params: Vec<_> = op
            .parameters
            .iter()
            .filter(|p| p.location == "path")
            .collect();

        if !path_params.is_empty() {
            let struct_name =
                format_ident!("{}PathParams", self.to_rust_type_name(&op.operation_id));
            let fields: Vec<TokenStream> = path_params
                .iter()
                .map(|p| {
                    let field_name = format_ident!("{}", self.to_rust_field_name(&p.name));
                    let field_type = self.get_param_rust_type(p);
                    quote! {
                        pub #field_name: #field_type
                    }
                })
                .collect();

            structs.extend(quote! {
                #[derive(Debug, Clone, Deserialize, Serialize)]
                pub struct #struct_name {
                    #(#fields),*
                }
            });
        }

        // Query parameters struct
        let query_params: Vec<_> = op
            .parameters
            .iter()
            .filter(|p| p.location == "query")
            .collect();

        if !query_params.is_empty() {
            let struct_name =
                format_ident!("{}QueryParams", self.to_rust_type_name(&op.operation_id));
            let fields: Vec<TokenStream> = query_params
                .iter()
                .map(|p| {
                    let field_name = format_ident!("{}", self.to_rust_field_name(&p.name));
                    let field_type = self.get_param_rust_type(p);

                    // Make optional if not required
                    if p.required {
                        quote! {
                            pub #field_name: #field_type
                        }
                    } else {
                        quote! {
                            #[serde(default, skip_serializing_if = "Option::is_none")]
                            pub #field_name: Option<#field_type>
                        }
                    }
                })
                .collect();

            structs.extend(quote! {
                #[derive(Debug, Clone, Deserialize, Serialize)]
                pub struct #struct_name {
                    #(#fields),*
                }
            });
        }

        structs
    }

    /// Generate error type for handlers
    fn generate_error_type(&self) -> TokenStream {
        quote! {
            /// Error type for API handlers
            #[derive(Debug, thiserror::Error)]
            pub enum ApiError {
                #[error("Not found: {0}")]
                NotFound(String),

                #[error("Bad request: {0}")]
                BadRequest(String),

                #[error("Unauthorized: {0}")]
                Unauthorized(String),

                #[error("Forbidden: {0}")]
                Forbidden(String),

                #[error("Conflict: {0}")]
                Conflict(String),

                #[error("Internal server error: {0}")]
                InternalServerError(String),

                #[error("Service unavailable: {0}")]
                ServiceUnavailable(String),
            }

            impl IntoResponse for ApiError {
                fn into_response(self) -> Response {
                    let (status, message) = match self {
                        ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
                        ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
                        ApiError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg),
                        ApiError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg),
                        ApiError::Conflict(msg) => (StatusCode::CONFLICT, msg),
                        ApiError::InternalServerError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
                        ApiError::ServiceUnavailable(msg) => (StatusCode::SERVICE_UNAVAILABLE, msg),
                    };

                    let body = serde_json::json!({
                        "error": message,
                    });

                    (status, Json(body)).into_response()
                }
            }
        }
    }

    /// Generate wrapper handlers that bridge axum extractors to trait methods
    fn generate_wrapper_handlers(&self, analysis: &SchemaAnalysis) -> crate::Result<TokenStream> {
        let handlers: Vec<TokenStream> = analysis
            .operations
            .values()
            .map(|op| self.generate_wrapper_handler(op))
            .collect();

        Ok(quote! {
            #(#handlers)*
        })
    }

    /// Generate a wrapper handler for a single operation
    fn generate_wrapper_handler(&self, op: &OperationInfo) -> TokenStream {
        let handler_name = format_ident!("handle_{}", self.to_rust_field_name(&op.operation_id));
        let method_name = format_ident!("{}", self.to_rust_field_name(&op.operation_id));

        // Generate extractor parameters
        let extractor_params = self.generate_extractor_params(op);

        // Generate call arguments
        let call_args = self.generate_call_args(op);

        quote! {
            async fn #handler_name<H>(
                State(handler): State<H>,
                #extractor_params
            ) -> impl IntoResponse
            where
                H: ApiHandlers,
            {
                match handler.#method_name(#call_args).await {
                    Ok(response) => response.into_response(),
                    Err(err) => {
                        // Convert error to ApiError if needed
                        let api_error = ApiError::InternalServerError(err.to_string());
                        api_error.into_response()
                    }
                }
            }
        }
    }

    /// Generate extractor parameters for wrapper handler
    fn generate_extractor_params(&self, op: &OperationInfo) -> TokenStream {
        let mut params = Vec::new();

        // Group parameters by location
        let path_params: Vec<_> = op
            .parameters
            .iter()
            .filter(|p| p.location == "path")
            .collect();
        let query_params: Vec<_> = op
            .parameters
            .iter()
            .filter(|p| p.location == "query")
            .collect();
        let header_params: Vec<_> = op
            .parameters
            .iter()
            .filter(|p| p.location == "header")
            .collect();

        // Path parameters
        if !path_params.is_empty() {
            let struct_name =
                format_ident!("{}PathParams", self.to_rust_type_name(&op.operation_id));
            params.push(quote! {
                Path(path_params): Path<#struct_name>
            });
        }

        // Query parameters
        if !query_params.is_empty() {
            let struct_name =
                format_ident!("{}QueryParams", self.to_rust_type_name(&op.operation_id));
            params.push(quote! {
                Query(query_params): Query<#struct_name>
            });
        }

        // Header parameters
        for param in &header_params {
            let param_name = format_ident!("{}", self.to_rust_field_name(&param.name));
            params.push(quote! {
                #param_name: Option<String>
            });
        }

        // Request body
        if let Some(ref body) = op.request_body {
            if let Some(schema_name) = body.schema_name() {
                let body_type = format_ident!("{}", self.to_rust_type_name(schema_name));
                params.push(quote! {
                    Json(body): Json<#body_type>
                });
            }
        }

        quote! {
            #(#params),*
        }
    }

    /// Generate call arguments for trait method invocation
    fn generate_call_args(&self, op: &OperationInfo) -> TokenStream {
        let mut args = Vec::new();

        let path_params: Vec<_> = op
            .parameters
            .iter()
            .filter(|p| p.location == "path")
            .collect();
        let query_params: Vec<_> = op
            .parameters
            .iter()
            .filter(|p| p.location == "query")
            .collect();
        let header_params: Vec<_> = op
            .parameters
            .iter()
            .filter(|p| p.location == "header")
            .collect();

        if !path_params.is_empty() {
            args.push(quote! { Path(path_params) });
        }

        if !query_params.is_empty() {
            args.push(quote! { Query(query_params) });
        }

        for param in &header_params {
            let param_name = format_ident!("{}", self.to_rust_field_name(&param.name));
            args.push(quote! { #param_name });
        }

        if op.request_body.is_some() {
            args.push(quote! { Json(body) });
        }

        quote! {
            #(#args),*
        }
    }

    /// Generate router builder
    fn generate_router_builder(&self, analysis: &SchemaAnalysis) -> crate::Result<TokenStream> {
        let routes: Vec<TokenStream> = analysis
            .operations
            .values()
            .map(|op| self.generate_route(op))
            .collect();

        Ok(quote! {
            /// Create a router with all API endpoints
            ///
            /// # Example
            /// ```rust,ignore
            /// let app = create_router(my_handler);
            /// let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
            /// axum::serve(listener, app).await.unwrap();
            /// ```
            pub fn create_router<H>(handler: H) -> Router<H>
            where
                H: ApiHandlers + Clone + Send + Sync + 'static,
            {
                Router::new()
                    #(#routes)*
                    .with_state(handler)
            }
        })
    }

    /// Generate a single route
    fn generate_route(&self, op: &OperationInfo) -> TokenStream {
        let path = &op.path;
        let method = self.get_axum_method(op);
        let handler_name = format_ident!("handle_{}", self.to_rust_field_name(&op.operation_id));

        quote! {
            .route(#path, #method(#handler_name))
        }
    }

    /// Get axum routing method for an operation
    fn get_axum_method(&self, op: &OperationInfo) -> TokenStream {
        match op.method.to_uppercase().as_str() {
            "GET" => quote! { get },
            "POST" => quote! { post },
            "PUT" => quote! { put },
            "DELETE" => quote! { delete },
            "PATCH" => quote! { patch },
            _ => quote! { get },
        }
    }
}
