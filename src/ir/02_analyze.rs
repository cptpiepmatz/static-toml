use std::{
    collections::BTreeMap,
    iter::{self, Peekable},
    mem,
    ops::RangeBounds,
    slice,
};
use syn::LitBool;

use crate::{
    error::AnalyzeError,
    item::{StructuredPath, StructuredPathSegment, TypeHint},
};

use super::{AnnotateIr, AnnotatedArray, AnnotatedTable, AnnotatedValue};

pub struct AnalyzeArgs {
    pub prefer_slices: Option<LitBool>,
    pub optional: Vec<(StructuredPath, Option<TypeHint>)>,
}

pub fn analyze(annotated: AnnotateIr, args: AnalyzeArgs) -> Result<AnalyzeIr, AnalyzeError> {
    let AnalyzeArgs {
        prefer_slices,
        optional,
    } = args;
    let ir = AnalyzeIr::from_annotated(annotated);
    let ir = AnalyzeIr::apply_optionality(ir, &optional)?;
    todo!()
}

pub struct AnalyzeIr {
    pub root: AnalyzedTable,
}

#[derive(Debug, Clone)]
pub enum AnalyzedValue {
    String(Optionality<String>, StructuredPath),
    Integer(Optionality<i64>, StructuredPath),
    Float(Optionality<f64>, StructuredPath),
    Boolean(Optionality<bool>, StructuredPath),
    Table(Optionality<AnalyzedTable>),
    Array(Optionality<AnalyzedArray>),

    Unknown(StructuredPath),
}

#[derive(Debug, Clone)]
pub struct AnalyzedTable {
    pub fields: BTreeMap<String, AnalyzedValue>,
    pub path: StructuredPath,
    pub additional_fields: bool,
}

#[derive(Debug, Clone)]
pub enum AnalyzedArray {
    Tuple(Vec<AnalyzedValue>, StructuredPath),
    Array(Box<AnalyzedValue>, StructuredPath), // every element has the same type
}

/// Represent the optionality of a field.
///
/// After checking the optionality of a field, it may become the `Optional` variant with the value
/// inside.
/// For fields that are marked optional but do not exist, we add them as `Optional<None>`.
#[derive(Debug, Clone)]
pub enum Optionality<T> {
    Required(T),
    Optional(Option<T>),
}

impl<T> Optionality<T> {
    fn make_optional(&mut self) {
        let value = mem::replace(self, Optionality::Optional(None));
        *self = match value {
            Self::Optional(_) => value,
            Self::Required(value) => Optionality::Optional(Some(value)),
        };
    }
}

struct SegmentIter<'p> {
    path: &'p StructuredPath,
    iter: slice::Iter<'p, StructuredPathSegment>,
}

impl<'p> SegmentIter<'p> {
    fn new(path: &'p StructuredPath) -> Self {
        SegmentIter {
            path,
            iter: path.segments.iter(),
        }
    }
}

impl<'p> Iterator for SegmentIter<'p> {
    type Item = &'p StructuredPathSegment;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

impl AnalyzeIr {
    fn from_annotated(annotated: AnnotateIr) -> Self {
        Self {
            root: AnalyzedTable::from_annotated(annotated.root),
        }
    }

    fn apply_optionality(
        mut self,
        optional_paths: &[(StructuredPath, Option<TypeHint>)],
    ) -> Result<Self, AnalyzeError> {
        for (structured_path, type_hint) in optional_paths {
            let mut iter = SegmentIter::new(structured_path);
            let segment = iter
                .next()
                .ok_or_else(|| AnalyzeError::EmptyStructuredPath(structured_path.clone()))?;
            self.root
                .apply_optionality(segment, &mut iter, type_hint.as_ref())?;
        }

        Ok(self)
    }
}

impl AnalyzedValue {
    fn from_annotated(annotated: AnnotatedValue) -> Self {
        match annotated {
            AnnotatedValue::String(string, path) => {
                AnalyzedValue::String(Optionality::Required(string), path)
            }
            AnnotatedValue::Integer(integer, path) => {
                AnalyzedValue::Integer(Optionality::Required(integer), path)
            }
            AnnotatedValue::Float(float, path) => {
                AnalyzedValue::Float(Optionality::Required(float), path)
            }
            AnnotatedValue::Boolean(boolean, path) => {
                AnalyzedValue::Boolean(Optionality::Required(boolean), path)
            }
            AnnotatedValue::Table(table) => {
                AnalyzedValue::Table(Optionality::Required(AnalyzedTable::from_annotated(table)))
            }
            AnnotatedValue::Array(array) => {
                AnalyzedValue::Array(Optionality::Required(AnalyzedArray::from_annotated(array)))
            }
        }
    }

