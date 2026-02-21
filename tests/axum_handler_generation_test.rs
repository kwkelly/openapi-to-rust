use openapi_to_rust::analysis::{OperationInfo, ParameterInfo, RequestBodyContent, SchemaAnalysis};
use openapi_to_rust::generator::{CodeGenerator, GeneratorConfig};
use std::collections::BTreeMap;

fn create_test_config() -> GeneratorConfig {
    GeneratorConfig {
        spec_path: "test.json".into(),
        output_dir: "test_output".into(),
        module_name: "test_api".to_string(),
        enable_async_client: false,
        enable_axum_handlers: true,
        ..Default::default()
    }
}

fn create_test_analysis_with_operations(operations: Vec<OperationInfo>) -> SchemaAnalysis {
    let mut ops_map = BTreeMap::new();
    for op in operations {
        ops_map.insert(op.operation_id.clone(), op);
    }

    SchemaAnalysis {
        schemas: BTreeMap::new(),
        dependencies: openapi_to_rust::analysis::DependencyGraph::new(),
        patterns: openapi_to_rust::analysis::DetectedPatterns {
            tagged_enum_schemas: Default::default(),
            untagged_enum_schemas: Default::default(),
            type_mappings: Default::default(),
        },
        operations: ops_map,
    }
}

#[test]
fn test_generate_basic_get_handler() {
    let config = create_test_config();
    let generator = CodeGenerator::new(config);

    let operation = OperationInfo {
        operation_id: "getUser".to_string(),
        method: "GET".to_string(),
        path: "/users/{id}".to_string(),
        request_body: None,
        response_schemas: {
            let mut map = BTreeMap::new();
            map.insert("200".to_string(), "User".to_string());
            map
        },
        parameters: vec![ParameterInfo {
            name: "id".to_string(),
            location: "path".to_string(),
            required: true,
            schema_ref: Some("string".to_string()),
            rust_type: "String".to_string(),
        }],
        supports_streaming: false,
        stream_parameter: None,
    };

    let analysis = create_test_analysis_with_operations(vec![operation]);
    let result = generator.generate_axum_handlers(&analysis);
    assert!(result.is_ok());

    let result_str = result.unwrap().to_string();
    insta::assert_snapshot!(result_str);
}

#[test]
fn test_generate_post_handler_with_body() {
    let config = create_test_config();
    let generator = CodeGenerator::new(config);

    let operation = OperationInfo {
        operation_id: "createUser".to_string(),
        method: "POST".to_string(),
        path: "/users".to_string(),
        request_body: Some(RequestBodyContent::Json {
            schema_name: "CreateUserRequest".to_string(),
        }),
        response_schemas: {
            let mut map = BTreeMap::new();
            map.insert("201".to_string(), "User".to_string());
            map
        },
        parameters: vec![],
        supports_streaming: false,
        stream_parameter: None,
    };

    let analysis = create_test_analysis_with_operations(vec![operation]);
    let result = generator.generate_axum_handlers(&analysis);
    assert!(result.is_ok());

    let result_str = result.unwrap().to_string();
    insta::assert_snapshot!(result_str);
}

#[test]
fn test_generate_handler_with_query_params() {
    let config = create_test_config();
    let generator = CodeGenerator::new(config);

    let operation = OperationInfo {
        operation_id: "listUsers".to_string(),
        method: "GET".to_string(),
        path: "/users".to_string(),
        request_body: None,
        response_schemas: {
            let mut map = BTreeMap::new();
            map.insert("200".to_string(), "UserList".to_string());
            map
        },
        parameters: vec![
            ParameterInfo {
                name: "limit".to_string(),
                location: "query".to_string(),
                required: false,
                schema_ref: Some("integer".to_string()),
                rust_type: "i32".to_string(),
            },
            ParameterInfo {
                name: "offset".to_string(),
                location: "query".to_string(),
                required: false,
                schema_ref: Some("integer".to_string()),
                rust_type: "i32".to_string(),
            },
        ],
        supports_streaming: false,
        stream_parameter: None,
    };

    let analysis = create_test_analysis_with_operations(vec![operation]);
    let result = generator.generate_axum_handlers(&analysis);
    assert!(result.is_ok());

    let result_str = result.unwrap().to_string();
    insta::assert_snapshot!(result_str);
}

