use ciq_compiler::{
    api::{ApiContext, ApiLevel},
    compile, compile_sources, compile_sources_with_api, parse_ir, parse_unit_ir,
};

#[test]
fn native_type_spellings_share_signature_and_saved_pipeline_contracts() {
    for template in [
        "$Object",
        "$Number",
        "$Long",
        "$Float",
        "$Double",
        "$Boolean",
        "$Char",
        "$String",
        "$Method",
        "$Comparable",
        "$Comparator",
        "$Comparable or Null",
        "$Comparator or Null",
        "$Symbol",
        "$WeakReference",
        "$Dictionary",
        "$Array<$Number>",
        "$Array<$String or Null>",
        "$Array<$Comparable>",
        "$Array<$Comparator>",
        "$Array<$Comparable or Null>",
        "$Array<$Comparator or Null>",
        "$Dictionary<$String, $Number>",
        "$Dictionary<$String, $String or Null>",
        "$Dictionary<$String, $Comparable>",
        "$Dictionary<$String, $Comparable or Null>",
        "$Dictionary<$String, $Comparator>",
        "$Dictionary<$String, $Comparator or Null>",
    ] {
        let canonical = template.replace('$', "Toybox.Lang.");
        let reference = compile(&format!(
            "function f(value as {canonical}) as {canonical} {{ return value; }}"
        ))
        .unwrap();
        for (prefix, qualifier) in [
            ("import Toybox.Lang;", ""),
            ("using Toybox.Lang as L;", "L."),
            ("using Toybox.Lang;", "Lang."),
        ] {
            let ty = template.replace('$', qualifier);
            let source = format!("{prefix} function f(value as {ty}) as {ty} {{ return value; }}");
            let function = compile(&source).unwrap_or_else(|e| panic!("{source}: {e}"));
            assert_eq!(function.parameter_types(), reference.parameter_types());
            assert_eq!(function.return_type(), reference.return_type());
            assert_eq!(function.lower(), reference.lower());
            assert_eq!(function, parse_ir(&function.to_ir_text()).unwrap());
        }
    }
    let unit = compile_sources(&["import Toybox.Lang; module M { function f(value as Object) { if (value instanceof Number) { return value as Number + 1; } else { return 0; } } }"]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    for ty in ["View", "BehaviorDelegate"] {
        let source =
            format!("import Toybox.WatchUi; function f(value as {ty}) as {ty} {{ return value; }}");
        assert!(compile(&source).is_ok(), "{source}");
    }
    assert!(compile("import Toybox.Lang; import Toybox.WatchUi; function f(a as Array<View>) as Array<View> { return a; }").is_ok());
}

#[test]
fn import_visibility_is_file_and_owner_local_and_using_keeps_qualification() {
    for source in [
        "using Toybox.Lang; function f(value as Number) { return value; }",
        "using Toybox.Lang as L; function f(value as Number) { return value; }",
        "module A { import Toybox.Lang; function f(value as Number) { return value; } } function g(value as Number) { return value; }",
    ] {
        assert!(compile_sources(&[source]).is_err(), "{source}");
    }
    assert!(
        compile_sources(&[
            "import Toybox.Lang; function f(value as Number) { return value; }",
            "function g(value as Number) { return value; }"
        ])
        .is_err()
    );
    let unit = compile_sources(&["module A { import Toybox.Lang; function f(value as Number) { return value; } } module B { using Toybox.Lang as L; function g(value as L.Number) { return value; } }"]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    assert!(
        compile_sources(&[
            "import Toybox.Lang; function f(value as C) as C { return value; }",
            "class C { function initialize() {} }"
        ])
        .is_ok()
    );
}

#[test]
fn imported_names_keep_shadowing_api_and_executable_profile_checks() {
    for source in [
        "import Toybox.Lang; function f(Number) {}",
        "import Toybox.Lang; function f() { var String=1; }",
        "import Toybox.Lang; class Number { function initialize() {} }",
        "import Toybox.Lang; class Comparator { function initialize() {} }",
        "import Toybox.Lang; using Toybox.System as Number; function f() {}",
        "using Toybox.System as Number; import Toybox.Lang; function f() {}",
        "import Toybox.Lang; function f(o as Object) as Boolean { return o instanceof Comparator; }",
        "import Toybox.Lang; function f(o as Object) { var a=o as Array; }",
    ] {
        assert!(compile_sources(&[source]).is_err(), "{source}");
    }
    let source = "import Toybox.Lang; function f(value as Char) as Char { return value; }";
    let low = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(1, 0, 0)).unwrap();
    assert!(compile_sources_with_api(&[source], low).is_err());
    let high = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(1, 3, 0)).unwrap();
    let unit = compile_sources_with_api(&[source], high).unwrap();
    assert!(unit.validate_api(low).is_err());
}
