//! Definite initialization through allocation hooks and explicitly selected initializers.
use super::{
    CompilationUnit, FIELD_INITIALIZER, Function, Operation, Owner, activation_callees,
    comparator_callees, erased_callbacks, requests_exit,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

type Field = (String, String);
type Pending = BTreeSet<Field>;
type Walk = Result<Option<Pending>, (usize, String)>;
type Active<'a> = BTreeSet<(&'a str, &'a str)>;

/// Check constructor-required storage without treating ordinary initialization
/// as allocation or replacing the runtime receiver with the selected parent.
pub(super) struct Proof<'a, 'b> {
    pub unit: &'a CompilationUnit,
    pub bodies: &'b [(Owner<'a>, &'a Function)],
    pub exits: &'b BTreeSet<(Owner<'a>, &'a str)>,
}

impl<'a> Proof<'a, '_> {
    /// Collect declaration identities, keeping private and shadowed requirements apart.
    pub(super) fn required(&self, class: &str) -> Pending {
        let ancestors = &self.unit.member_index.classes[class].ancestors;
        self.unit
            .classes
            .iter()
            .filter(|c| c.name == class || ancestors.contains(&c.name))
            .flat_map(|c| {
                c.fields
                    .iter()
                    .filter(|f| !f.kind.class_shared() && f.kind.requires_initialization())
                    .map(|f| (c.name.clone(), f.name.clone()))
            })
            .collect()
    }

    /// Validate the initializer; allocation additionally requires all ancestor
    /// fields ready on return. Declaration checks cover own fields; callers
    /// supply ancestor readiness when inspecting a parent initializer's body.
    pub fn check(&self, class: &str, allocation: bool) -> Result<(), (usize, String)> {
        let mut pending = self.required(class);
        if !allocation {
            pending.retain(|(owner, _)| owner == class);
        }
        if allocation {
            self.check_hooks(class, &pending)?;
        }
        self.walk(class, allocation, pending, &mut BTreeSet::new())?;
        Ok(())
    }

    /// Literal metadata defaults are already present. Expression defaults become
    /// ready in parent-first order, and ordinary constructor fields remain pending.
    fn check_hooks(&self, class: &str, required: &Pending) -> Result<(), (usize, String)> {
        let chain = self.unit.member_index.classes[class]
            .ancestors
            .iter()
            .rev()
            .map(String::as_str)
            .chain(std::iter::once(class));
        let mut pending = required.clone();
        for owner in chain.clone() {
            let declaration = self.unit.classes.iter().find(|c| c.name == owner).unwrap();
            pending.extend(
                declaration
                    .fields
                    .iter()
                    .filter(|f| f.kind.allocation_write())
                    .map(|f| (declaration.name.clone(), f.name.clone())),
            );
        }
        for owner in chain {
            let Some((declaration, function)) = self.unit.method(owner, FIELD_INITIALIZER) else {
                continue;
            };
            let Some(remaining) = self
                .walk_body(
                    &declaration.name,
                    function,
                    pending,
                    &Pending::new(),
                    &mut Active::new(),
                    Some(class),
                )
                .map_err(|(at, message)| (at, format!("allocation hook {owner}: {message}")))?
            else {
                break;
            };
            pending = remaining;
        }
        Ok(())
    }

    /// Validate a hook against its own class even if the unit never allocates it.
    pub(super) fn check_declared_hook(
        &self,
        class: &'a str,
        function: &'a Function,
    ) -> Result<(), (usize, String)> {
        let mut pending = self.required(class);
        let declaration = self.unit.classes.iter().find(|c| c.name == class).unwrap();
        pending.extend(
            declaration
                .fields
                .iter()
                .filter(|f| f.kind.allocation_write())
                .map(|f| (declaration.name.clone(), f.name.clone())),
        );
        self.walk_body(
            class,
            function,
            pending,
            &Pending::new(),
            &mut Active::new(),
            Some(class),
        )?;
        Ok(())
    }

    /// A known call can borrow the in-progress receiver if its actual body
    /// only reads ready fields and does not expose that receiver or shut down.
    fn hook_callee(
        &self,
        class: &'a str,
        runtime: &str,
        function: &'a Function,
        operation: &Operation,
    ) -> Option<(&'a str, &'a Function)> {
        let call = if let Operation::CallBegin { builder } = operation {
            &function
                .operations()
                .iter()
                .find(|op| op.operation.call_capture() == Some(*builder))?
                .operation
        } else {
            operation
        };
        let lexical = crate::inheritance::MethodView::Lexical;
        let (owner, callee) = match call {
            Operation::SelfCall { member, .. } | Operation::SelfCallStatement { member, .. } => {
                self.unit
                    .method_view(class, member, Some(class), lexical)
                    .filter(|(owner, _)| {
                        self.unit.member_visibility(&owner.name, member)
                            == crate::Visibility::Private
                    })
                    .or_else(|| {
                        self.unit.ordinary_runtime_method(
                            runtime,
                            member,
                            Some(class),
                            super::MethodReceiver::InstanceSelf,
                        )
                    })?
            }
            Operation::ClassCall {
                class: target,
                member,
                ..
            }
            | Operation::ClassCallStatement {
                class: target,
                member,
                ..
            } if self
                .unit
                .has_instance_class_receiver(target, Owner::Class(class), function) =>
            {
                self.unit
                    .method_view(target, member, Some(class), lexical)?
            }
            _ => return None,
        };
        Some((&owner.name, callee))
    }

    /// Select only a fixed initializer on this object's own class or ancestry.
    /// Captures resolve a callable now but execute it only at completion.
    fn initializer(
        &self,
        class: &str,
        function: &Function,
        operation: &Operation,
    ) -> Option<&'a str> {
        let call = if let Operation::CallBegin { builder } = operation {
            &function
                .operations()
                .iter()
                .find(|op| op.operation.call_capture() == Some(*builder))?
                .operation
        } else {
            operation
        };
        let (Operation::ClassCall {
            class: target,
            member,
            ..
        }
        | Operation::ClassCallStatement {
            class: target,
            member,
            ..
        }) = call
        else {
            return None;
        };
        if member != "initialize"
            || !self
                .unit
                .has_instance_class_receiver(target, Owner::Class(class), function)
        {
            return None;
        }
        self.unit
            .method_view(
                target,
                member,
                Some(class),
                crate::inheritance::MethodView::Lexical,
            )
            .filter(|(_, f)| f.is_instance_method())
            .map(|(owner, _)| owner.name.as_str())
    }

    /// Inspect the selected initializer with live readiness state instead of its
    /// whole-body exit summary; retain conservative effects from other callees
    /// and statics activated by callable lookup.
    fn may_exit(
        &self,
        class: &'a str,
        function: &'a Function,
        operation: &'a Operation,
        initializer: Option<&str>,
    ) -> bool {
        requests_exit(operation)
            || erased_callbacks(function, operation)
            || activation_callees(
                self.unit,
                Owner::Class(class),
                operation,
                function,
                self.bodies,
            )
            .into_iter()
            .filter(|(owner, member)| {
                !(*member == "initialize"
                    && matches!(owner, Owner::Class(c) if Some(*c) == initializer))
            })
            .any(|key| self.exits.contains(&key))
            || comparator_callees(self.unit, function, operation)
                .into_iter()
                .any(|key| self.exits.contains(&key))
    }

    fn walk(
        &self,
        class: &str,
        ancestors: bool,
        pending: Pending,
        active: &mut Active<'a>,
    ) -> Walk {
        if pending.is_empty() {
            return Ok(Some(pending));
        }
        let declaration = self
            .unit
            .classes
            .iter()
            .find(|c| c.name == class)
            .expect("declared class");
        let Some(function) = declaration
            .methods
            .iter()
            .find(|f| f.name() == "initialize")
        else {
            return Err((0, "required fields require a source initializer".into()));
        };
        let key = (declaration.name.as_str(), function.name());
        if !active.insert(key) {
            return Err((
                0,
                "recursive initializer cannot prove required fields initialized".into(),
            ));
        }
        let required = if ancestors {
            self.required(class)
        } else {
            self.required(class)
                .into_iter()
                .filter(|(owner, _)| owner == class)
                .collect()
        };
        let result = self.walk_body(
            &declaration.name,
            function,
            pending,
            &required,
            active,
            None,
        );
        active.remove(&key);
        result
    }

    /// Follow entry-reachable blocks. A field remains pending at a join if any
    /// incoming path has yet to initialize it. A call contributes only its
    /// returning paths, and capturing the callable does not execute the body.
    fn walk_body(
        &self,
        class: &'a str,
        function: &'a Function,
        pending: Pending,
        required: &Pending,
        active: &mut Active<'a>,
        runtime: Option<&str>,
    ) -> Walk {
        let operations = function.operations();
        let structure = crate::control_flow::validate(operations).expect("checked function CFG");
        let mut blocks = BTreeMap::<_, Vec<_>>::new();
        for (at, block) in structure.blocks.iter().enumerate() {
            blocks.entry(*block).or_default().push(at);
        }
        let entry = structure.roots[0];
        let mut inputs = BTreeMap::from([(entry, pending)]);
        let mut queue = VecDeque::from([entry]);
        let mut returned = None;
        while let Some(block) = queue.pop_front() {
            let mut pending = inputs[&block].clone();
            for &at in &blocks[&block] {
                let operation = &operations[at].operation;
                let error = |message: &str| Err((at, message.into()));
                match operation {
                    Operation::ReturnVoid | Operation::Return { .. } => {
                        if !pending.is_disjoint(required) {
                            return error(
                                "initialize all required fields before return, including ancestor fields",
                            );
                        }
                        returned.get_or_insert_with(Pending::new).extend(pending);
                        break;
                    }
                    Operation::Branch {
                        on_true, on_false, ..
                    } => {
                        for target in [on_true, on_false] {
                            if merge_pending(&mut inputs, *target, &pending) {
                                queue.push_back(*target);
                            }
                        }
                        break;
                    }
                    Operation::Jump { target } | Operation::JumpValue { target, .. } => {
                        if merge_pending(&mut inputs, *target, &pending) {
                            queue.push_back(*target);
                        }
                        break;
                    }
                    _ => {}
                }
                if pending.is_empty() {
                    continue;
                }
                let initializer = if runtime.is_none() {
                    self.initializer(class, function, operation)
                } else {
                    None
                };
                if self.may_exit(class, function, operation, initializer) {
                    return error(
                        "initialize required fields before calls that may request shutdown",
                    );
                }
                if let Some(target) = initializer {
                    if !matches!(operation, Operation::CallBegin { .. }) {
                        let Some(remaining) =
                            self.walk(target, true, pending, active)
                                .map_err(|(_, message)| {
                                    (at, format!("initializer {target}: {message}"))
                                })?
                        else {
                            break;
                        };
                        pending = remaining;
                    }
                    continue;
                }
                if let Some((owner, callee)) = runtime
                    .and_then(|runtime| self.hook_callee(class, runtime, function, operation))
                {
                    if matches!(operation, Operation::CallBegin { .. }) || callee.is_static_method()
                    {
                        continue;
                    }
                    let key = (owner, callee.name());
                    if !active.insert(key) {
                        return error("recursive call cannot prove allocation receiver readiness");
                    }
                    let result = self.walk_body(
                        owner,
                        callee,
                        pending.clone(),
                        &Pending::new(),
                        active,
                        runtime,
                    );
                    active.remove(&key);
                    if result
                        .map_err(|(_, message)| {
                            (at, format!("{owner}.{}: {message}", callee.name()))
                        })?
                        .is_none()
                    {
                        break;
                    }
                    continue;
                }
                match operation {
                    Operation::SelfFieldRead { field, .. }
                    | Operation::SelfFieldAssign { field, .. } => {
                        if let Some((owner, field)) =
                            self.unit.bound_instance_field(class, field, Some(class))
                        {
                            let key = (owner.name.clone(), field.name.clone());
                            if matches!(operation, Operation::SelfFieldRead { .. })
                                && (pending.contains(&key)
                                    || runtime.is_some_and(|runtime| {
                                        self.pending_shadow(runtime, &key, &pending)
                                    }))
                            {
                                return error("required field read before initialization");
                            }
                            if matches!(operation, Operation::SelfFieldAssign { .. }) {
                                pending.remove(&key);
                            }
                        }
                    }
                    Operation::SelfValue { .. } => {
                        return error("initialize all required fields before exposing self");
                    }
                    Operation::SelfCall { .. } | Operation::SelfCallStatement { .. } => {
                        return error("initialize all required fields before self calls");
                    }
                    _ if self.unit.instance_class_call(
                        Owner::Class(class),
                        function,
                        operation,
                    ) =>
                    {
                        return error(
                            "initialize all required fields before qualified instance calls",
                        );
                    }
                    _ => {}
                }
            }
        }
        Ok(returned)
    }

    /// An ordinary self selector may also reach a non-private descendant slot
    /// on the object being allocated. A private lexical selector is independent.
    fn pending_shadow(&self, runtime: &str, field: &Field, pending: &Pending) -> bool {
        if self.unit.member_visibility(&field.0, &field.1) == crate::Visibility::Private {
            return false;
        }
        let identity = &self.unit.member_index.classes[runtime];
        std::iter::once(&identity.name)
            .chain(&identity.ancestors)
            .take_while(|owner| **owner != field.0)
            .any(|owner| {
                pending.contains(&(owner.clone(), field.1.clone()))
                    && self.unit.member_visibility(owner, &field.1) != crate::Visibility::Private
            })
    }
}

fn merge_pending(
    inputs: &mut BTreeMap<crate::BlockId, Pending>,
    block: crate::BlockId,
    pending: &Pending,
) -> bool {
    use std::collections::btree_map::Entry;
    match inputs.entry(block) {
        Entry::Vacant(entry) => {
            entry.insert(pending.clone());
            true
        }
        Entry::Occupied(mut entry) => {
            let input = entry.get_mut();
            let before = input.len();
            input.extend(pending.iter().cloned());
            before != input.len()
        }
    }
}