#[test]
fn test_generate_handler_with_mixed_params() {
    let config = create_test_config();
    let generator = CodeGenerator::new(config);

    let operation = OperationInfo {
        operation_id: "updateUser".to_string(),
        method: "PUT".to_string(),
        path: "/users/{id}".to_string(),
        request_body: Some(RequestBodyContent::Json {
            schema_name: "UpdateUserRequest".to_string(),
        }),
        response_schemas: {
            let mut map = BTreeMap::new();
            map.insert("200".to_string(), "User".to_string());
            map
        },
        parameters: vec![
            ParameterInfo {
                name: "id".to_string(),
                location: "path".to_string(),
                required: true,
                schema_ref: Some("string".to_string()),
                rust_type: "String".to_string(),
            },
            ParameterInfo {
                name: "version".to_string(),
                location: "query".to_string(),
                required: false,
                schema_ref: Some("integer".to_string()),
                rust_type: "i32".to_string(),
            },
        ],
        supports_streaming: false,
        stream_parameter: None,
    };

    let analysis = create_test_analysis_with_operations(vec![operation]);
    let result = generator.generate_axum_handlers(&analysis);
    assert!(result.is_ok());

    let result_str = result.unwrap().to_string();
    insta::assert_snapshot!(result_str);
}

#[test]
fn test_generate_delete_handler() {
    let config = create_test_config();
    let generator = CodeGenerator::new(config);

    let operation = OperationInfo {
        operation_id: "deleteUser".to_string(),
        method: "DELETE".to_string(),
        path: "/users/{id}".to_string(),
        request_body: None,
        response_schemas: {
            let mut map = BTreeMap::new();
            map.insert("204".to_string(), "void".to_string());
            map
        },
        parameters: vec![ParameterInfo {
            name: "id".to_string(),
            location: "path".to_string(),
            required: true,
            schema_ref: Some("string".to_string()),
            rust_type: "String".to_string(),
        }],
        supports_streaming: false,
        stream_parameter: None,
    };

    let analysis = create_test_analysis_with_operations(vec![operation]);
    let result = generator.generate_axum_handlers(&analysis);
    assert!(result.is_ok());

    let result_str = result.unwrap().to_string();
    insta::assert_snapshot!(result_str);
}

#[test]
fn test_generate_error_type() {
    let config = create_test_config();
    let generator = CodeGenerator::new(config);

    let operation = OperationInfo {
        operation_id: "testOp".to_string(),
        method: "GET".to_string(),
        path: "/test".to_string(),
        request_body: None,
        response_schemas: BTreeMap::new(),
        parameters: vec![],
        supports_streaming: false,
        stream_parameter: None,
    };

    let analysis = create_test_analysis_with_operations(vec![operation]);
    let result = generator.generate_axum_handlers(&analysis);
    assert!(result.is_ok());

    let result_str = result.unwrap().to_string();
    insta::assert_snapshot!(result_str);
}

#[test]
fn test_generate_router_builder() {
    let config = create_test_config();
    let generator = CodeGenerator::new(config);

    let operations = vec![
        OperationInfo {
            operation_id: "getUsers".to_string(),
            method: "GET".to_string(),
            path: "/users".to_string(),
            request_body: None,
            response_schemas: BTreeMap::new(),
            parameters: vec![],
            supports_streaming: false,
            stream_parameter: None,
        },
        OperationInfo {
            operation_id: "createUser".to_string(),
            method: "POST".to_string(),
            path: "/users".to_string(),
            request_body: None,
            response_schemas: BTreeMap::new(),
            parameters: vec![],
            supports_streaming: false,
            stream_parameter: None,
        },
    ];

    let analysis = create_test_analysis_with_operations(operations);
    let result = generator.generate_axum_handlers(&analysis);
    assert!(result.is_ok());

    let result_str = result.unwrap().to_string();
    insta::assert_snapshot!(result_str);
}

#[test]
fn test_generate_wrapper_handlers() {
    let config = create_test_config();
    let generator = CodeGenerator::new(config);

    let operation = OperationInfo {
        operation_id: "getUser".to_string(),
        method: "GET".to_string(),
        path: "/users/{id}".to_string(),
        request_body: None,
        response_schemas: {
            let mut map = BTreeMap::new();
            map.insert("200".to_string(), "User".to_string());
            map
        },
        parameters: vec![ParameterInfo {
            name: "id".to_string(),
            location: "path".to_string(),
            required: true,
            schema_ref: Some("string".to_string()),
            rust_type: "String".to_string(),
        }],
        supports_streaming: false,
        stream_parameter: None,
    };

    let analysis = create_test_analysis_with_operations(vec![operation]);
    let result = generator.generate_axum_handlers(&analysis);
    assert!(result.is_ok());

    let result_str = result.unwrap().to_string();
    insta::assert_snapshot!(result_str);
}
