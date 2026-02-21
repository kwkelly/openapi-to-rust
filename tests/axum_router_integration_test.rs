//! Integration test that verifies generated axum handlers can be compiled and used in a real router
//!
//! This test generates complete axum handler code from an OpenAPI spec and verifies
//! that it compiles and can be used to create a working router.

use openapi_to_rust::analysis::SchemaAnalyzer;
use openapi_to_rust::generator::{CodeGenerator, GeneratorConfig};
use std::path::PathBuf;
use tempfile::TempDir;

fn create_petstore_spec() -> serde_json::Value {
    serde_json::json!({
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
                            "description": "Pet created",
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
                            "description": "Pet deleted"
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
                            "enum": ["dog", "cat", "bird"]
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
                            "enum": ["dog", "cat", "bird"]
                        }
                    },
                    "required": ["name", "species"]
                }
            }
        }
    })
}

#[test]
fn test_generated_handlers_compile() {
    // Create a temporary directory for generated code
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let output_dir = temp_dir.path().to_path_buf();

    // Create and analyze the spec
    let spec = create_petstore_spec();
    let mut analyzer = SchemaAnalyzer::new(spec).expect("Failed to create analyzer");
    let mut analysis = analyzer.analyze().expect("Failed to analyze spec");

    // Configure generator with axum handlers enabled
    let config = GeneratorConfig {
        spec_path: PathBuf::from("petstore.json"),
        output_dir: output_dir.clone(),
        module_name: "petstore_api".to_string(),
        enable_axum_handlers: true,
        enable_async_client: false,
        enable_sse_client: false,
        ..Default::default()
    };

    let generator = CodeGenerator::new(config);

    // Generate all code
    let result = generator
        .generate_all(&mut analysis)
        .expect("Failed to generate code");

    // Create a subdirectory for the module
    let module_dir = output_dir.join("petstore_api");
    std::fs::create_dir_all(&module_dir).expect("Failed to create module directory");

    // Write generated files to the module subdirectory
    for file in &result.files {
        let file_path = module_dir.join(&file.path);
        std::fs::write(&file_path, &file.content).expect("Failed to write file");
    }

    // Write mod.rs inside the module directory
    let mod_path = module_dir.join("mod.rs");
    std::fs::write(&mod_path, &result.mod_file.content).expect("Failed to write mod.rs");

    // Create a lib.rs that exposes the generated module
    let lib_rs = r#"
pub mod petstore_api;
pub use petstore_api::*;
"#;
    let lib_path = output_dir.join("lib.rs");
    std::fs::write(&lib_path, lib_rs).expect("Failed to write lib.rs");

    // Create a test file that uses the generated code
    let test_code = r#"
use petstore_api::handlers::*;
use petstore_api::types::*;
use axum::{
    extract::{Path, Query},
    Json, Router,
    http::StatusCode,
};

// Create a simple handler implementation
#[derive(Clone)]
struct TestHandler;

#[async_trait::async_trait]
impl ApiHandlers for TestHandler {
    type Error = ApiError;

    async fn list_pets(
        &self,
        Query(params): Query<ListPetsQueryParams>,
    ) -> Result<Json<PetList>, Self::Error> {
        Ok(Json(PetList {
            pets: Some(vec![]),
            total: Some(0),
        }))
    }

    async fn create_pet(
        &self,
        Json(body): Json<CreatePetRequest>,
    ) -> Result<Json<Pet>, Self::Error> {
        Ok(Json(Pet {
            id: "test-id".to_string(),
            name: body.name,
            species: PetSpecies::Dog,
        }))
    }

    async fn get_pet(
        &self,
        Path(params): Path<GetPetPathParams>,
    ) -> Result<Json<Pet>, Self::Error> {
        Ok(Json(Pet {
            id: params.id,
            name: "Test Pet".to_string(),
            species: PetSpecies::Dog,
        }))
    }

    async fn delete_pet(
        &self,
        Path(params): Path<DeletePetPathParams>,
    ) -> Result<StatusCode, Self::Error> {
        Ok(StatusCode::NO_CONTENT)
    }
}

fn main() {
    // Create router from the handler
    let handler = TestHandler;
    let _router: Router<TestHandler> = create_router(handler);
    println!("Router created successfully!");
}
"#;

    let test_file_path = output_dir.join("test_usage.rs");
    std::fs::write(&test_file_path, test_code).expect("Failed to write test file");

    // Create a Cargo.toml for the test project
    let cargo_toml = r#"
[package]
name = "test_petstore"
version = "0.1.0"
edition = "2021"

[lib]
name = "petstore_api"
path = "lib.rs"

[dependencies]
axum = "0.7"
async-trait = "0.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.0", features = ["full"] }
thiserror = "1.0"

