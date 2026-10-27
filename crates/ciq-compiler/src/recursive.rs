//! Finite, lexically bound recursive contracts. References count enclosing binders.
use crate::api::Type;

fn result_type(ty: Type) -> Option<crate::ReturnType> {
    if ty == Type::Void {
        Some(crate::ReturnType::Void)
    } else {
        crate::unions::result(ty)
    }
}

fn has_reference(ty: &Type) -> bool {
    match ty {
        Type::RecursionRef(_) => true,
        Type::Recursive(body) | Type::Array(body) => has_reference(body),
        Type::Dictionary { key, value } => has_reference(key) || has_reference(value),
        Type::Union(parts) | Type::Tuple(parts) => parts.iter().any(has_reference),
        Type::Structural(shape) => shape.api_types().any(|ty| has_reference(&ty)),
        Type::Callback(signature) => {
            signature
                .parameters
                .iter()
                .any(|ty| has_reference(&ty.exact_type()))
                || has_reference(&signature.result.api_type())
        }
        Type::Options(fields) => fields.values().any(has_reference),
        Type::TupleValue(tuple) => tuple.slots().iter().any(|ty| has_reference(&ty.api_type())),
        Type::SourceValue(contract) => contract.native().iter().any(has_reference),
        _ => false,
    }
}

/// Rebuild a type while rewriting recursion references at their lexical depth.
fn rewrite(ty: &Type, depth: usize, f: &mut impl FnMut(usize, usize) -> Type) -> Type {
    match ty {
        Type::Recursive(body) => Type::Recursive(Box::new(rewrite(body, depth + 1, f))),
        Type::RecursionRef(index) => f(*index, depth),
        Type::SourceValue(contract) => {
            let mut contract = (**contract).clone();
            for ty in &mut contract.native {
                *ty = rewrite(ty, depth, f);
            }
            Type::SourceValue(Box::new(contract))
        }
        Type::Array(element) => Type::Array(Box::new(rewrite(element, depth, f))),
        Type::Dictionary { key, value } => {
            Type::dictionary(rewrite(key, depth, f), rewrite(value, depth, f))
        }
        Type::Union(parts) => Type::Union(parts.iter().map(|ty| rewrite(ty, depth, f)).collect()),
        Type::Options(fields) => Type::Options(
            fields
                .iter()
                .map(|(name, ty)| (name.clone(), rewrite(ty, depth, f)))
                .collect(),
        ),
        Type::Structural(shape) => {
            let mut shape = (**shape).clone();
            for ty in shape.fields.values_mut() {
                *ty = result_type(rewrite(&crate::tuples::exact_type(ty), depth, f))
                    .expect("supported recursive field");
            }
            for method in shape.methods.values_mut() {
                for ty in &mut method.parameters {
                    *ty = crate::unions::parameter(rewrite(&ty.exact_type(), depth, f))
                        .expect("supported recursive parameter");
                }
                method.result = result_type(rewrite(
                    &crate::tuples::exact_type(&method.result),
                    depth,
                    f,
                ))
                .expect("supported recursive result");
            }
            Type::Structural(Box::new(shape))
        }
        Type::Callback(signature) => {
            let mut signature = (**signature).clone();
            for ty in &mut signature.parameters {
                *ty = crate::unions::parameter(rewrite(&ty.exact_type(), depth, f))
                    .expect("supported recursive callback parameter");
            }
            signature.result = result_type(rewrite(
                &crate::tuples::exact_type(&signature.result),
                depth,
                f,
            ))
            .expect("supported recursive callback result");
            Type::Callback(Box::new(signature))
        }
        Type::TupleValue(tuple) => Type::TupleValue(
            crate::TupleType::new(
                tuple
                    .slots()
                    .iter()
                    .map(|ty| {
                        result_type(rewrite(&crate::tuples::exact_type(ty), depth, f))
                            .expect("supported recursive slot")
                    })
                    .collect(),
            )
            .expect("supported recursive tuple"),
        ),
        Type::Tuple(parts) => Type::Tuple(parts.iter().map(|ty| rewrite(ty, depth, f)).collect()),
        _ => ty.clone(),
    }
}

