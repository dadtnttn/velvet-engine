use anyhow::{anyhow, Context, Result};
use velvet_script_vs3::Value;
pub fn field(value: &Value, key: &str) -> Result<Value> {
    value
        .map_get(key)
        .map_err(|e| anyhow!(e))?
        .ok_or_else(|| anyhow!("missing VS3 field `{key}`"))
}
pub fn optional_field(value: &Value, key: &str) -> Result<Option<Value>> {
    value.map_get(key).map_err(|e| anyhow!(e))
}
pub fn string(value: &Value, key: &str) -> Result<String> {
    field(value, key)?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| anyhow!("`{key}` is not a string"))
}
pub fn optional_string(value: &Value, key: &str) -> String {
    optional_field(value, key)
        .ok()
        .flatten()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default()
}
pub fn optional_number(value: &Value, key: &str) -> f32 {
    optional_field(value, key)
        .ok()
        .flatten()
        .and_then(|v| v.as_f64())
        .map(|v| v as f32)
        .unwrap_or_default()
}
pub fn optional_integer(value: &Value, key: &str) -> i64 {
    optional_field(value, key)
        .ok()
        .flatten()
        .and_then(|v| v.as_i64())
        .unwrap_or_default()
}
pub fn optional_boolean(value: &Value, key: &str) -> bool {
    matches!(optional_field(value, key), Ok(Some(Value::Bool(true))))
}
pub fn optional_list<T>(
    value: &Value,
    key: &str,
    parse: fn(&Value) -> Result<T>,
) -> Result<Vec<T>> {
    let Some(items) = optional_field(value, key)? else {
        return Ok(Vec::new());
    };
    items
        .list_items()
        .map_err(|e| anyhow!(e))?
        .iter()
        .map(parse)
        .collect::<Result<Vec<_>>>()
        .with_context(|| format!("VS3 list `{key}`"))
}
