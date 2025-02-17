use std::collections::BTreeMap;

use proc_macro2::Span;

use crate::{
    error::{AnnotateError, AnnotateErrorKind},
    item::{StructuredPath, StructuredPathSegment},
};

pub fn annotate(value: toml::Value, span: Span) -> Result<AnnotateIr, AnnotateError> {
    let path = StructuredPath::new(span);
    let toml::Value::Table(table) = value else {
        return Err(AnnotateError { kind: AnnotateErrorKind::RootNotTable(value), path });
    };
    let root = AnnotatedTable::try_from_table(table, &path)?;
    Ok(AnnotateIr { root })
}

#[derive(Debug, Clone)]
pub struct AnnotateIr {
    pub root: AnnotatedTable,
}

#[derive(Debug, Clone)]
pub struct AnnotatedTable(pub BTreeMap<String, AnnotatedValue>);

#[derive(Debug, Clone)]
pub struct AnnotatedArray(pub Vec<AnnotatedValue>);

#[derive(Debug, Clone)]
pub struct AnnotatedValue {
    pub kind: AnnotatedValueKind,
    pub path: StructuredPath,
}

#[derive(Debug, Clone)]
pub enum AnnotatedValueKind {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Table(AnnotatedTable),
    Array(AnnotatedArray),
}

impl AnnotatedTable {
    fn try_from_table(table: toml::Table, path: &StructuredPath) -> Result<Self, AnnotateError> {
        let mut fields = BTreeMap::new();
        for (key, value) in table.into_iter() {
            let value = AnnotatedValue::try_from_value(
                value,
                path.with_segment(StructuredPathSegment::key(&key, None)),
            )?;
            fields.insert(key, value);
        }

        Ok(Self(fields))
    }
}

impl AnnotatedArray {
    fn try_from_array(
        array: Vec<toml::Value>,
        path: &StructuredPath,
    ) -> Result<Self, AnnotateError> {
        let mut elements = Vec::with_capacity(array.len());
        for (index, element) in array.into_iter().enumerate() {
            elements.push(AnnotatedValue::try_from_value(
                element,
                path.with_segment(StructuredPathSegment::index(index..=index, None)),
            )?);
        }

        Ok(Self(elements))
    }
}

impl AnnotatedValue {
    fn try_from_value(value: toml::Value, path: StructuredPath) -> Result<Self, AnnotateError> {
        use toml::Value as V;
        use AnnotatedValueKind as AVK;
        let kind = match value {
            V::String(string) => AVK::String(string),
            V::Integer(integer) => AVK::Integer(integer),
            V::Float(float) => AVK::Float(float),
            V::Boolean(boolean) => AVK::Boolean(boolean),
            V::Array(array) => AVK::Array(AnnotatedArray::try_from_array(array, &path)?),
            V::Table(table) => AVK::Table(AnnotatedTable::try_from_table(table, &path)?),
            V::Datetime(datetime) => {
                return Err(AnnotateError {
                    kind: AnnotateErrorKind::DatetimeFound(datetime),
                    path,
                })
            }
        };

        Ok(Self { kind, path })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;
    use syn::parse_quote;
    use AnnotatedValueKind as AVK;

    impl AnnotatedValue {
        fn as_table(&self) -> &AnnotatedTable {
            match self.kind {
                AVK::Table(ref table) => table,
                _ => panic!("not a table"),
            }
        }

        fn as_array(&self) -> &AnnotatedArray {
            match self.kind {
                AVK::Array(ref array) => array,
                _ => panic!("not an array"),
            }
        }
    }

    #[test]
    #[rustfmt::skip]
    fn test_try_from_value() {
        let example = indoc! {r#"
            name = "example"
            age = 30
            nested.key = "value"
            nested.number = 42
            list = [1, 2, 3]

            [[sea]]
            deep = true
        "#};
        let example: toml::Value = toml::from_str(example).unwrap();
        let toml::Value::Table(example) = example else { panic!("not a table") };
        let annotated = AnnotatedTable::try_from_table(
            example, 
            &StructuredPath::new(Span::call_site())
        ).unwrap();

        assert_eq!(annotated.0["name"].path, parse_quote!(name));
        assert_eq!(annotated.0["age"].path, parse_quote!(age));
        assert_eq!(annotated.0["nested"].path, parse_quote!(nested));
        assert_eq!(annotated.0["nested"].as_table().0["key"].path, parse_quote!(nested.key));
        assert_eq!(annotated.0["nested"].as_table().0["number"].path, parse_quote!(nested.number));
        assert_eq!(annotated.0["list"].as_array().0[0].path, parse_quote!(list.0));
        assert_eq!(annotated.0["list"].as_array().0[1].path, parse_quote!(list.1));
        assert_eq!(annotated.0["list"].as_array().0[2].path, parse_quote!(list.2));
        assert_eq!(annotated.0["sea"].as_array().0[0].path, parse_quote!(sea.0));
        assert_eq!(annotated.0["sea"].as_array().0[0].as_table().0["deep"].path, parse_quote!(sea.0.deep));
    }
}