[[bin]]
name = "test_usage"
path = "test_usage.rs"
"#;

    let cargo_path = output_dir.join("Cargo.toml");
    std::fs::write(&cargo_path, cargo_toml).expect("Failed to write Cargo.toml");

    // Try to compile the generated code
    let compile_output = std::process::Command::new("cargo")
        .args(&["build", "--manifest-path", cargo_path.to_str().unwrap()])
        .output()
        .expect("Failed to run cargo build");

    if !compile_output.status.success() {
        let stderr = String::from_utf8_lossy(&compile_output.stderr);
        let stdout = String::from_utf8_lossy(&compile_output.stdout);

        panic!(
            "Generated code failed to compile!\n\nSTDOUT:\n{}\n\nSTDERR:\n{}",
            stdout, stderr
        );
    }

    println!("✓ Generated axum handlers compile successfully!");
}

#[test]
fn test_handlers_have_correct_imports() {
    let spec = create_petstore_spec();
    let mut analyzer = SchemaAnalyzer::new(spec).expect("Failed to create analyzer");
    let mut analysis = analyzer.analyze().expect("Failed to analyze spec");

    let config = GeneratorConfig {
        spec_path: PathBuf::from("petstore.json"),
        output_dir: PathBuf::from("test_output"),
        module_name: "petstore_api".to_string(),
        enable_axum_handlers: true,
        enable_async_client: false,
        enable_sse_client: false,
        ..Default::default()
    };

    let generator = CodeGenerator::new(config);
    let result = generator
        .generate_all(&mut analysis)
        .expect("Failed to generate code");

    // Find the handlers file
    let handlers_file = result
        .files
        .iter()
        .find(|f| f.path == PathBuf::from("handlers.rs"))
        .expect("handlers.rs not found in generated files");

    // Verify it has the required imports
    assert!(
        handlers_file.content.contains("use super::types::*;"),
        "handlers.rs is missing 'use super::types::*;' import"
    );

    assert!(
        handlers_file.content.contains("use axum::"),
        "handlers.rs is missing axum imports"
    );

    assert!(
        handlers_file.content.contains("use serde::"),
        "handlers.rs is missing serde imports"
    );

    println!("✓ Generated handlers have correct imports");
}

#[test]
fn test_handlers_use_types_from_types_module() {
    let spec = create_petstore_spec();
    let mut analyzer = SchemaAnalyzer::new(spec).expect("Failed to create analyzer");
    let mut analysis = analyzer.analyze().expect("Failed to analyze spec");

    let config = GeneratorConfig {
        spec_path: PathBuf::from("petstore.json"),
        output_dir: PathBuf::from("test_output"),
        module_name: "petstore_api".to_string(),
        enable_axum_handlers: true,
        enable_async_client: false,
        enable_sse_client: false,
        ..Default::default()
    };

    let generator = CodeGenerator::new(config);
    let result = generator
        .generate_all(&mut analysis)
        .expect("Failed to generate code");

    // Find the handlers file
    let handlers_file = result
        .files
        .iter()
        .find(|f| f.path == PathBuf::from("handlers.rs"))
        .expect("handlers.rs not found");

    // Find the types file
    let types_file = result
        .files
        .iter()
        .find(|f| f.path == PathBuf::from("types.rs"))
        .expect("types.rs not found");

    // Check that handlers reference types that are defined in types.rs
    // For example, PetList, Pet, CreatePetRequest should be used in handlers
    assert!(
        handlers_file.content.contains("PetList") || handlers_file.content.contains("pet_list"),
        "handlers.rs should reference PetList type"
    );

    assert!(
        handlers_file.content.contains("Pet") || handlers_file.content.contains("pet"),
        "handlers.rs should reference Pet type"
    );

    // Verify these types are actually defined in types.rs
    assert!(
        types_file.content.contains("struct Pet") || types_file.content.contains("Pet"),
        "types.rs should define Pet type"
    );

    println!("✓ Handlers correctly reference types from types module");
}
