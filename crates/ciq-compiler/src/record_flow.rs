//! Reconstruct mutable record views from reference identity and CFG effects.
use crate::{
    BlockId, BuilderId, LocalId, LocatedOperation, Operation, ParameterType, ValueId, api::Type,
};
use std::collections::{BTreeMap, BTreeSet};

type Roots = BTreeSet<Root>;
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Root {
    Builder(BuilderId),
    Parameter(usize),
    Value(ValueId),
    Child(Box<Root>, String),
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct State {
    values: BTreeMap<ValueId, Roots>,
    locals: BTreeMap<LocalId, Roots>,
    parameters: BTreeMap<usize, Roots>,
    fields: BTreeMap<String, Roots>,
    heap: BTreeMap<Root, BTreeMap<String, Type>>,
    children: BTreeMap<(Root, String), Roots>,
    other_locals: BTreeMap<LocalId, Vec<Type>>,
    writes: BTreeMap<String, Type>,
    erased: bool,
    block: Option<BlockId>,
    edges: BTreeMap<BlockId, Roots>,
}

pub(crate) fn has_view(ty: &Type) -> bool {
    crate::unions::alternatives(ty)
        .iter()
        .any(|t| matches!(t, Type::Options(_)))
}
fn schema(ty: &Type) -> Option<BTreeMap<String, Type>> {
    let mut result = BTreeMap::new();
    let mut found = false;
    for part in crate::unions::alternatives(ty) {
        if let Type::Options(fields) = part {
            found = true;
            for (key, ty) in fields {
                result
                    .entry(key.clone())
                    .and_modify(|old| *old = join(old, ty))
                    .or_insert_with(|| ty.clone());
            }
        }
    }
    for (key, value) in &mut result {
        if crate::unions::alternatives(ty)
            .iter()
            .any(|part| matches!(part,Type::Options(fields) if !fields.contains_key(key)))
        {
            *value = Type::Any;
        }
    }
    found.then_some(result)
}
fn other_parts(ty: &Type) -> Vec<Type> {
    crate::unions::alternatives(ty)
        .iter()
        .filter(|t| !matches!(t, Type::Options(_)))
        .cloned()
        .collect()
}
fn join(a: &Type, b: &Type) -> Type {
    let ty = crate::unions::normalize(vec![a.clone(), b.clone()]);
    if crate::structural::type_depth(&ty, 1) && crate::unions::result(ty.clone()).is_some() {
        ty
    } else {
        Type::Any
    }
}
fn merge_refs<K: Ord + Clone>(to: &mut BTreeMap<K, Roots>, from: &BTreeMap<K, Roots>) {
    for (key, roots) in from {
        to.entry(key.clone())
            .or_default()
            .extend(roots.iter().cloned());
    }
}
impl State {
    pub(crate) fn active(&self) -> bool {
        !self.heap.is_empty()
    }
    pub(crate) fn set_builder(&mut self, builder: BuilderId, fields: BTreeMap<String, Type>) {
        self.heap.insert(Root::Builder(builder), fields);
    }
    pub(crate) fn new(parameters: &[ParameterType]) -> Self {
        let mut state = Self::default();
        for (index, ty) in parameters.iter().enumerate() {
            let roots = state.seed(Root::Parameter(index), &ty.api_type());
            if !roots.is_empty() {
                state.parameters.insert(index, roots);
            }
        }
        state
    }
    fn seed(&mut self, root: Root, ty: &Type) -> Roots {
        if let Some(mut fields) = schema(ty) {
            if !matches!(root, Root::Builder(_)) {
                for (key, value) in &self.writes {
                    fields
                        .entry(key.clone())
                        .and_modify(|old| *old = join(old, value))
                        .or_insert(Type::Any);
                }
                if self.erased {
                    for ty in fields.values_mut() {
                        *ty = Type::Any;
                    }
                }
            }
            self.heap.entry(root.clone()).or_insert(fields);
            BTreeSet::from([root])
        } else {
            Roots::new()
        }
    }
    pub(crate) fn note_result(&mut self, value: ValueId, ty: &Type) {
        if !self.values.get(&value).is_some_and(|r| !r.is_empty()) {
            let roots = self.seed(Root::Value(value), ty);
            if !roots.is_empty() {
                self.values.insert(value, roots);
            }
        }
    }
    fn merge(&mut self, other: &Self) {
        merge_refs(&mut self.values, &other.values);
        merge_refs(&mut self.edges, &other.edges);
        merge_refs(&mut self.locals, &other.locals);
        merge_refs(&mut self.parameters, &other.parameters);
        merge_refs(&mut self.fields, &other.fields);
        merge_refs(&mut self.children, &other.children);
        self.erased |= other.erased;
        for (key, value) in &other.writes {
            self.writes
                .entry(key.clone())
                .and_modify(|old| *old = join(old, value))
                .or_insert_with(|| value.clone());
        }
        for (local, values) in &other.other_locals {
            let target = self.other_locals.entry(*local).or_default();
            for value in values {
                if !target.contains(value) {
                    target.push(value.clone());
                }
            }
        }
        for (root, fields) in &other.heap {
            if let Some(target) = self.heap.get_mut(root) {
                let keys: BTreeSet<_> = target.keys().chain(fields.keys()).cloned().collect();
                for key in keys {
                    let ty = match (target.get(&key), fields.get(&key)) {
                        (Some(a), Some(b)) => join(a, b),
                        (Some(a), None) | (None, Some(a)) if matches!(root, Root::Builder(_)) => {
                            a.clone()
                        }
                        _ => Type::Any,
                    };
                    target.insert(key, ty);
                }
            } else {
                self.heap.insert(root.clone(), fields.clone());
            }
        }
    }

    fn view(&self, roots: &Roots, depth: usize) -> Option<Type> {
        if roots.is_empty() || depth > 64 {
            return None;
        }
        let mut fields = BTreeMap::new();
        for root in roots {
            for (key, ty) in self.heap.get(root)? {
                let mut ty = ty.clone();
                if schema(&ty).is_some() {
                    if let Some(children) = self.children.get(&(root.clone(), key.clone())) {
                        if let Some(child) = self.view(children, depth + 1) {
                            ty = if crate::unions::alternatives(&ty).contains(&Type::Null) {
                                child.with_null()
                            } else {
                                child
                            };
                        }
                    }
                }
                fields
                    .entry(key.clone())
                    .and_modify(|old| *old = join(old, &ty))
                    .or_insert(ty);
            }
        }
        for (key, ty) in &mut fields {
            if roots.iter().any(|root| {
                !matches!(root, Root::Builder(_))
                    && self
                        .heap
                        .get(root)
                        .is_some_and(|schema| !schema.contains_key(key))
            }) {
                *ty = Type::Any;
            }
        }
        if crate::records::validate(&fields).is_err() {
            Some(Type::Any)
        } else {
            Some(Type::Options(fields))
        }
    }
    pub(crate) fn value_type(&self, value: ValueId, baseline: &Type) -> Type {
        if schema(baseline).is_none() {
            return baseline.clone();
        }
        self.values
            .get(&value)
            .and_then(|roots| self.view(roots, 0))
            .map(|ty| {
                if crate::unions::alternatives(baseline).contains(&Type::Null) {
                    ty.with_null()
                } else {
                    ty
                }
            })
            .unwrap_or_else(|| baseline.clone())
    }
    pub(crate) fn local_type(&self, local: LocalId) -> Option<Type> {
        let record = self
            .locals
            .get(&local)
            .and_then(|roots| self.view(roots, 0))?;
        Some(crate::unions::normalize(
            std::iter::once(record)
                .chain(self.other_locals.get(&local).into_iter().flatten().cloned())
                .collect(),
        ))
    }
    fn invalidate(&mut self) {
        self.erased = true;
        for (root, fields) in &mut self.heap {
            if matches!(root, Root::Builder(_)) && !self.values.values().any(|r| r.contains(root)) {
                continue;
            }
            for ty in fields.values_mut() {
                *ty = Type::Any;
            }
        }
        self.children.clear();
        self.fields.clear();
    }
    /// Apply an operation after its ordinary types have been checked. Unknown
    /// calls erase mutable field knowledge; they never restore entry schemas.
    pub(crate) fn observe(
        &mut self,
        op: &Operation,
        types: &BTreeMap<ValueId, Type>,
        symbols: &BTreeMap<ValueId, String>,
        weak: bool,
    ) {
        if let Operation::LocalVariable { local, value } = op {
            self.other_locals.insert(
                *local,
                value
                    .and_then(|v| types.get(&v))
                    .map(other_parts)
                    .unwrap_or_default(),
            );
        }
        if let Operation::LocalInitialize { local, value }
        | Operation::LocalAssign { local, value } = op
        {
            self.other_locals.insert(
                *local,
                types.get(value).map(other_parts).unwrap_or_default(),
            );
        }
        let refs = |value: &ValueId| self.values.get(value).cloned().unwrap_or_default();
        let mut result = None;
        let mut roots = Roots::new();
        if self.heap.is_empty()
            && !matches!(
                op,
                Operation::RecordBegin { .. }
                    | Operation::SelfFieldRead { .. }
                    | Operation::MemberRead { .. }
                    | Operation::GlobalCall { .. }
                    | Operation::ModuleCall { .. }
                    | Operation::SelfCall { .. }
                    | Operation::ObjectCall { .. }
            )
        {
            return;
        }
        match op {
            Operation::Block { block } => self.block = Some(*block),
            Operation::JumpValue {
                target: block,
                value,
            } => {
                let roots = refs(value);
                self.edges.insert(*block, roots);
            }
            Operation::BlockParameter { result: r, .. } => {
                result = Some(*r);
                roots = self
                    .block
                    .and_then(|b| self.edges.get(&b))
                    .cloned()
                    .unwrap_or_default();
            }
            Operation::RecordBegin {
                builder, fields, ..
            } => {
                if weak {
                    let target = self.heap.entry(Root::Builder(*builder)).or_default();
                    for (key, ty) in fields {
                        target
                            .entry(key.clone())
                            .and_modify(|old| *old = join(old, ty))
                            .or_insert_with(|| ty.clone());
                    }
                } else {
                    self.heap.insert(Root::Builder(*builder), fields.clone());
                }
            }
            Operation::DictionaryFinish { result: r, builder }
                if self.heap.contains_key(&Root::Builder(*builder)) =>
            {
                result = Some(*r);
                roots.insert(Root::Builder(*builder));
            }
            Operation::ParameterRead { result: r, index } => {
                result = Some(*r);
                roots = self.parameters.get(index).cloned().unwrap_or_default();
            }
            Operation::LocalRead { result: r, local }
            | Operation::LocalTake { result: r, local }
            | Operation::VariableRead {
                result: r, local, ..
            } => {
                result = Some(*r);
                roots = self.locals.get(local).cloned().unwrap_or_default();
            }
            Operation::LocalVariable { local, value } => {
                let roots = value.map(|v| refs(&v)).unwrap_or_default();
                self.locals.insert(*local, roots);
            }
            Operation::LocalInitialize { local, value }
            | Operation::LocalAssign { local, value } => {
                let roots = refs(value);
                self.locals.insert(*local, roots);
            }
            Operation::ParameterAssign { index, value } => {
                let roots = refs(value);
                self.parameters.insert(*index, roots);
            }
            Operation::SelfFieldRead {
                result: r,
                field,
                result_type,
                ..
            } => {
                result = Some(*r);
                roots = self.fields.get(field).cloned().unwrap_or_default();
                if roots.is_empty() {
                    roots = self.seed(Root::Value(*r), &result_type.api_type());
                    self.fields.insert(field.clone(), roots.clone());
                }
            }
            Operation::MemberRead {
                result: r,
                field_type,
                ..
            } => {
                result = Some(*r);
                if let Some(ty) = types
                    .get(r)
                    .cloned()
                    .or_else(|| field_type.as_ref().map(|t| t.api_type()))
                {
                    roots = self.seed(Root::Value(*r), &ty);
                }
            }
            Operation::SelfFieldAssign { field, value } => {
                let roots = refs(value);
                self.fields.insert(field.clone(), roots);
            }
            Operation::MemberWrite { .. } => self.invalidate(),
            Operation::DictionaryFill {
                builder, entries, ..
            } => {
                for (key, value) in entries {
                    if let Some(key) = symbols.get(key) {
                        let roots = refs(value);
                        self.children
                            .insert((Root::Builder(*builder), key.clone()), roots);
                    }
                }
            }
            Operation::DictionaryRead {
                result: r,
                dictionary: receiver,
                key,
            }
            | Operation::ArrayRead {
                result: r,
                array: receiver,
                index: key,
            } => {
                result = Some(*r);
                roots = self.read_children(*receiver, *key, types, symbols);
            }
            Operation::NativeCall {
                result: r,
                receiver,
                member,
                arguments,
            } if member == "get" && arguments.len() == 1 => {
                result = Some(*r);
                roots = self.read_children(*receiver, arguments[0], types, symbols);
            }
            Operation::DictionaryWrite {
                dictionary: receiver,
                key,
                value,
            }
            | Operation::ArrayWrite {
                array: receiver,
                index: key,
                value,
            } => {
                self.write(*receiver, *key, *value, types, symbols, weak);
            }
            Operation::NativeCallStatement {
                receiver,
                member,
                arguments,
            } if member == "put" && arguments.len() == 2 => {
                self.write(*receiver, arguments[0], arguments[1], types, symbols, weak);
            }
            Operation::NativeCall {
                receiver,
                member,
                arguments,
                ..
            } if member == "remove" && arguments.len() == 1 => {
                if let Some(key) = symbols.get(&arguments[0]) {
                    let roots = refs(receiver);
                    for root in &roots {
                        if let Some(fields) = self.heap.get_mut(root) {
                            let ty = if weak || roots.len() != 1 {
                                fields
                                    .get(key)
                                    .map(|ty| join(ty, &Type::Null))
                                    .unwrap_or(Type::Null)
                            } else {
                                Type::Null
                            };
                            fields.insert(key.clone(), ty);
                        }
                        self.children.remove(&(root.clone(), key.clone()));
                    }
                }
            }
            Operation::GlobalCall {
                result: r,
                result_type,
                ..
            }
            | Operation::ModuleCall {
                result: r,
                result_type,
                ..
            }
            | Operation::SelfCall {
                result: r,
                result_type,
                ..
            }
            | Operation::ObjectCall {
                result: r,
                result_type,
                ..
            } => {
                result = Some(*r);
                self.invalidate();
                // The call's checked result contract is established at return;
                // it does not restore knowledge about earlier aliases.
                if let Some(fields) = schema(&result_type.api_type()) {
                    let root = Root::Value(*r);
                    self.heap.insert(root.clone(), fields);
                    roots.insert(root);
                }
            }
            Operation::GlobalCallStatement { .. }
            | Operation::ModuleCallStatement { .. }
            | Operation::SelfCallStatement { .. }
            | Operation::ObjectCallStatement { .. } => self.invalidate(),
            Operation::NativeCall { .. }
            | Operation::NativeCallStatement { .. }
            | Operation::NativeModuleCall { .. }
            | Operation::NativeModuleCallStatement { .. }
            | Operation::SourceNew { .. }
            | Operation::Println { .. } => self.invalidate(),
            _ => {}
        }
        if let Some(value) = result {
            if roots.is_empty() {
                if let Some(ty) = types.get(&value).cloned().or_else(|| match op {
                    Operation::GlobalCall { result_type, .. }
                    | Operation::ModuleCall { result_type, .. }
                    | Operation::SelfCall { result_type, .. }
                    | Operation::ObjectCall { result_type, .. } => Some(result_type.api_type()),
                    _ => None,
                }) {
                    roots = self.seed(Root::Value(value), &ty);
                }
            }
            self.values.insert(value, roots);
        }
    }
    fn read_children(
        &mut self,
        receiver: ValueId,
        key: ValueId,
        types: &BTreeMap<ValueId, Type>,
        symbols: &BTreeMap<ValueId, String>,
    ) -> Roots {
        let mut result = Roots::new();
        let Some(key) = symbols.get(&key) else {
            return result;
        };
        for parent in self.values.get(&receiver).cloned().unwrap_or_default() {
            let at = (parent.clone(), key.clone());
            if !self.children.contains_key(&at) {
                let ty = self
                    .heap
                    .get(&parent)
                    .and_then(|f| f.get(key))
                    .cloned()
                    .unwrap_or(Type::Any);
                let child = self.seed(Root::Child(Box::new(parent), key.clone()), &ty);
                self.children.insert(at.clone(), child);
            }
            result.extend(self.children[&at].iter().cloned());
        }
        let _ = types;
        result
    }
    fn write(
        &mut self,
        receiver: ValueId,
        key: ValueId,
        value: ValueId,
        types: &BTreeMap<ValueId, Type>,
        symbols: &BTreeMap<ValueId, String>,
        weak: bool,
    ) {
        let roots = self.values.get(&receiver).cloned().unwrap_or_default();
        if roots.is_empty() {
            return;
        }
        let ty = types
            .get(&value)
            .map(|ty| self.value_type(value, ty))
            .unwrap_or(Type::Any);
        let child = self.values.get(&value).cloned().unwrap_or_default();
        if let Some(key) = symbols.get(&key) {
            self.writes
                .entry(key.clone())
                .and_modify(|old| *old = join(old, &ty))
                .or_insert_with(|| ty.clone());
        } else {
            self.erased = true;
        }

        for root in &roots {
            if let Some(fields) = self.heap.get_mut(root) {
                if let Some(key) = symbols.get(&key) {
                    let value = if weak || roots.len() != 1 {
                        fields
                            .get(key)
                            .map(|old| join(old, &ty))
                            .unwrap_or_else(|| ty.clone())
                    } else {
                        ty.clone()
                    };
                    fields.insert(key.clone(), value);
                    if weak || roots.len() != 1 {
                        self.children
                            .entry((root.clone(), key.clone()))
                            .or_default()
                            .extend(child.iter().cloned());
                    } else {
                        self.children
                            .insert((root.clone(), key.clone()), child.clone());
                    }
                } else {
                    for old in fields.values_mut() {
                        *old = join(old, &ty);
                    }
                    self.children.retain(|(parent, _), _| parent != root);
                }
            }
        }
        // Independently obtained external references may denote the same object.
        if roots.iter().any(|r| !matches!(r, Root::Builder(_))) {
            for (root, fields) in &mut self.heap {
                if !roots.contains(root) && !matches!(root, Root::Builder(_)) {
                    for old in fields.values_mut() {
                        *old = join(old, &ty);
                    }
                }
            }
        }
    }
}

/// Check whether a body can expose record memory; scalar-only bodies skip it.
pub(crate) fn needed(ops: &[LocatedOperation], parameters: &[ParameterType]) -> bool {
    fn includes(ty: &Type) -> bool {
        match ty {
            Type::Options(_) => true,
            Type::Array(t) => includes(t),
            Type::Dictionary { key, value } => includes(key) || includes(value),
            Type::Union(parts) | Type::Tuple(parts) => parts.iter().any(includes),
            Type::Structural(shape) => {
                shape.fields.values().any(|ty| includes(&ty.api_type()))
                    || shape.methods.values().any(|method| {
                        includes(&method.result.api_type())
                            || method.parameters.iter().any(|p| includes(&p.api_type()))
                    })
            }
            Type::Callback(method) => {
                includes(&method.result.api_type())
                    || method.parameters.iter().any(|p| includes(&p.api_type()))
            }
            Type::TupleValue(tuple) => tuple.slots().iter().any(|slot| includes(&slot.api_type())),
            _ => false,
        }
    }
    parameters.iter().any(|p| includes(&p.api_type()))
        || ops.iter().any(|op| match &op.operation {
            Operation::RecordBegin { .. } => true,
            Operation::GlobalCall { result_type, .. }
            | Operation::ModuleCall { result_type, .. }
            | Operation::SelfCall { result_type, .. }
            | Operation::ObjectCall { result_type, .. }
            | Operation::SelfFieldRead { result_type, .. } => includes(&result_type.api_type()),
            Operation::MemberRead {
                field_type: Some(ty),
                ..
            } => includes(&ty.api_type()),
            Operation::VariableRead { value_type, .. }
            | Operation::LocalDeclare { value_type, .. }
            | Operation::BlockParameter { value_type, .. } => includes(&value_type.api_type()),
            _ => false,
        })
}
/// Mark blocks whose allocation sites can execute repeatedly.
pub(crate) fn cyclic(ops: &[LocatedOperation]) -> BTreeSet<BlockId> {
    let graph = crate::control_flow::prefix(ops);
    let mut result = BTreeSet::new();
    for &start in graph.predecessors.keys() {
        let mut pending = graph.predecessors[&start].clone();
        let mut visited = BTreeSet::new();
        while let Some(at) = pending.pop() {
            if at == start {
                result.insert(start);
                break;
            }
            if visited.insert(at) {
                pending.extend(graph.predecessors.get(&at).into_iter().flatten().copied());
            }
        }
    }
    result
}
/// Find block-entry heap states without trusting record read annotations.
/// Allocation sites in cyclic regions use weak updates to represent instances
/// retained from earlier iterations. Unreachable components start independently.
pub(crate) fn inputs(
    ops: &[LocatedOperation],
    parameters: &[ParameterType],
    types: &BTreeMap<ValueId, Type>,
) -> BTreeMap<BlockId, State> {
    let graph = crate::control_flow::prefix(ops);
    if !needed(ops, parameters) {
        return graph
            .predecessors
            .keys()
            .map(|b| (*b, State::default()))
            .collect();
    }
    let cyclic = cyclic(ops);
    let symbols = ops
        .iter()
        .filter_map(|op| {
            if let Operation::Symbol { result, name } = &op.operation {
                Some((*result, name.clone()))
            } else {
                None
            }
        })
        .collect();
    let entry = State::new(parameters);
    let mut incoming: BTreeMap<_, _> = graph
        .predecessors
        .keys()
        .map(|b| (*b, State::default()))
        .collect();
    let mut outgoing = incoming.clone();
    // A finite key set and finite type union lattice bound ordinary convergence.
    for round in 0..=ops.len() + 1 {
        let mut changed = false;
        for (block, preds) in &graph.predecessors {
            let mut state = if graph.roots.contains(block) {
                entry.clone()
            } else {
                State::default()
            };
            for pred in preds {
                state.merge(&outgoing[pred]);
            }
            incoming.insert(*block, state.clone());
            for (at, op) in ops.iter().enumerate() {
                if graph.blocks[at] == *block {
                    state.observe(&op.operation, types, &symbols, cyclic.contains(block));
                }
            }

            if outgoing.get(block) != Some(&state) {
                outgoing.insert(*block, state);
                changed = true;
            }
        }
        if !changed {
            return incoming;
        }
        if round == ops.len() + 1 {
            for state in incoming.values_mut() {
                state.invalidate();
            }
            return incoming;
        }
    }
    incoming
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unknown_external_fields_remain_dynamic_after_potential_alias_writes() {
        let parameter = ParameterType::Options(BTreeMap::new());
        let original = State::new(&[parameter]);
        let mut written = original.clone();
        written
            .heap
            .get_mut(&Root::Parameter(0))
            .unwrap()
            .insert("extra".into(), Type::named("Toybox.Lang.String"));
        written
            .writes
            .insert("extra".into(), Type::named("Toybox.Lang.String"));
        written.seed(Root::Value(ValueId(9)), &Type::Options(BTreeMap::new()));
        assert_eq!(written.heap[&Root::Value(ValueId(9))]["extra"], Type::Any);
        written.merge(&original);
        assert_eq!(written.heap[&Root::Parameter(0)]["extra"], Type::Any);
        let union = Type::Union(vec![
            Type::Options(BTreeMap::from([(
                "x".into(),
                Type::named("Toybox.Lang.Number"),
            )])),
            Type::Options(BTreeMap::new()),
        ]);
        assert_eq!(schema(&union).unwrap()["x"], Type::Any);
    }
}
