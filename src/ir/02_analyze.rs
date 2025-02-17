use std::{
    collections::{BTreeMap, HashSet},
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

#[derive(Debug)]
pub struct AnalyzeArgs {
    pub prefer_slices: Option<LitBool>,
    pub optional: Vec<(StructuredPath, Option<TypeHint>)>,
}

pub fn analyze(annotated: AnnotateIr, args: AnalyzeArgs) -> Result<AnalyzeIr, AnalyzeError> {
    let AnalyzeArgs {
        prefer_slices,
        optional,
    } = args;
    let mut ir = AnalyzeIr::from_annotated(annotated);
    AnalyzeIr::apply_optionality(&mut ir, &optional)?;
    if prefer_slices.map(|b| b.value()).unwrap_or(true) {}
    todo!()
}

#[derive(Debug)]
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
        &mut self,
        optional_paths: &[(StructuredPath, Option<TypeHint>)],
    ) -> Result<(), AnalyzeError> {
        for (structured_path, type_hint) in optional_paths {
            let mut iter = SegmentIter::new(structured_path);
            let segment = iter
                .next()
                .ok_or_else(|| AnalyzeError::EmptyStructuredPath(structured_path.clone()))?;
            self.root
                .apply_optionality(segment, &mut iter, type_hint.as_ref())?;
        }

        Ok(())
    }

    fn merge_slices(&mut self) -> Result<(), AnalyzeError> {
        todo!()
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

    fn type_equality(&self, other: &Self) -> bool {
        use AnalyzedValue as AV;
        match (self, other) {
            (AV::Unknown(_), _) => true,
            (_, AV::Unknown(_)) => true,
            (AV::String(left, _), AV::String(right, _)) => match (left, right) {
                (Optionality::Required(_), Optionality::Required(_)) => true,
                (Optionality::Optional(_), Optionality::Optional(_)) => true,
                _ => false,
            },
            (AV::Integer(left, _), AV::Integer(right, _)) => match (left, right) {
                (Optionality::Required(_), Optionality::Required(_)) => true,
                (Optionality::Optional(_), Optionality::Optional(_)) => true,
                _ => false,
            },
            (AV::Float(left, _), AV::Float(right, _)) => match (left, right) {
                (Optionality::Required(_), Optionality::Required(_)) => true,
                (Optionality::Optional(_), Optionality::Optional(_)) => true,
                _ => false,
            },
            (AV::Boolean(left, _), AV::Boolean(right, _)) => match (left, right) {
                (Optionality::Required(_), Optionality::Required(_)) => true,
                (Optionality::Optional(_), Optionality::Optional(_)) => true,
                _ => false,
            },
            (AV::Table(left), AV::Table(right)) => match (left, right) {
                (Optionality::Required(left), Optionality::Required(right))
                | (Optionality::Optional(Some(left)), Optionality::Optional(Some(right))) => {
                    left.type_equality(right)
                }
                (Optionality::Optional(None), Optionality::Optional(_)) => true,
                (Optionality::Optional(_), Optionality::Optional(None)) => true,
                _ => false,
            },
            (AV::Array(left), AV::Array(right)) => match (left, right) {
                (Optionality::Required(left), Optionality::Required(right))
                | (Optionality::Optional(Some(left)), Optionality::Optional(Some(right))) => {
                    left.type_equality(right)
                }
                (Optionality::Optional(None), Optionality::Optional(_)) => true,
                (Optionality::Optional(_), Optionality::Optional(None)) => true,
                _ => false,
            },
            _ => false,
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

    fn type_equality(&self, other: &Self) -> bool {
        let mut keys = HashSet::new();
        keys.extend(self.fields.keys());
        keys.extend(other.fields.keys());

        for key in keys {
            let left = self.fields.get(key);
            let right = other.fields.get(key);
            let typed_equal = match (left, self.additional_fields, right, other.additional_fields) {
                (None, _, None, _) => unreachable!("keys are selected from both tables"),
                (Some(_), _, None, true) => true,
                (None, true, Some(_), _) => true,
                (None, false, Some(_), _) => false,
                (Some(_), _, None, false) => false,
                (Some(left), _, Some(right), _) => left.type_equality(right),
            };

            if !typed_equal {
                return false;
            }
        }

        true
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

    fn type_equality(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Tuple(..), Self::Array(..)) => false,
            (Self::Array(..), Self::Tuple(..)) => false,
            (Self::Array(left, _), Self::Array(right, _)) => left.type_equality(right),
            (Self::Tuple(left, _), Self::Tuple(right, _)) => {
                if left.len() != right.len() {
                    return false;
                }

                left.iter()
                    .zip(right.iter())
                    .all(|(left, right)| left.type_equality(right))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proc_macro2::Span;
    use syn::parse_quote;

    use AnalyzedArray as AA;
    use AnalyzedTable as AT;
    use AnalyzedValue as AV;
    use Optionality as Opt;

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
        let mut ir = example_initial_analyze_ir();
        AnalyzeIr::apply_optionality(&mut ir, &[]).expect("should apply optionality");
        let root = ir.root;
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
        let mut ir = example_initial_analyze_ir();
        AnalyzeIr::apply_optionality(&mut ir, &[(parse_quote!(title), None)])
            .expect("should apply optionality");
        let root = ir.root;
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
        let mut ir = example_initial_analyze_ir();
        AnalyzeIr::apply_optionality(
            &mut ir,
            &[
                (parse_quote!(owner.name), None),
                (parse_quote!(database.enabled), None),
                (parse_quote!(database.temp_targets.cpu), None),
                (parse_quote!(servers.beta), None),
                (parse_quote!(database.ports[1]), None),
            ],
        )
        .expect("should apply optionality");
        let root = ir.root;
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
        let mut ir = example_initial_analyze_ir();
        AnalyzeIr::apply_optionality(
            &mut ir,
            &[
                (parse_quote!(database.ports[..=1]), None),
                (parse_quote!(database.data[]), None),
            ],
        )
        .expect("should apply optionality");
        let root = ir.root;
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
        let mut ir = example_initial_analyze_ir();
        AnalyzeIr::apply_optionality(
            &mut ir,
            &[
                (parse_quote!(subtitle), None),
                (parse_quote!(description), Some(TypeHint::String)),
                (
                    parse_quote!(database.temp_targets.gpu),
                    Some(TypeHint::Float),
                ),
            ],
        )
        .expect("should apply optionality");
        let root = ir.root;
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
        let mut ir = example_initial_analyze_ir();
        AnalyzeIr::apply_optionality(&mut ir, &[(parse_quote!(database.temp_targets.*), None)])
            .expect("should apply optionality");
        let root = ir.root;

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

    #[test]
    fn apply_optionality_empty_path() {
        let mut ir = example_initial_analyze_ir();
        let err = AnalyzeIr::apply_optionality(&mut ir, &[(parse_quote!(), None)])
            .expect_err("should return an error for empty structured path");

        match err {
            AnalyzeError::EmptyStructuredPath(_) => (),
            _ => panic!("Unexpected error: {:?}", err),
        }
    }

    #[test]
    fn apply_optionality_unmatched_field() {
        let mut ir = example_initial_analyze_ir();
        let err = AnalyzeIr::apply_optionality(&mut ir, &[(parse_quote!(nonexistent.key), None)])
            .expect_err("should return an error for unmatched field");

        match err {
            AnalyzeError::OptionalSegmentNotFound(path, segment) => {
                assert_eq!(path.to_string(), "nonexistent.key");
                assert_eq!(segment.to_string(), "nonexistent");
            }
            _ => panic!("Unexpected error: {:?}", err),
        }
    }

    #[test]
    fn apply_optionality_key_of_primitive() {
        let mut ir = example_initial_analyze_ir();
        let err = AnalyzeIr::apply_optionality(&mut ir, &[(parse_quote!(title.invalid), None)])
            .expect_err("should return an error for accessing key on primitive");

        match err {
            AnalyzeError::KeyOfPrimitive(path, segment) => {
                assert_eq!(path.to_string(), "title.invalid");
                assert_eq!(segment.to_string(), "invalid");
            }
            _ => panic!("Unexpected error: {:?}", err),
        }
    }

    #[test]
    fn apply_optionality_index_on_table() {
        let mut ir = example_initial_analyze_ir();
        let err = AnalyzeIr::apply_optionality(&mut ir, &[(parse_quote!(database[0]), None)])
            .expect_err("should return an error for using index on table");

        match err {
            AnalyzeError::UnmatchedField(path, segment) => {
                assert_eq!(path.to_string(), "database[0]");
                assert_eq!(segment.to_string(), "[0]");
            }
            _ => panic!("Unexpected error: {:?}", err),
        }
    }

    #[test]
    fn apply_optionality_wildcard_not_at_end() {
        let mut ir = example_initial_analyze_ir();
        let err = AnalyzeIr::apply_optionality(&mut ir, &[(parse_quote!(database.*.field), None)])
            .expect_err("should return an error for wildcard not at the end");

        match err {
            AnalyzeError::OptionalSegmentNotFound(path, segment) => {
                assert_eq!(path.to_string(), "database.*.field");
                assert_eq!(segment.to_string(), "field");
            }
            _ => panic!("Unexpected error: {:?}", err),
        }
    }

    #[test]
    fn test_type_equality_primitives() {
        let dummy: StructuredPath = parse_quote!(dummy);

        // Test String values.
        let s_req1 = AV::String(Opt::Required("hello".to_string()), dummy.clone());
        let s_req2 = AV::String(Opt::Required("world".to_string()), dummy.clone());
        assert!(s_req1.type_equality(&s_req2));

        let s_opt1 = AV::String(Opt::Optional(Some("hello".to_string())), dummy.clone());
        let s_opt2 = AV::String(Opt::Optional(Some("world".to_string())), dummy.clone());
        assert!(s_opt1.type_equality(&s_opt2));
        assert!(!s_req1.type_equality(&s_opt1));

        // Test Integer values.
        let i_req1 = AV::Integer(Opt::Required(10), dummy.clone());
        let i_req2 = AV::Integer(Opt::Required(20), dummy.clone());
        assert!(i_req1.type_equality(&i_req2));

        let i_opt1 = AV::Integer(Opt::Optional(Some(10)), dummy.clone());
        let i_opt2 = AV::Integer(Opt::Optional(Some(20)), dummy.clone());
        assert!(i_opt1.type_equality(&i_opt2));
        assert!(!i_req1.type_equality(&i_opt1));

        // Test Float values.
        let f_req1 = AV::Float(Opt::Required(1.0), dummy.clone());
        let f_req2 = AV::Float(Opt::Required(2.0), dummy.clone());
        assert!(f_req1.type_equality(&f_req2));

        let f_opt1 = AV::Float(Opt::Optional(Some(1.0)), dummy.clone());
        let f_opt2 = AV::Float(Opt::Optional(Some(2.0)), dummy.clone());
        assert!(f_opt1.type_equality(&f_opt2));
        assert!(!f_req1.type_equality(&f_opt1));

        // Test Boolean values.
        let b_req1 = AV::Boolean(Opt::Required(true), dummy.clone());
        let b_req2 = AV::Boolean(Opt::Required(false), dummy.clone());
        assert!(b_req1.type_equality(&b_req2));

        let b_opt1 = AV::Boolean(Opt::Optional(Some(true)), dummy.clone());
        let b_opt2 = AV::Boolean(Opt::Optional(Some(false)), dummy.clone());
        assert!(b_opt1.type_equality(&b_opt2));
        assert!(!b_req1.type_equality(&b_opt1));
    }

    #[test]
    fn test_type_equality_unknown() {
        let dummy: StructuredPath = parse_quote!(dummy);
        let unknown = AV::Unknown(dummy.clone());
        let s_val = AV::String(Opt::Required("test".to_string()), dummy.clone());
        // Unknown always equals any type.
        assert!(unknown.type_equality(&s_val));
        assert!(s_val.type_equality(&unknown));
    }

    #[test]
    fn test_type_equality_table() {
        let dummy: StructuredPath = parse_quote!(dummy);

        // Two tables with the same field.
        let mut fields1 = BTreeMap::new();
        let mut fields2 = BTreeMap::new();
        fields1.insert(
            "a".to_string(),
            AnalyzedValue::String(Optionality::Required("x".to_string()), dummy.clone()),
        );
        fields2.insert(
            "a".to_string(),
            AnalyzedValue::String(Optionality::Required("y".to_string()), dummy.clone()),
        );
        let table1 = AnalyzedTable {
            fields: fields1,
            path: dummy.clone(),
            additional_fields: false,
        };
        let table2 = AnalyzedTable {
            fields: fields2,
            path: dummy.clone(),
            additional_fields: false,
        };
        let val_table1 = AnalyzedValue::Table(Optionality::Required(table1.clone()));
        let val_table2 = AnalyzedValue::Table(Optionality::Required(table2.clone()));
        assert!(val_table1.type_equality(&val_table2));

        // One table missing a field but allowing additional fields.
        let mut fields3 = BTreeMap::new();
        fields3.insert(
            "a".to_string(),
            AnalyzedValue::String(Optionality::Required("x".to_string()), dummy.clone()),
        );
        let table3 = AnalyzedTable {
            fields: fields3,
            path: dummy.clone(),
            additional_fields: false,
        };
        let table4 = AnalyzedTable {
            fields: BTreeMap::new(),
            path: dummy.clone(),
            additional_fields: true,
        };
        let val_table3 = AnalyzedValue::Table(Optionality::Required(table3));
        let val_table4 = AnalyzedValue::Table(Optionality::Required(table4));
        assert!(val_table3.type_equality(&val_table4));

        // Same missing field but additional_fields is false should not match.
        let table5 = AnalyzedTable {
            fields: BTreeMap::new(),
            path: dummy.clone(),
            additional_fields: false,
        };
        let val_table5 = AnalyzedValue::Table(Optionality::Required(table5));
        assert!(!val_table3.type_equality(&val_table5));
    }

    #[test]
    fn test_type_equality_array() {
        let dummy: StructuredPath = parse_quote!(dummy);

        // Test Tuple arrays with equal lengths and matching element types.
        let tuple1 = AnalyzedArray::Tuple(
            vec![
                AnalyzedValue::Integer(Optionality::Required(1), dummy.clone()),
                AnalyzedValue::Integer(Optionality::Required(2), dummy.clone()),
            ],
            dummy.clone(),
        );
        let tuple2 = AnalyzedArray::Tuple(
            vec![
                AnalyzedValue::Integer(Optionality::Required(3), dummy.clone()),
                AnalyzedValue::Integer(Optionality::Required(4), dummy.clone()),
            ],
            dummy.clone(),
        );
        assert!(tuple1.type_equality(&tuple2));

        // Different lengths should fail.
        let tuple3 = AnalyzedArray::Tuple(
            vec![AnalyzedValue::Integer(
                Optionality::Required(1),
                dummy.clone(),
            )],
            dummy.clone(),
        );
        assert!(!tuple1.type_equality(&tuple3));

        // Test Array variants.
        let inner1 = AnalyzedValue::Float(Optionality::Required(1.0), dummy.clone());
        let inner2 = AnalyzedValue::Float(Optionality::Required(2.0), dummy.clone());
        let array1 = AnalyzedArray::Array(Box::new(inner1), dummy.clone());
        let array2 = AnalyzedArray::Array(Box::new(inner2), dummy.clone());
        assert!(array1.type_equality(&array2));

        // Tuple vs Array should return false.
        assert!(!tuple1.type_equality(&array1));
    }
}
