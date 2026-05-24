use serde_json::{Map, Value, json};

pub fn normalize_for_swagger_editor(value: &mut Value) {
    value["openapi"] = Value::String("3.0.3".to_owned());
    assign_unique_operation_ids(value);
    normalize_schema(value);
}

fn assign_unique_operation_ids(value: &mut Value) {
    let Some(paths) = value.get_mut("paths").and_then(Value::as_object_mut) else {
        return;
    };

    for (path, item) in paths {
        let Some(methods) = item.as_object_mut() else {
            continue;
        };

        for (method, operation) in methods {
            if !matches!(
                method.as_str(),
                "get" | "post" | "put" | "patch" | "delete" | "head" | "options" | "trace"
            ) {
                continue;
            }

            let operation_id = format!(
                "{}_{}",
                method,
                path.trim_start_matches('/')
                    .replace(['/', '-', '{', '}'], "_")
                    .trim_matches('_')
            );
            operation["operationId"] = Value::String(operation_id);
        }
    }
}

fn normalize_schema(value: &mut Value) {
    match value {
        Value::Object(object) => {
            normalize_type_array(object);
            normalize_one_of_null(object);

            for value in object.values_mut() {
                normalize_schema(value);
            }
        }
        Value::Array(items) => {
            for value in items {
                normalize_schema(value);
            }
        }
        _ => {}
    }
}

fn normalize_type_array(object: &mut Map<String, Value>) {
    let Some(types) = object.get("type").and_then(Value::as_array) else {
        return;
    };
    let non_null_types = types
        .iter()
        .filter_map(Value::as_str)
        .filter(|schema_type| *schema_type != "null")
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    let has_null = types
        .iter()
        .filter_map(Value::as_str)
        .any(|schema_type| schema_type == "null");

    if has_null {
        object.insert("nullable".to_owned(), Value::Bool(true));
    }

    match non_null_types.as_slice() {
        [schema_type] => {
            object.insert("type".to_owned(), Value::String(schema_type.clone()));
        }
        [] => {
            object.remove("type");
        }
        _ => {
            object.insert(
                "oneOf".to_owned(),
                Value::Array(
                    non_null_types
                        .into_iter()
                        .map(|schema_type| json!({ "type": schema_type }))
                        .collect(),
                ),
            );
            object.remove("type");
        }
    }
}

fn normalize_one_of_null(object: &mut Map<String, Value>) {
    let Some(Value::Array(mut one_of)) = object.remove("oneOf") else {
        return;
    };

    let original_len = one_of.len();
    one_of.retain(|schema| !is_null_schema(schema));
    if one_of.len() == original_len {
        object.insert("oneOf".to_owned(), Value::Array(one_of));
        return;
    }

    object.insert("nullable".to_owned(), Value::Bool(true));
    if one_of.len() == 1 {
        if let Some(schema) = one_of.pop() {
            object.remove("oneOf");
            if let Some(schema) = schema.as_object() {
                for (key, value) in schema {
                    object.entry(key.clone()).or_insert_with(|| value.clone());
                }
            }
        }
    } else if !one_of.is_empty() {
        object.insert("oneOf".to_owned(), Value::Array(one_of));
    }
}

fn is_null_schema(value: &Value) -> bool {
    value
        .get("type")
        .and_then(Value::as_str)
        .map(|schema_type| schema_type == "null")
        .unwrap_or(false)
}
