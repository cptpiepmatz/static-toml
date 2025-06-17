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

use super::{AnnotateIr, AnnotatedArray, AnnotatedTable, AnnotatedValue, AnnotatedValueKind};

#[derive(Debug)]
pub struct AnalyzeArgs {
    pub prefer_slices: Option<LitBool>,
    pub optional: Vec<(StructuredPath, Option<TypeHint>)>,
}

pub fn analyze(annotated: AnnotateIr, args: AnalyzeArgs) -> Result<AnalyzeIr, AnalyzeError> {
    let AnalyzeArgs { prefer_slices, optional } = args;
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
pub struct AnalyzedValue {
    pub kind: AnalyzedValueKind,
    pub path: StructuredPath,
}

// Each variant has its own Optionality so we keep the type info even when there's no value.
// If we wrapped the whole enum instead, we'd lose that info when a value is absent.
#[derive(Debug, Clone)]
pub enum AnalyzedValueKind {
    String(Optionality<String>),
    Integer(Optionality<i64>),
    Float(Optionality<f64>),
    Boolean(Optionality<bool>),
    Table(Optionality<AnalyzedTable>),
    Array(Optionality<AnalyzedArray>),
    Unknown,
}

#[derive(Debug, Clone)]
pub struct AnalyzedTable {
    pub fields: BTreeMap<String, AnalyzedValue>,
    pub additional_fields: bool,
}

#[derive(Debug, Clone)]
pub enum AnalyzedArray {
    Tuple(Vec<AnalyzedValue>),
    Array(Vec<AnalyzedValue>),
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
        SegmentIter { path, iter: path.segments.iter() }
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
        Self { root: AnalyzedTable::from_annotated(annotated.root) }
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
            self.root.apply_optionality(segment, &mut iter, type_hint.as_ref())?;
        }

        Ok(())
    }

    fn merge_slices(&mut self) -> Result<(), AnalyzeError> {
        todo!()
    }
}

impl AnalyzedValue {
    fn from_annotated(annotated: AnnotatedValue) -> Self {
        let path = annotated.path;
        match annotated.kind {
            AnnotatedValueKind::String(string) => AnalyzedValue {
                kind: AnalyzedValueKind::String(Optionality::Required(string)),
                path,
            },
            AnnotatedValueKind::Integer(integer) => AnalyzedValue {
                kind: AnalyzedValueKind::Integer(Optionality::Required(integer)),
                path,
            },
            AnnotatedValueKind::Float(float) => {
                AnalyzedValue { kind: AnalyzedValueKind::Float(Optionality::Required(float)), path }
            }
            AnnotatedValueKind::Boolean(boolean) => AnalyzedValue {
                kind: AnalyzedValueKind::Boolean(Optionality::Required(boolean)),
                path,
            },
            AnnotatedValueKind::Table(table) => AnalyzedValue {
                kind: AnalyzedValueKind::Table(Optionality::Required(
                    AnalyzedTable::from_annotated(table),
                )),
                path,
            },
            AnnotatedValueKind::Array(array) => AnalyzedValue {
                kind: AnalyzedValueKind::Array(Optionality::Required(
                    AnalyzedArray::from_annotated(array),
                )),
                path,
            },
        }
    }

    fn apply_optionality(
        &mut self,
        iter: &mut SegmentIter,
        type_hint: Option<&TypeHint>,
    ) -> Result<(), AnalyzeError> {
        use AnalyzedValueKind as AVK;

        let segment = iter.next();

        let key_of_primitive_err = |segment: &StructuredPathSegment| {
            Err(AnalyzeError::KeyOfPrimitive(iter.path.clone(), segment.clone()))
        };

        match (segment, &mut self.kind) {
            (Some(segment), AVK::String(_)) => return key_of_primitive_err(segment),
            (Some(segment), AVK::Integer(_)) => return key_of_primitive_err(segment),
            (Some(segment), AVK::Float(_)) => return key_of_primitive_err(segment),
            (Some(segment), AVK::Boolean(_)) => return key_of_primitive_err(segment),
            (Some(segment), AVK::Table(opt)) => match opt {
                Optionality::Optional(None) => {}
                Optionality::Required(table) | Optionality::Optional(Some(table)) => {
                    table.apply_optionality(segment, iter, type_hint)?
                }
            },
            (Some(segment), AVK::Array(opt)) => match opt {
                Optionality::Optional(None) => {}
                Optionality::Required(array) | Optionality::Optional(Some(array)) => {
                    array.apply_optionality(segment, iter, type_hint)?
                }
            },
            (None, AVK::String(opt)) => opt.make_optional(),
            (None, AVK::Integer(opt)) => opt.make_optional(),
            (None, AVK::Float(opt)) => opt.make_optional(),
            (None, AVK::Boolean(opt)) => opt.make_optional(),
            (None, AVK::Table(opt)) => opt.make_optional(),
            (None, AVK::Array(opt)) => opt.make_optional(),
            (_, AVK::Unknown) => {}
        }

        Ok(())
    }

