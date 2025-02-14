use std::collections::BTreeMap;

use crate::{error::AnalyzeError, item::StructuredPath};

use super::{AnnotateIr, AnnotatedArray, AnnotatedTable, AnnotatedValue};

pub fn analyze(annotated: AnnotateIr) -> Result<AnalyzeIr, AnalyzeError> {
    let converted = AnalyzeIr::from_annotated(annotated);
    todo!()
}

pub struct AnalyzeIr {
    pub root: AnalyzedTable,
}

pub enum AnalyzedValue {
    String(Optionality<String>, StructuredPath),
    Integer(Optionality<i64>, StructuredPath),
    Float(Optionality<f64>, StructuredPath),
    Boolean(Optionality<bool>, StructuredPath),
    Table(AnalyzedTable),
    Array(AnalyzedArray),
}

/// Represent an optional value for a yet unknown type.
///
/// This will cause an error if at the end of this phase we still have unknown types.
#[derive(Debug)]
pub struct Unknown;

/// Represent the optionality of a field.
///
/// After checking the optionality of a field, it may become the `Optional` variant with the value
/// inside.
/// For fields that are marked optional but do not exist, we add them as `Optional<None>`.
pub enum Optionality<T> {
    Required(T),
    Optional(Option<T>),
}

pub struct AnalyzedTable {
    pub fields: BTreeMap<String, AnalyzedValue>,
    pub path: StructuredPath,
    pub additional_fields: bool,
}

pub enum AnalyzedArray {
    Tuple(Vec<AnalyzedValue>, StructuredPath),
    Array(Box<AnalyzedValue>, StructuredPath), // every element has the same type
}

impl AnalyzeIr {
    fn from_annotated(annotated: AnnotateIr) -> Self {
        Self {
            root: AnalyzedTable::from_annotated(annotated.root)
        }
    }
}

impl AnalyzedValue {
    fn from_annotated(annotated: AnnotatedValue) -> Self {
        match annotated {
            AnnotatedValue::String(s, path) => {
                AnalyzedValue::String(Optionality::Required(s), path)
            }
            AnnotatedValue::Integer(i, path) => {
                AnalyzedValue::Integer(Optionality::Required(i), path)
            }
            AnnotatedValue::Float(f, path) => AnalyzedValue::Float(Optionality::Required(f), path),
            AnnotatedValue::Boolean(b, path) => {
                AnalyzedValue::Boolean(Optionality::Required(b), path)
            }
            AnnotatedValue::Table(t) => AnalyzedValue::Table(AnalyzedTable::from_annotated(t)),
            AnnotatedValue::Array(a) => AnalyzedValue::Array(AnalyzedArray::from_annotated(a)),
        }
    }
}

impl AnalyzedTable {
    fn from_annotated(annotated: AnnotatedTable) -> Self {
        let fields = annotated
            .fields
            .into_iter()
            .map(|(key, value)| (key, AnalyzedValue::from_annotated(value)))
            .collect();

        Self {
            fields,
            path: annotated.path,
            additional_fields: false,
        }
    }
}

impl AnalyzedArray {
    fn from_annotated(annotated: AnnotatedArray) -> Self {
        Self::Tuple(
            annotated
                .elements
                .into_iter()
                .map(AnalyzedValue::from_annotated)
                .collect(),
            annotated.path,
        )
    }
}
