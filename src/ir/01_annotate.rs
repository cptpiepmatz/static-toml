use std::collections::BTreeMap;

use proc_macro2::Span;

use crate::{
    error::{AnnotateError, AnnotateErrorKind},
    item::{StructuredPath, StructuredPathSegment},
};

pub fn annotate(value: toml::Value, span: Span) -> Result<AnnotateIr, AnnotateError> {
    let path = StructuredPath::new(span);
    let toml::Value::Table(table) = value else {
        return Err(AnnotateError {
            kind: AnnotateErrorKind::RootNotTable(value),
            path,
        });
    };
    let root = AnnotatedTable::try_from_table(table, path)?;
    Ok(AnnotateIr { root })
}

pub struct AnnotateIr {
    pub root: AnnotatedTable,
}

pub struct AnnotatedTable {
    pub fields: BTreeMap<String, AnnotatedValue>,
    pub path: StructuredPath,
}

pub struct AnnotatedArray {
    pub elements: Vec<AnnotatedValue>,
    pub path: StructuredPath,
}

pub enum AnnotatedValue {
    String(String, StructuredPath),
    Integer(i64, StructuredPath),
    Float(f64, StructuredPath),
    Boolean(bool, StructuredPath),
    Table(AnnotatedTable),
    Array(AnnotatedArray),
}

impl AnnotatedTable {
    fn try_from_table(table: toml::Table, path: StructuredPath) -> Result<Self, AnnotateError> {
        let mut fields = BTreeMap::new();
        for (key, value) in table.into_iter() {
            let value = AnnotatedValue::try_from_value(
                value,
                path.with_segment(StructuredPathSegment::key(&key)),
            )?;
            fields.insert(key, value);
        }

        Ok(Self { fields, path })
    }
}

impl AnnotatedArray {
    fn try_from_array(
        array: Vec<toml::Value>,
        path: StructuredPath,
    ) -> Result<Self, AnnotateError> {
        let mut elements = Vec::with_capacity(array.len());
        for (index, element) in array.into_iter().enumerate() {
            elements.push(AnnotatedValue::try_from_value(
                element,
                path.with_segment(StructuredPathSegment::index(index..=index)),
            )?);
        }

        Ok(Self { elements, path })
    }
}

impl AnnotatedValue {
    fn try_from_value(value: toml::Value, path: StructuredPath) -> Result<Self, AnnotateError> {
        use toml::Value as V;
        Ok(match value {
            V::String(string) => AnnotatedValue::String(string, path),
            V::Integer(integer) => AnnotatedValue::Integer(integer, path),
            V::Float(float) => AnnotatedValue::Float(float, path),
            V::Boolean(boolean) => AnnotatedValue::Boolean(boolean, path),
            V::Array(array) => AnnotatedValue::Array(AnnotatedArray::try_from_array(array, path)?),
            V::Table(table) => AnnotatedValue::Table(AnnotatedTable::try_from_table(table, path)?),
            V::Datetime(datetime) => {
                return Err(AnnotateError {
                    kind: AnnotateErrorKind::DatetimeFound(datetime),
                    path,
                })
            }
        })
    }
}