/// Close an alias's own references; remove an unused binder without capturing outer references.
pub(crate) fn bind(body: Type) -> Type {
    if !has_reference(&body) {
        return body;
    }
    let mut used = false;
    rewrite(&body, 0, &mut |index, depth| {
        used |= index == depth;
        Type::RecursionRef(index)
    });
    if used {
        Type::Recursive(Box::new(body))
    } else {
        rewrite(&body, 0, &mut |index, depth| {
            Type::RecursionRef(if index > depth { index - 1 } else { index })
        })
    }
}

/// Unfold one recursive constructor, returning closed contracts for its children.
pub(crate) fn unfold(ty: &Type) -> Option<Type> {
    let Type::Recursive(body) = ty else {
        return None;
    };
    Some(rewrite(body, 0, &mut |index, depth| {
        if index == depth {
            ty.clone()
        } else {
            Type::RecursionRef(if index > depth { index - 1 } else { index })
        }
    }))
}

/// Validate closed, guarded references with bounded syntax depth. A reference
/// must cross a container, tuple, callback, record, or structural constructor.
pub(crate) fn validate(ty: &Type) -> Result<(), String> {
    fn visit(
        ty: &Type,
        binders: &mut Vec<(usize, bool)>,
        guards: usize,
        depth: usize,
    ) -> Result<(), String> {
        if depth > 64 {
            return Err("recursive contract exceeds 64 syntax levels".into());
        }
        match ty {
            Type::Recursive(body) => {
                binders.push((guards, false));
                visit(body, binders, guards, depth + 1)?;
                if !binders.pop().unwrap().1 {
                    return Err("recursive binder has no recursive reference".into());
                }
                if crate::unions::parameter((**body).clone()).is_none() {
                    return Err("unsupported recursive contract body".into());
                }
            }
            Type::RecursionRef(index) => {
                let binder = index
                    .checked_add(1)
                    .and_then(|count| binders.len().checked_sub(count))
                    .and_then(|index| binders.get_mut(index))
                    .ok_or("unbound recursive contract reference")?;
                if guards <= binder.0 {
                    return Err("unguarded recursive contract reference".into());
                }
                binder.1 = true;
            }
            Type::SourceValue(contract) => {
                for ty in contract.native() {
                    visit(ty, binders, guards, depth + 1)?;
                }
            }
            Type::Array(element) => visit(element, binders, guards + 1, depth + 1)?,
            Type::Dictionary { key, value } => {
                visit(key, binders, guards + 1, depth + 1)?;
                visit(value, binders, guards + 1, depth + 1)?;
            }
            Type::Structural(shape) => {
                for ty in shape.api_types() {
                    visit(&ty, binders, guards + 1, depth + 1)?;
                }
            }
            Type::Callback(signature) => {
                for ty in signature
                    .parameters
                    .iter()
                    .map(crate::ParameterType::exact_type)
                    .chain(std::iter::once(signature.result.api_type()))
                {
                    visit(&ty, binders, guards + 1, depth + 1)?;
                }
            }
            Type::TupleValue(tuple) => {
                for ty in tuple.slots() {
                    visit(&ty.api_type(), binders, guards + 1, depth + 1)?;
                }
            }
            Type::Options(fields) => {
                for ty in fields.values() {
                    visit(ty, binders, guards + 1, depth + 1)?;
                }
            }
            Type::Tuple(parts) => {
                for ty in parts {
                    visit(ty, binders, guards + 1, depth + 1)?;
                }
            }
            Type::Union(parts) => {
                for ty in parts {
                    visit(ty, binders, guards, depth + 1)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    visit(ty, &mut Vec::new(), 0, 0)
}

/// Print a recursive term using the ordinary saved-type grammar for its body.
pub(crate) fn tag(ty: &Type) -> String {
    match ty {
        Type::Recursive(body) => format!(
            "rec({})",
            crate::type_text::parameter_tag(
                &crate::unions::parameter((**body).clone())
                    .expect("recursive body has a type spelling")
            )
        ),
        Type::RecursionRef(index) => format!("ref({index})"),
        _ => unreachable!("recursive wrapper contains a recursive term"),
    }
}
