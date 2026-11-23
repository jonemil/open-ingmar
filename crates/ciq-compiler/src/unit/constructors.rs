//! Straight-line construction proofs across explicitly selected initializers.
use super::{
    CompilationUnit, Function, Operation, Owner, activation_callees, comparator_callees,
    erased_callbacks, requests_exit,
};
use std::collections::BTreeSet;

type Field = (String, String);

/// Check constructor-required storage without treating ordinary initialization
/// as allocation or replacing the runtime receiver with the selected parent.
pub(super) struct Proof<'a, 'b> {
    pub unit: &'a CompilationUnit,
    pub bodies: &'b [(Owner<'a>, &'a Function)],
    pub exits: &'b BTreeSet<(Owner<'a>, &'a str)>,
}

impl<'a> Proof<'a, '_> {
    /// Collect declaration identities, keeping private and shadowed requirements apart.
    pub(super) fn required(&self, class: &str) -> BTreeSet<Field> {
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

    /// Validate the body prefix; allocation additionally requires all ancestor
    /// fields ready on return. Declaration checks cover own fields; callers
    /// supply ancestor readiness when inspecting a parent initializer's body.
    pub fn check(&self, class: &str, allocation: bool) -> Result<(), (usize, String)> {
        let mut pending = self.required(class);
        if !allocation {
            pending.retain(|(owner, _)| owner == class);
        }
        self.walk(class, allocation, &mut pending, &mut BTreeSet::new())
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
        pending: &mut BTreeSet<Field>,
        active: &mut BTreeSet<String>,
    ) -> Result<(), (usize, String)> {
        if pending.is_empty() {
            return Ok(());
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
        if !active.insert(class.into()) {
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
        for (at, located) in function.operations().iter().enumerate() {
            if pending.is_empty() {
                break;
            }
            let operation = &located.operation;
            let initializer = self.initializer(class, function, operation);
            let error = |message: &str| Err((at, message.into()));
            if self.may_exit(&declaration.name, function, operation, initializer) {
                return error("initialize required fields before calls that may request shutdown");
            }
            if let Some(target) = initializer {
                if !matches!(operation, Operation::CallBegin { .. }) {
                    self.walk(target, true, pending, active)
                        .map_err(|(_, message)| (at, format!("initializer {target}: {message}")))?;
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
                            && pending.contains(&key)
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
                Operation::SelfCall { .. }
                | Operation::SelfCallStatement { .. }
                | Operation::Branch { .. }
                | Operation::Jump { .. } => {
                    return error(
                        "initialize all required fields before self calls or control flow",
                    );
                }
                Operation::ReturnVoid => {
                    if !pending.is_disjoint(&required) {
                        return error(
                            "initialize all required fields before return, including ancestor fields",
                        );
                    }
                    break;
                }
                _ if self
                    .unit
                    .instance_class_call(Owner::Class(class), function, operation) =>
                {
                    return error("initialize all required fields before qualified instance calls");
                }
                _ => {}
            }
        }
        active.remove(class);
        Ok(())
    }
}