    fn apply_optionality(
        &mut self,
        iter: &mut SegmentIter,
        type_hint: Option<&TypeHint>,
    ) -> Result<(), AnalyzeError> {
        let segment = iter.next();

        let key_of_primitive_err = |segment: &StructuredPathSegment| {
            Err(AnalyzeError::KeyOfPrimitive(
                iter.path.clone(),
                segment.clone(),
            ))
        };

        match (segment, self) {
            (Some(segment), Self::String(..)) => return key_of_primitive_err(segment),
            (Some(segment), Self::Integer(..)) => return key_of_primitive_err(segment),
            (Some(segment), Self::Float(..)) => return key_of_primitive_err(segment),
            (Some(segment), Self::Boolean(..)) => return key_of_primitive_err(segment),
            (Some(segment), Self::Table(opt)) => match opt {
                Optionality::Optional(None) => {}
                Optionality::Required(table) | Optionality::Optional(Some(table)) => {
                    table.apply_optionality(segment, iter, type_hint)?
                }
            },
            (Some(segment), Self::Array(opt)) => match opt {
                Optionality::Optional(None) => {}
                Optionality::Required(array) | Optionality::Optional(Some(array)) => {
                    array.apply_optionality(segment, iter, type_hint)?
                }
            },
            (None, Self::String(opt, _)) => opt.make_optional(),
            (None, Self::Integer(opt, _)) => opt.make_optional(),
            (None, Self::Float(opt, _)) => opt.make_optional(),
            (None, Self::Boolean(opt, _)) => opt.make_optional(),
            (None, Self::Table(opt)) => opt.make_optional(),
            (None, Self::Array(opt)) => opt.make_optional(),
            (_, Self::Unknown(_)) => {}
        }

        Ok(())
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

    fn apply_optionality(
        &mut self,
        segment: &StructuredPathSegment,
        iter: &mut SegmentIter,
        type_hint: Option<&TypeHint>,
    ) -> Result<(), AnalyzeError> {
        match segment {
            StructuredPathSegment::Index(..) => {
                return Err(AnalyzeError::UnmatchedField(
                    iter.path.clone(),
                    segment.clone(),
                ));
            }

            StructuredPathSegment::Key(key, _) => {
                // if we find the element, run further with it
                if let Some(element) = self.fields.get_mut(key) {
                    return element.apply_optionality(iter, type_hint);
                }

                // check if we are the last in the chain
                let next = iter.next();
                let path = iter.path.clone();
                if next.is_none() {
                    // the next element would be the end, add it as optional
                    let key = key.clone();
                    match type_hint {
                        Some(TypeHint::String) => self.fields.insert(
                            key,
                            AnalyzedValue::String(Optionality::Optional(None), path),
                        ),
                        Some(TypeHint::Integer) => self.fields.insert(
                            key,
                            AnalyzedValue::Integer(Optionality::Optional(None), path),
                        ),
                        Some(TypeHint::Float) => self
                            .fields
                            .insert(key, AnalyzedValue::Float(Optionality::Optional(None), path)),
                        Some(TypeHint::Boolean) => self.fields.insert(
                            key,
                            AnalyzedValue::Boolean(Optionality::Optional(None), path),
                        ),
                        None => self.fields.insert(key, AnalyzedValue::Unknown(path)),
                    };

                    return Ok(());
                }

                // we are not at the end, this is not allowed
                return Err(AnalyzeError::OptionalSegmentNotFound(path, segment.clone()));
            }

            StructuredPathSegment::Wildcard(_) => {
                let next = iter.next();
                match next {
                    Some(next) => {
                        return Err(AnalyzeError::OptionalSegmentNotFound(
                            iter.path.clone(),
                            next.clone(),
                        ))
                    }
                    None => self.additional_fields = true,
                }

                return Ok(());
            }
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

    fn apply_optionality(
        &mut self,
        segment: &StructuredPathSegment,
        iter: &mut SegmentIter,
        type_hint: Option<&TypeHint>,
    ) -> Result<(), AnalyzeError> {
        let range = match segment {
            StructuredPathSegment::Index(lower, upper, _) => (lower.as_ref(), upper.as_ref()),
            StructuredPathSegment::Key(..) | StructuredPathSegment::Wildcard(_) => {
                return Err(AnalyzeError::UnmatchedField(
                    iter.path.clone(),
                    segment.clone(),
                ));
            }
        };

        // we have no `Array` variants at this point
        if let Self::Tuple(items, _) = self {
            for (index, item) in items.iter_mut().enumerate() {
                if range.contains(&index) {
                    item.apply_optionality(iter, type_hint)?;
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proc_macro2::Span;
    use syn::parse_quote;

    impl AnalyzedValue {
        pub fn find_by_path<'v>(
            &'v self,
            target_path: &StructuredPath,
        ) -> Option<&'v AnalyzedValue> {
            match self {
                AnalyzedValue::String(_, path)
                | AnalyzedValue::Integer(_, path)
                | AnalyzedValue::Float(_, path)
                | AnalyzedValue::Boolean(_, path)
                | AnalyzedValue::Unknown(path) => match path == target_path {
                    true => Some(self),
                    false => None,
                },

                value @ (AnalyzedValue::Table(Optionality::Required(table))
                | AnalyzedValue::Table(Optionality::Optional(Some(table)))) => {
                    table.find_by_path(target_path, value)
                }
                value @ (AnalyzedValue::Array(Optionality::Required(array))
                | AnalyzedValue::Array(Optionality::Optional(Some(array)))) => {
                    array.find_by_path(target_path, value)
                }

                _ => None,
            }
        }
    }

    impl AnalyzedTable {
        pub fn find_by_path<'v>(
            &'v self,
            target_path: &StructuredPath,
            this: &'v AnalyzedValue,
        ) -> Option<&'v AnalyzedValue> {
            if &self.path == target_path {
                return Some(this);
            }

            for value in self.fields.values() {
                if let Some(found) = value.find_by_path(target_path) {
                    return Some(found);
                }
            }

            None
        }
    }

    impl AnalyzedArray {
        pub fn find_by_path<'v>(
            &'v self,
            target_path: &StructuredPath,
            this: &'v AnalyzedValue,
        ) -> Option<&'v AnalyzedValue> {
            match self {
                AnalyzedArray::Tuple(values, path) => {
                    if path == target_path {
                        return Some(this);
                    }

                    for value in values {
                        if let Some(found) = value.find_by_path(target_path) {
                            return Some(found);
                        }
                    }

                    None
                }

                AnalyzedArray::Array(value, path) => {
                    if path == target_path {
                        return Some(this);
                    }

                    value.find_by_path(target_path)
                }
            }
        }
    }

    macro_rules! assert_optionality {
        ($table:expr, $query:expr, Required($ty:ident)) => {{
            let query_path = parse_quote!($query);
            let path = StructuredPath::new(Span::call_site());
            // unknown value should be enough for testing
            match $table.find_by_path(&query_path, &AnalyzedValue::Unknown(path)) {
                Some(AnalyzedValue::$ty(Optionality::Required(_), ..)) => (),
                _ => panic!(
                    "Expected required {} value at {:?}",
                    stringify!($ty),
                    stringify!($query)
                ),
            }
        }};
        ($table:expr, $query:expr, Optional(Unknown)) => {{
            let query_path = parse_quote!($query);
            let path = StructuredPath::new(Span::call_site());
            match $table.find_by_path(&query_path, &AnalyzedValue::Unknown(path)) {
                Some(AnalyzedValue::Unknown(_)) => (),
                _ => panic!(
                    "Expected unknown optional value at {:?}",
                    stringify!($query)
                ),
            }
        }};
        ($table:expr, $query:expr, Optional($ty:ident)) => {{
            let query_path = parse_quote!($query);
            let path = StructuredPath::new(Span::call_site());
            match $table.find_by_path(&query_path, &AnalyzedValue::Unknown(path)) {
                Some(AnalyzedValue::$ty(Optionality::Optional(_), ..)) => (),
                _ => panic!(
                    "Expected optional {} value at {:?}",
                    stringify!($ty),
                    stringify!($query)
                ),
            }
        }};
    }

    fn example_initial_analyze_ir() -> AnalyzeIr {
        let example = include_str!("../../example.toml");
        let value: toml::Value = toml::from_str(example).expect("should be valid toml");
        let annotated =
            crate::ir::annotate(value, Span::call_site()).expect("should be valid annotated");
        AnalyzeIr::from_annotated(annotated)
    }

    #[test]
    fn apply_optionality_empty() {
        let initial = example_initial_analyze_ir();
        let ir = AnalyzeIr::apply_optionality(initial, &[]);
        let root = ir.expect("should apply optionality").root;
        assert_optionality!(root, title, Required(String));
        assert_optionality!(root, owner.name, Required(String));
        assert_optionality!(root, database.enabled, Required(Boolean));
        assert_optionality!(root, database.ports, Required(Array));
        assert_optionality!(root, database.ports[0], Required(Integer));
        assert_optionality!(root, database.ports[1], Required(Integer));
        assert_optionality!(root, database.ports[2], Required(Integer));
        assert_optionality!(root, database.data, Required(Array));
        assert_optionality!(root, database.data.0, Required(Array));
        assert_optionality!(root, database.data.0[0], Required(String));
        assert_optionality!(root, database.data.0[1], Required(String));
        assert_optionality!(root, database.data.1[0], Required(Float));
        assert_optionality!(root, database.temp_targets.cpu, Required(Float));
        assert_optionality!(root, database.temp_targets.case, Required(Float));
        assert_optionality!(root, servers, Required(Table));
        assert_optionality!(root, servers.alpha, Required(Table));
        assert_optionality!(root, servers.alpha.ip, Required(String));
        assert_optionality!(root, servers.alpha.role, Required(String));
        assert_optionality!(root, servers.beta, Required(Table));
        assert_optionality!(root, servers.beta.ip, Required(String));
        assert_optionality!(root, servers.beta.role, Required(String));
    }

    #[test]
    fn apply_optionality_plain() {
        let initial = example_initial_analyze_ir();
        let ir = AnalyzeIr::apply_optionality(initial, &[(parse_quote!(title), None)]);
        let root = ir.expect("should apply optionality").root;
        assert_optionality!(root, title, Optional(String));
        assert_optionality!(root, owner.name, Required(String));
        assert_optionality!(root, database.enabled, Required(Boolean));
        assert_optionality!(root, database.ports, Required(Array));
        assert_optionality!(root, database.ports[0], Required(Integer));
        assert_optionality!(root, database.ports[1], Required(Integer));
        assert_optionality!(root, database.ports[2], Required(Integer));
        assert_optionality!(root, database.data, Required(Array));
        assert_optionality!(root, database.data.0, Required(Array));
        assert_optionality!(root, database.data.0[0], Required(String));
        assert_optionality!(root, database.data.0[1], Required(String));
        assert_optionality!(root, database.data.1[0], Required(Float));
        assert_optionality!(root, database.temp_targets.cpu, Required(Float));
        assert_optionality!(root, database.temp_targets.case, Required(Float));
        assert_optionality!(root, servers, Required(Table));
        assert_optionality!(root, servers.alpha, Required(Table));
        assert_optionality!(root, servers.alpha.ip, Required(String));
        assert_optionality!(root, servers.alpha.role, Required(String));
        assert_optionality!(root, servers.beta, Required(Table));
        assert_optionality!(root, servers.beta.ip, Required(String));
        assert_optionality!(root, servers.beta.role, Required(String));
    }

    #[test]
    fn apply_optionality_nested() {
        let initial = example_initial_analyze_ir();
        let ir = AnalyzeIr::apply_optionality(
            initial,
            &[
                (parse_quote!(owner.name), None),
                (parse_quote!(database.enabled), None),
                (parse_quote!(database.temp_targets.cpu), None),
                (parse_quote!(servers.beta), None),
                (parse_quote!(database.ports[1]), None),
            ],
        );
        let root = ir.expect("should apply optionality").root;
        assert_optionality!(root, title, Required(String));
        assert_optionality!(root, owner.name, Optional(String));
        assert_optionality!(root, database.enabled, Optional(Boolean));
        assert_optionality!(root, database.ports, Required(Array));
        assert_optionality!(root, database.ports[0], Required(Integer));
        assert_optionality!(root, database.ports[1], Optional(Integer));
        assert_optionality!(root, database.ports[2], Required(Integer));
        assert_optionality!(root, database.data, Required(Array));
        assert_optionality!(root, database.data.0, Required(Array));
        assert_optionality!(root, database.data.0[0], Required(String));
        assert_optionality!(root, database.data.0[1], Required(String));
        assert_optionality!(root, database.data.1[0], Required(Float));
        assert_optionality!(root, database.temp_targets.cpu, Optional(Float));
        assert_optionality!(root, database.temp_targets.case, Required(Float));
        assert_optionality!(root, servers, Required(Table));
        assert_optionality!(root, servers.alpha, Required(Table));
        assert_optionality!(root, servers.alpha.ip, Required(String));
        assert_optionality!(root, servers.alpha.role, Required(String));
        assert_optionality!(root, servers.beta, Optional(Table));
        assert_optionality!(root, servers.beta.ip, Required(String));
        assert_optionality!(root, servers.beta.role, Required(String));
    }

    #[test]
    fn apply_optionality_range() {
        let initial = example_initial_analyze_ir();
        let ir = AnalyzeIr::apply_optionality(
            initial,
            &[
                (parse_quote!(database.ports[..=1]), None),
                (parse_quote!(database.data[]), None),
            ],
        );
        let root = ir.expect("should apply optionality").root;
        assert_optionality!(root, title, Required(String));
        assert_optionality!(root, owner.name, Required(String));
        assert_optionality!(root, database.enabled, Required(Boolean));
        assert_optionality!(root, database.ports, Required(Array));
        assert_optionality!(root, database.ports[0], Optional(Integer));
        assert_optionality!(root, database.ports[1], Optional(Integer));
        assert_optionality!(root, database.ports[2], Required(Integer));
        assert_optionality!(root, database.data, Required(Array));
        assert_optionality!(root, database.data.0, Optional(Array));
        assert_optionality!(root, database.data.0[0], Required(String));
        assert_optionality!(root, database.data.0[1], Required(String));
        assert_optionality!(root, database.data.1[0], Required(Float));
        assert_optionality!(root, database.temp_targets.cpu, Required(Float));
        assert_optionality!(root, database.temp_targets.case, Required(Float));
        assert_optionality!(root, servers, Required(Table));
        assert_optionality!(root, servers.alpha, Required(Table));
        assert_optionality!(root, servers.alpha.ip, Required(String));
        assert_optionality!(root, servers.alpha.role, Required(String));
        assert_optionality!(root, servers.beta, Required(Table));
        assert_optionality!(root, servers.beta.ip, Required(String));
        assert_optionality!(root, servers.beta.role, Required(String));
    }

    #[test]
    fn apply_optionality_add_new_entries() {
        let initial = example_initial_analyze_ir();
        let ir = AnalyzeIr::apply_optionality(
            initial,
            &[
                (parse_quote!(subtitle), None),
                (parse_quote!(description), Some(TypeHint::String)),
                (
                    parse_quote!(database.temp_targets.gpu),
                    Some(TypeHint::Float),
                ),
            ],
        );
        let root = ir.expect("should apply optionality").root;
        assert_optionality!(root, title, Required(String));
        assert_optionality!(root, subtitle, Optional(Unknown));
        assert_optionality!(root, description, Optional(String));
        assert_optionality!(root, owner.name, Required(String));
        assert_optionality!(root, database.enabled, Required(Boolean));
        assert_optionality!(root, database.ports, Required(Array));
        assert_optionality!(root, database.ports[0], Required(Integer));
        assert_optionality!(root, database.ports[1], Required(Integer));
        assert_optionality!(root, database.ports[2], Required(Integer));
        assert_optionality!(root, database.data, Required(Array));
        assert_optionality!(root, database.data.0, Required(Array));
        assert_optionality!(root, database.data.0[0], Required(String));
        assert_optionality!(root, database.data.0[1], Required(String));
        assert_optionality!(root, database.data.1[0], Required(Float));
        assert_optionality!(root, database.temp_targets.cpu, Required(Float));
        assert_optionality!(root, database.temp_targets.case, Required(Float));
        assert_optionality!(root, database.temp_targets.gpu, Optional(Float));
        assert_optionality!(root, servers, Required(Table));
        assert_optionality!(root, servers.alpha, Required(Table));
        assert_optionality!(root, servers.alpha.ip, Required(String));
        assert_optionality!(root, servers.alpha.role, Required(String));
        assert_optionality!(root, servers.beta, Required(Table));
        assert_optionality!(root, servers.beta.ip, Required(String));
        assert_optionality!(root, servers.beta.role, Required(String));
    }

    #[test]
    fn apply_optionality_wildcard() {
        let initial = example_initial_analyze_ir();
        let ir =
            AnalyzeIr::apply_optionality(initial, &[(parse_quote!(database.temp_targets.*), None)]);
        let root = ir.expect("should apply optionality").root;

        let path = StructuredPath::new(Span::call_site());
        let this = &AnalyzedValue::Unknown(path);
        let temp_targets = root
            .find_by_path(&parse_quote!(database.temp_targets), this)
            .expect("should exist");
        let AnalyzedValue::Table(Optionality::Required(temp_targets)) = temp_targets else {
            panic!("not a required table")
        };
        assert!(temp_targets.additional_fields);
    }
}