    fn type_equality(&self, other: &Self) -> bool {
        use AnalyzedValue as AV;
        use AnalyzedValueKind as AVK;
        match (&self.kind, &other.kind) {
            (AVK::Unknown, _) => true,
            (_, AVK::Unknown) => true,
            (AVK::String(left), AVK::String(right)) => match (left, right) {
                (Optionality::Required(_), Optionality::Required(_)) => true,
                (Optionality::Optional(_), Optionality::Optional(_)) => true,
                _ => false,
            },
            (AVK::Integer(left), AVK::Integer(right)) => match (left, right) {
                (Optionality::Required(_), Optionality::Required(_)) => true,
                (Optionality::Optional(_), Optionality::Optional(_)) => true,
                _ => false,
            },
            (AVK::Float(left), AVK::Float(right)) => match (left, right) {
                (Optionality::Required(_), Optionality::Required(_)) => true,
                (Optionality::Optional(_), Optionality::Optional(_)) => true,
                _ => false,
            },
            (AVK::Boolean(left), AVK::Boolean(right)) => match (left, right) {
                (Optionality::Required(_), Optionality::Required(_)) => true,
                (Optionality::Optional(_), Optionality::Optional(_)) => true,
                _ => false,
            },
            (AVK::Table(left), AVK::Table(right)) => match (left, right) {
                (Optionality::Required(left), Optionality::Required(right))
                | (Optionality::Optional(Some(left)), Optionality::Optional(Some(right))) => {
                    left.type_equality(right)
                }
                (Optionality::Optional(None), Optionality::Optional(_)) => true,
                (Optionality::Optional(_), Optionality::Optional(None)) => true,
                _ => false,
            },
            (AVK::Array(left), AVK::Array(right)) => match (left, right) {
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

    fn merge_arrays(&mut self) {
        use AnalyzedValueKind as AVK;

        match &mut self.kind {
            AVK::Table(Optionality::Required(table) | Optionality::Optional(Some(table))) => {
                table.merge_arrays();
            }
            AVK::Array(Optionality::Required(array) | Optionality::Optional(Some(array))) => {
                array.merge_arrays();
            }
            _ => {}
        }
    }

    fn union_with(&self, other: &Self) -> Result<Self, AnalyzeError> {
        use AnalyzedValueKind as AVK;

        let path = self.path.clone();
        let kind = match (&self.kind, &other.kind) {
            (_, AVK::Unknown) => self.kind.clone(),
            (AVK::String(_), AVK::String(_))
            | (AVK::Integer(_), AVK::Integer(_))
            | (AVK::Float(_), AVK::Float(_))
            | (AVK::Boolean(_), AVK::Boolean(_)) => self.kind.clone(),
            (
                AVK::Table(Optionality::Required(this)),
                AVK::Table(Optionality::Required(that) | Optionality::Optional(Some(that))),
            ) => AVK::Table(Optionality::Required(this.union_with(that)?)),
            (
                AVK::Table(Optionality::Optional(Some(this))),
                AVK::Table(Optionality::Required(that) | Optionality::Optional(Some(that))),
            ) => AVK::Table(Optionality::Optional(Some(this.union_with(that)?))),
            (AVK::Table(Optionality::Optional(None)), AVK::Table(_)) => self.kind.clone(),
            (AVK::Array(_), AVK::Array(_)) => todo!(),
            (
                AVK::Unknown,
                AVK::String(Optionality::Required(_))
                | AVK::Integer(Optionality::Required(_))
                | AVK::Float(Optionality::Required(_))
                | AVK::Boolean(Optionality::Required(_))
                | AVK::Table(Optionality::Required(_))
                | AVK::Array(Optionality::Required(_)),
            ) => return Err(AnalyzeError::TypeUnionConflict(path, other.path.clone())),
            (AVK::Unknown, AVK::String(Optionality::Optional(_))) => {
                AVK::String(Optionality::Optional(None))
            }
            (AVK::Unknown, AVK::Integer(Optionality::Optional(_))) => {
                AVK::Integer(Optionality::Optional(None))
            }
            (AVK::Unknown, AVK::Float(Optionality::Optional(_))) => {
                AVK::Float(Optionality::Optional(None))
            }
            (AVK::Unknown, AVK::Boolean(Optionality::Optional(_))) => {
                AVK::Boolean(Optionality::Optional(None))
            }
            (AVK::Unknown, AVK::Table(Optionality::Optional(_))) => {
                AVK::Table(Optionality::Optional(None))
            }
            (AVK::Unknown, AVK::Array(Optionality::Optional(_))) => {
                AVK::Array(Optionality::Optional(None))
            }
            _ => return Err(AnalyzeError::TypeUnionConflict(path, other.path.clone())),
        };

        Ok(Self { kind, path })
    }
}

impl AnalyzedTable {
    fn from_annotated(annotated: AnnotatedTable) -> Self {
        let fields = annotated
            .0
            .into_iter()
            .map(|(key, value)| (key, AnalyzedValue::from_annotated(value)))
            .collect();

        Self { fields, additional_fields: false }
    }

    fn apply_optionality(
        &mut self,
        segment: &StructuredPathSegment,
        iter: &mut SegmentIter,
        type_hint: Option<&TypeHint>,
    ) -> Result<(), AnalyzeError> {
        use AnalyzedValue as AV;
        use AnalyzedValueKind as AVK;

        match segment {
            StructuredPathSegment::Index(..) => {
                return Err(AnalyzeError::UnmatchedField(iter.path.clone(), segment.clone()));
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
                            AV { kind: AVK::String(Optionality::Optional(None)), path },
                        ),
                        Some(TypeHint::Integer) => self.fields.insert(
                            key,
                            AV { kind: AVK::Integer(Optionality::Optional(None)), path },
                        ),
                        Some(TypeHint::Float) => self.fields.insert(
                            key,
                            AV { kind: AVK::Float(Optionality::Optional(None)), path },
                        ),
                        Some(TypeHint::Boolean) => self.fields.insert(
                            key,
                            AV { kind: AVK::Boolean(Optionality::Optional(None)), path },
                        ),
                        None => self.fields.insert(key, AV { kind: AVK::Unknown, path }),
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

    fn merge_arrays(&mut self) {
        self.fields.values_mut().for_each(AnalyzedValue::merge_arrays);
    }

    fn union_with(&self, other: &Self) -> Result<Self, AnalyzeError> {
        todo!()
    }
}

impl AnalyzedArray {
    fn from_annotated(annotated: AnnotatedArray) -> Self {
        Self::Tuple(annotated.0.into_iter().map(AnalyzedValue::from_annotated).collect())
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
                return Err(AnalyzeError::UnmatchedField(iter.path.clone(), segment.clone()));
            }
        };

        // we have no `Array` variants at this point
        if let Self::Tuple(items) = self {
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
            (Self::Tuple(..), Self::Array(..)) | (Self::Array(..), Self::Tuple(..)) => false,
            (Self::Tuple(left), Self::Tuple(right))
            | (Self::Array(left), Self::Array(right)) => {
                if left.len() != right.len() {
                    return false;
                }

                left.iter().zip(right.iter()).all(|(left, right)| left.type_equality(right))
            }
        }
    }

    fn merge_arrays(&mut self) {
        match self {
            Self::Tuple(items) | Self::Array(items) => {
                items.iter_mut().for_each(AnalyzedValue::merge_arrays)
            }
        }

        if let Self::Tuple(items) = self {
            let mut type_equal = true;
            'outer: for a in items.iter() {
                for b in items.iter() {
                    if !a.type_equality(b) {
                        type_equal = false;
                        break 'outer;
                    }
                }
            }

            if type_equal {
                todo!("construct type unions");
            }
        }
    }

    fn union_with(&self, other: &Self) -> Result<Self, AnalyzeError> {
        todo!()
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
    use AnalyzedValueKind as AVK;
    use Optionality as Opt;

    impl AnalyzedValue {
        pub fn find_by_path<'v>(
            &'v self,
            target_path: &StructuredPath,
        ) -> Option<&'v AnalyzedValue> {
            let path = &self.path;
            match self.kind {
                _ if path == target_path => Some(self),

                AVK::String(_)
                | AVK::Integer(_)
                | AVK::Float(_)
                | AVK::Boolean(_)
                | AVK::Unknown => None,

                (AVK::Table(
                    Optionality::Required(ref table) | Optionality::Optional(Some(ref table)),
                )) => {
                    for value in table.fields.values() {
                        if let found @ Some(_) = value.find_by_path(target_path) {
                            return found;
                        }
                    }

                    None
                }

                AVK::Array(
                    Optionality::Required(ref array) | Optionality::Optional(Some(ref array)),
                ) => {
                    match array {
                        AA::Tuple(values) | AA::Array(values) => {
                            for value in values {
                                if let found @ Some(_) = value.find_by_path(target_path) {
                                    return found;
                                }
                            }
                        }
                    }

                    None
                }

                _ => None,
            }
        }
    }

    macro_rules! impl_optionality_from {
        // Case with a custom conversion function
        ($from:ty => $variant:ident::<$to:ty>, $convert:expr) => {
            impl From<$from> for Optionality<$to> {
                fn from(value: $from) -> Self {
                    Optionality::$variant($convert(value))
                }
            }
        };

        // Default case using `.into()`
        ($from:ty => $variant:ident::<$to:ty>) => {
            impl_optionality_from!($from => $variant::<$to>, Into::into);
        };
    }

    impl_optionality_from!(&str => Required::<String>);
    impl_optionality_from!(String => Required::<String>);
    impl_optionality_from!(i64 => Required::<i64>);
    impl_optionality_from!(f64 => Required::<f64>);
    impl_optionality_from!(bool => Required::<bool>);
    impl_optionality_from!(AnalyzedTable => Required::<AnalyzedTable>);
    impl_optionality_from!(AnalyzedArray => Required::<AnalyzedArray>);
    impl_optionality_from!(Option<&str> => Optional::<String>, |s: Option<&str>| s.map(String::from));
    impl_optionality_from!(Option<String> => Optional::<String>);
    impl_optionality_from!(Option<i64> => Optional::<i64>);
    impl_optionality_from!(Option<f64> => Optional::<f64>);
    impl_optionality_from!(Option<bool> => Optional::<bool>);
    impl_optionality_from!(Option<AnalyzedTable> => Optional::<AnalyzedTable>);
    impl_optionality_from!(Option<AnalyzedArray> => Optional::<AnalyzedArray>);

    macro_rules! av {
        ($var:ident$(($value:expr))?$(, $path:expr)?) => {{
            AV {
                kind: AVK::$var$(($value.into()))?,
                path: parse_quote!($($path)?)
            }
        }}
    }

    macro_rules! aa {
        ($var:ident[$($item:expr),*]) => {{
            AA::$var(vec![$($item),*])
        }}
    }

    macro_rules! assert_optionality {
        ($table:expr, $query:expr, Required($ty:ident)) => {{
            let query_path = parse_quote!($query);
            let table: &AV = $table;
            match table.find_by_path(&query_path) {
                Some(AV { kind: AVK::$ty(Opt::Required(_)), .. }) => (),
                _ => panic!(
                    "Expected required {} value at {:?}",
                    stringify!($ty),
                    stringify!($query)
                ),
            }
        }};
        ($table:expr, $query:expr, Optional(Unknown)) => {{
            let query_path = parse_quote!($query);
            let table: &AV = $table;
            match table.find_by_path(&query_path) {
                Some(AV { kind: AVK::Unknown, .. }) => (),
                _ => panic!("Expected unknown optional value at {:?}", stringify!($query)),
            }
        }};
        ($table:expr, $query:expr, Optional($ty:ident)) => {{
            let query_path = parse_quote!($query);
            let table: &AV = $table;
            match table.find_by_path(&query_path) {
                Some(AV { kind: AVK::$ty(Opt::Optional(_)), .. }) => (),
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
        let root = av!(Table(ir.root));
        assert_optionality!(&root, title, Required(String));
        assert_optionality!(&root, owner.name, Required(String));
        assert_optionality!(&root, database.enabled, Required(Boolean));
        assert_optionality!(&root, database.ports, Required(Array));
        assert_optionality!(&root, database.ports[0], Required(Integer));
        assert_optionality!(&root, database.ports[1], Required(Integer));
        assert_optionality!(&root, database.ports[2], Required(Integer));
        assert_optionality!(&root, database.data, Required(Array));
        assert_optionality!(&root, database.data.0, Required(Array));
        assert_optionality!(&root, database.data.0[0], Required(String));
        assert_optionality!(&root, database.data.0[1], Required(String));
        assert_optionality!(&root, database.data.1[0], Required(Float));
        assert_optionality!(&root, database.temp_targets.cpu, Required(Float));
        assert_optionality!(&root, database.temp_targets.case, Required(Float));
        assert_optionality!(&root, servers, Required(Table));
        assert_optionality!(&root, servers.alpha, Required(Table));
        assert_optionality!(&root, servers.alpha.ip, Required(String));
        assert_optionality!(&root, servers.alpha.role, Required(String));
        assert_optionality!(&root, servers.beta, Required(Table));
        assert_optionality!(&root, servers.beta.ip, Required(String));
        assert_optionality!(&root, servers.beta.role, Required(String));
    }

    #[test]
    fn apply_optionality_plain() {
        let mut ir = example_initial_analyze_ir();
        AnalyzeIr::apply_optionality(&mut ir, &[(parse_quote!(title), None)])
            .expect("should apply optionality");
        let root = av!(Table(ir.root));
        assert_optionality!(&root, title, Optional(String));
        assert_optionality!(&root, owner.name, Required(String));
        assert_optionality!(&root, database.enabled, Required(Boolean));
        assert_optionality!(&root, database.ports, Required(Array));
        assert_optionality!(&root, database.ports[0], Required(Integer));
        assert_optionality!(&root, database.ports[1], Required(Integer));
        assert_optionality!(&root, database.ports[2], Required(Integer));
        assert_optionality!(&root, database.data, Required(Array));
        assert_optionality!(&root, database.data.0, Required(Array));
        assert_optionality!(&root, database.data.0[0], Required(String));
        assert_optionality!(&root, database.data.0[1], Required(String));
        assert_optionality!(&root, database.data.1[0], Required(Float));
        assert_optionality!(&root, database.temp_targets.cpu, Required(Float));
        assert_optionality!(&root, database.temp_targets.case, Required(Float));
        assert_optionality!(&root, servers, Required(Table));
        assert_optionality!(&root, servers.alpha, Required(Table));
        assert_optionality!(&root, servers.alpha.ip, Required(String));
        assert_optionality!(&root, servers.alpha.role, Required(String));
        assert_optionality!(&root, servers.beta, Required(Table));
        assert_optionality!(&root, servers.beta.ip, Required(String));
        assert_optionality!(&root, servers.beta.role, Required(String));
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
        let root = av!(Table(ir.root));
        assert_optionality!(&root, title, Required(String));
        assert_optionality!(&root, owner.name, Optional(String));
        assert_optionality!(&root, database.enabled, Optional(Boolean));
        assert_optionality!(&root, database.ports, Required(Array));
        assert_optionality!(&root, database.ports[0], Required(Integer));
        assert_optionality!(&root, database.ports[1], Optional(Integer));
        assert_optionality!(&root, database.ports[2], Required(Integer));
        assert_optionality!(&root, database.data, Required(Array));
        assert_optionality!(&root, database.data.0, Required(Array));
        assert_optionality!(&root, database.data.0[0], Required(String));
        assert_optionality!(&root, database.data.0[1], Required(String));
        assert_optionality!(&root, database.data.1[0], Required(Float));
        assert_optionality!(&root, database.temp_targets.cpu, Optional(Float));
        assert_optionality!(&root, database.temp_targets.case, Required(Float));
        assert_optionality!(&root, servers, Required(Table));
        assert_optionality!(&root, servers.alpha, Required(Table));
        assert_optionality!(&root, servers.alpha.ip, Required(String));
        assert_optionality!(&root, servers.alpha.role, Required(String));
        assert_optionality!(&root, servers.beta, Optional(Table));
        assert_optionality!(&root, servers.beta.ip, Required(String));
        assert_optionality!(&root, servers.beta.role, Required(String));
    }

    #[test]
    fn apply_optionality_range() {
        let mut ir = example_initial_analyze_ir();
        AnalyzeIr::apply_optionality(
            &mut ir,
            &[(parse_quote!(database.ports[..=1]), None), (parse_quote!(database.data[]), None)],
        )
        .expect("should apply optionality");
        let root = av!(Table(ir.root));
        assert_optionality!(&root, title, Required(String));
        assert_optionality!(&root, owner.name, Required(String));
        assert_optionality!(&root, database.enabled, Required(Boolean));
        assert_optionality!(&root, database.ports, Required(Array));
        assert_optionality!(&root, database.ports[0], Optional(Integer));
        assert_optionality!(&root, database.ports[1], Optional(Integer));
        assert_optionality!(&root, database.ports[2], Required(Integer));
        assert_optionality!(&root, database.data, Required(Array));
        assert_optionality!(&root, database.data.0, Optional(Array));
        assert_optionality!(&root, database.data.0[0], Required(String));
        assert_optionality!(&root, database.data.0[1], Required(String));
        assert_optionality!(&root, database.data.1[0], Required(Float));
        assert_optionality!(&root, database.temp_targets.cpu, Required(Float));
        assert_optionality!(&root, database.temp_targets.case, Required(Float));
        assert_optionality!(&root, servers, Required(Table));
        assert_optionality!(&root, servers.alpha, Required(Table));
        assert_optionality!(&root, servers.alpha.ip, Required(String));
        assert_optionality!(&root, servers.alpha.role, Required(String));
        assert_optionality!(&root, servers.beta, Required(Table));
        assert_optionality!(&root, servers.beta.ip, Required(String));
        assert_optionality!(&root, servers.beta.role, Required(String));
    }

    #[test]
    fn apply_optionality_add_new_entries() {
        let mut ir = example_initial_analyze_ir();
        AnalyzeIr::apply_optionality(
            &mut ir,
            &[
                (parse_quote!(subtitle), None),
                (parse_quote!(description), Some(TypeHint::String)),
                (parse_quote!(database.temp_targets.gpu), Some(TypeHint::Float)),
            ],
        )
        .expect("should apply optionality");
        let root = av!(Table(ir.root));
        assert_optionality!(&root, title, Required(String));
        assert_optionality!(&root, subtitle, Optional(Unknown));
        assert_optionality!(&root, description, Optional(String));
        assert_optionality!(&root, owner.name, Required(String));
        assert_optionality!(&root, database.enabled, Required(Boolean));
        assert_optionality!(&root, database.ports, Required(Array));
        assert_optionality!(&root, database.ports[0], Required(Integer));
        assert_optionality!(&root, database.ports[1], Required(Integer));
        assert_optionality!(&root, database.ports[2], Required(Integer));
        assert_optionality!(&root, database.data, Required(Array));
        assert_optionality!(&root, database.data.0, Required(Array));
        assert_optionality!(&root, database.data.0[0], Required(String));
        assert_optionality!(&root, database.data.0[1], Required(String));
        assert_optionality!(&root, database.data.1[0], Required(Float));
        assert_optionality!(&root, database.temp_targets.cpu, Required(Float));
        assert_optionality!(&root, database.temp_targets.case, Required(Float));
        assert_optionality!(&root, database.temp_targets.gpu, Optional(Float));
        assert_optionality!(&root, servers, Required(Table));
        assert_optionality!(&root, servers.alpha, Required(Table));
        assert_optionality!(&root, servers.alpha.ip, Required(String));
        assert_optionality!(&root, servers.alpha.role, Required(String));
        assert_optionality!(&root, servers.beta, Required(Table));
        assert_optionality!(&root, servers.beta.ip, Required(String));
        assert_optionality!(&root, servers.beta.role, Required(String));
    }

    #[test]
    fn apply_optionality_wildcard() {
        let mut ir = example_initial_analyze_ir();
        AnalyzeIr::apply_optionality(&mut ir, &[(parse_quote!(database.temp_targets.*), None)])
            .expect("should apply optionality");
        let this = &av!(Table(ir.root));

        let temp_targets =
            this.find_by_path(&parse_quote!(database.temp_targets)).expect("should exist");

        let AV { kind: AVK::Table(Opt::Required(temp_targets)), .. } = temp_targets else {
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
        // Test String values.
        let s_req1 = av!(String("hello"));
        let s_req2 = av!(String("world"));
        assert!(s_req1.type_equality(&s_req2));

        let s_opt1 = av!(String(Some("hello")));
        let s_opt2 = av!(String(Some("world")));
        assert!(s_opt1.type_equality(&s_opt2));
        assert!(!s_req1.type_equality(&s_opt1));

        // Test Integer values.
        let i_req1 = av!(Integer(10));
        let i_req2 = av!(Integer(20));
        assert!(i_req1.type_equality(&i_req2));

        let i_opt1 = av!(Integer(Some(10)));
        let i_opt2 = av!(Integer(Some(20)));
        assert!(i_opt1.type_equality(&i_opt2));
        assert!(!i_req1.type_equality(&i_opt1));

        // Test Float values.
        let f_req1 = av!(Float(1.0));
        let f_req2 = av!(Float(2.0));
        assert!(f_req1.type_equality(&f_req2));

        let f_opt1 = av!(Float(Some(1.0)));
        let f_opt2 = av!(Float(Some(2.0)));
        assert!(f_opt1.type_equality(&f_opt2));
        assert!(!f_req1.type_equality(&f_opt1));

        // Test Boolean values.
        let b_req1 = av!(Boolean(true));
        let b_req2 = av!(Boolean(false));
        assert!(b_req1.type_equality(&b_req2));

        let b_opt1 = av!(Boolean(Some(true)));
        let b_opt2 = av!(Boolean(Some(false)));
        assert!(b_opt1.type_equality(&b_opt2));
        assert!(!b_req1.type_equality(&b_opt1));
    }

    #[test]
    fn test_type_equality_unknown() {
        let unknown = av!(Unknown);
        let s_val = av!(String("test"));

        // Unknown always equals any type.
        assert!(unknown.type_equality(&s_val));
        assert!(s_val.type_equality(&unknown));
    }

    #[test]
    fn test_type_equality_table() {
        // Two tables with the same field.
        let mut fields1 = BTreeMap::new();
        let mut fields2 = BTreeMap::new();
        fields1.insert("a".to_string(), av!(String("x"), a));
        fields2.insert("a".to_string(), av!(String("y"), a));
        let table1 = AT { fields: fields1, additional_fields: false };
        let table2 = AT { fields: fields2, additional_fields: false };
        let val_table1 = av!(Table(table1));
        let val_table2 = av!(Table(table2));
        assert!(val_table1.type_equality(&val_table2));

        // One table missing a field but allowing additional fields.
        let mut fields3 = BTreeMap::new();
        fields3.insert("a".to_string(), av!(String("x"), a));
        let table3 = AT { fields: fields3, additional_fields: false };
        let table4 = AT { fields: BTreeMap::new(), additional_fields: true };
        let val_table3 = av!(Table(table3));
        let val_table4 = av!(Table(table4));
        assert!(val_table3.type_equality(&val_table4));

        // Same missing field but additional_fields is false should not match.
        let table5 = AT { fields: BTreeMap::new(), additional_fields: false };
        let val_table5 = av!(Table(table5));
        assert!(!val_table3.type_equality(&val_table5));
    }

    #[test]
    fn test_type_equality_array() {
        // Test Tuple arrays with equal lengths and matching element types.
        let tuple1 = aa!(Tuple[av!(Integer(1)), av!(Integer(2))]);
        let tuple2 = aa!(Tuple[av!(Integer(3)), av!(Integer(4))]);
        assert!(tuple1.type_equality(&tuple2));

        // Different lengths should fail.
        let tuple3 = aa!(Tuple[av!(Integer(1))]);
        assert!(!tuple1.type_equality(&tuple3));

        // Test Array variants.
        let array1 = aa!(Array[av!(Float(1.0))]);
        let array2 = aa!(Array[av!(Float(2.0))]);
        assert!(array1.type_equality(&array2));

        // Tuple vs Array should return false.
        assert!(!tuple1.type_equality(&array1));
    }
}
