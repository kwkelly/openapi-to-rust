//! Example demonstrating axum handler generation from OpenAPI specs
//!
//! This example shows how to generate axum handler stubs that can be used
//! to implement server-side handlers for an OpenAPI specification.

use openapi_to_rust::analysis::SchemaAnalyzer;
use openapi_to_rust::generator::{CodeGenerator, GeneratorConfig};
use std::path::PathBuf;

fn main() {
    // Example OpenAPI spec with multiple endpoints
    let spec_json = r##"{
  "openapi": "3.1.0",
  "info": {
    "title": "Pet Store API",
    "version": "1.0.0"
  },
  "paths": {
    "/pets": {
      "get": {
        "operationId": "listPets",
        "summary": "List all pets",
        "parameters": [
          {
            "name": "limit",
            "in": "query",
            "required": false,
            "schema": {
              "type": "integer",
              "format": "int32"
            }
          },
          {
            "name": "offset",
            "in": "query",
            "required": false,
            "schema": {
              "type": "integer",
              "format": "int32"
            }
          }
        ],
        "responses": {
          "200": {
            "description": "A list of pets",
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/PetList"
                }
              }
            }
          }
        }
      },
      "post": {
        "operationId": "createPet",
        "summary": "Create a new pet",
        "requestBody": {
          "required": true,
          "content": {
            "application/json": {
              "schema": {
                "$ref": "#/components/schemas/CreatePetRequest"
              }
            }
          }
        },
        "responses": {
          "201": {
            "description": "Pet created successfully",
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/Pet"
                }
              }
            }
          }
        }
      }
    },
    "/pets/{id}": {
      "get": {
        "operationId": "getPet",
        "summary": "Get a pet by ID",
        "parameters": [
          {
            "name": "id",
            "in": "path",
            "required": true,
            "schema": {
              "type": "string"
            }
          }
        ],
        "responses": {
          "200": {
            "description": "A pet",
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/Pet"
                }
              }
            }
          },
          "404": {
            "description": "Pet not found"
          }
        }
      },
      "put": {
        "operationId": "updatePet",
        "summary": "Update a pet",
        "parameters": [
          {
            "name": "id",
            "in": "path",
            "required": true,
            "schema": {
              "type": "string"
            }
          }
        ],
        "requestBody": {
          "required": true,
          "content": {
            "application/json": {
              "schema": {
                "$ref": "#/components/schemas/UpdatePetRequest"
              }
            }
          }
        },
        "responses": {
          "200": {
            "description": "Pet updated successfully",
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/Pet"
                }
              }
            }
          }
        }
      },
      "delete": {
        "operationId": "deletePet",
        "summary": "Delete a pet",
        "parameters": [
          {
            "name": "id",
            "in": "path",
            "required": true,
            "schema": {
              "type": "string"
            }
          }
        ],
        "responses": {
          "204": {
            "description": "Pet deleted successfully"
          }
        }
      }
    }
  },
  "components": {
    "schemas": {
      "Pet": {
        "type": "object",
        "properties": {
          "id": {
            "type": "string"
          },
          "name": {
            "type": "string"
          },
          "species": {
            "type": "string",
            "enum": ["dog", "cat", "bird", "fish"]
          },
          "age": {
            "type": "integer"
          }
        },
        "required": ["id", "name", "species"]
      },
      "PetList": {
        "type": "object",
        "properties": {
          "pets": {
            "type": "array",
            "items": {
              "$ref": "#/components/schemas/Pet"
            }
          },
          "total": {
            "type": "integer"
          }
        }
      },
      "CreatePetRequest": {
        "type": "object",
        "properties": {
          "name": {
            "type": "string"
          },
          "species": {
            "type": "string",
            "enum": ["dog", "cat", "bird", "fish"]
          },
          "age": {
            "type": "integer"
          }
        },
        "required": ["name", "species"]
      },
      "UpdatePetRequest": {
        "type": "object",
        "properties": {
          "name": {
            "type": "string"
          },
          "age": {
            "type": "integer"
          }
        }
      }
    }
  }
}
"##;

    // Parse the spec
    let spec_value: serde_json::Value =
        serde_json::from_str(spec_json).expect("Failed to parse spec");

    // Analyze the spec
    let mut analyzer = SchemaAnalyzer::new(spec_value).expect("Failed to create analyzer");
    let mut analysis = analyzer.analyze().expect("Failed to analyze spec");

    // Configure generator with axum handlers enabled
    let config = GeneratorConfig {
        spec_path: PathBuf::from("petstore.json"),
        output_dir: PathBuf::from("src/generated"),
        module_name: "petstore_api".to_string(),
        enable_axum_handlers: true,
        enable_async_client: false, // We don't need the client for this example
        enable_sse_client: false,
        ..Default::default()
    };

    let generator = CodeGenerator::new(config);

    // Generate all code
    let result = generator
        .generate_all(&mut analysis)
        .expect("Failed to generate code");

    // Print the generated handler file
    println!("=== GENERATED AXUM HANDLERS ===\n");
    for file in &result.files {
        if file.path == std::path::PathBuf::from("handlers.rs") {
            // Format the code for better readability
            let syntax = syn::parse_file(&file.content).expect("Failed to parse generated code");
            let formatted = prettyplease::unparse(&syntax);
            println!("{}", formatted);
        }
    }

    println!("\n=== USAGE EXAMPLE ===\n");
    println!(
        r#"
// In your main.rs or server setup:

use petstore_api::handlers::{{ApiHandlers, create_router, ApiError}};
use petstore_api::types::{{Pet, PetList, CreatePetRequest, UpdatePetRequest}};
use axum::{{
    extract::{{Path, Query, Json}},
    http::StatusCode,
    response::Json as JsonResponse,
}};

// Implement the generated trait
struct PetStoreHandler {{
    db: Database,  // Your database connection
}}

#[async_trait::async_trait]
impl ApiHandlers for PetStoreHandler {{
    type Error = ApiError;

    async fn list_pets(
        &self,
        Query(params): Query<ListPetsQueryParams>,
    ) -> Result<JsonResponse<PetList>, Self::Error> {{
        let limit = params.limit.unwrap_or(10);
        let offset = params.offset.unwrap_or(0);

        let pets = self.db.list_pets(limit, offset).await
            .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

        Ok(JsonResponse(pets))
    }}

    async fn create_pet(
        &self,
        Json(body): Json<CreatePetRequest>,
    ) -> Result<JsonResponse<Pet>, Self::Error> {{
        let pet = self.db.create_pet(body).await
            .map_err(|e| ApiError::BadRequest(e.to_string()))?;

        Ok(JsonResponse(pet))
    }}

    async fn get_pet(
        &self,
        Path(params): Path<GetPetPathParams>,
    ) -> Result<JsonResponse<Pet>, Self::Error> {{
        let pet = self.db.get_pet(&params.id).await
            .ok_or_else(|| ApiError::NotFound(format!("Pet {{}} not found", params.id)))?;

        Ok(JsonResponse(pet))
    }}

    async fn update_pet(
        &self,
        Path(params): Path<UpdatePetPathParams>,
        Json(body): Json<UpdatePetRequest>,
    ) -> Result<JsonResponse<Pet>, Self::Error> {{
        let pet = self.db.update_pet(&params.id, body).await
            .map_err(|e| ApiError::BadRequest(e.to_string()))?;

        Ok(JsonResponse(pet))
    }}

    async fn delete_pet(
        &self,
        Path(params): Path<DeletePetPathParams>,
    ) -> Result<StatusCode, Self::Error> {{
        self.db.delete_pet(&params.id).await
            .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

        Ok(StatusCode::NO_CONTENT)
    }}
}}

#[tokio::main]
async fn main() {{
    let handler = PetStoreHandler {{
        db: Database::new(),
    }};

    let app = create_router(handler);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();

    println!("Server running on http://localhost:3000");
    axum::serve(listener, app).await.unwrap();
}}
"#
    );
}
