#[path = "common/checking.rs"]
mod checking;
use ciq_compiler::{compile_sources, parse_unit_ir};
const CLASSES: &str = "class A{function initialize(){}function count() as Toybox.Lang.Number{return 7;}}class B{function initialize(){}function count() as Toybox.Lang.Number{return 9;}}";
#[test]
fn union_fields_preserve_native_and_exact_contracts_through_saved_stages() {
    for body in [
        "class Box{var value as Toybox.Lang.Number or Toybox.Lang.String;function initialize(){self.value=42;}function set(x as Toybox.Lang.Number or Toybox.Lang.String){self.value=x;}function read() as Toybox.Lang.Number or Toybox.Lang.String{return self.value;}}",
        "class Box{var value as A or B;function initialize(x as A or B){self.value=x;}function read() as Toybox.Lang.Number{return self.value.count();}}",
        "class Box{var value as A or Toybox.Lang.String or Null = null;function initialize(){}function read() as Toybox.Lang.Number{var x=self.value;if(x==null){return 0;}if(x instanceof A){return x.count();}return x.length();}}",
        "class Box{var a as Toybox.Lang.Array<Toybox.Lang.Number> or Null;var d as Null or Toybox.Lang.Dictionary<Toybox.Lang.String,Toybox.Lang.Number>;function initialize(){}function set(){self.a=[1];self.d={\"a\"=>2};}function read() as Toybox.Lang.Number{var a=self.a;if(a==null){return 0;}return a[0];}}",
        "class Box{var value as Toybox.Lang.Array<Toybox.Lang.Number> or Toybox.Lang.Array<Toybox.Lang.Long>;function initialize(){self.value=[1];}function read() as Toybox.Lang.Integer{var a=self.value;return a[0];}}",
    ] {
        let unit = compile_sources(&[CLASSES, body]).unwrap_or_else(|e| panic!("{body}: {e}"));
        assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
        assert_eq!(
            unit.lower(),
            ciq_assembly::Program::parse(&unit.lower().to_text()).unwrap()
        );
    }
}
#[test]
fn required_initialization_invariance_and_read_claims_are_enforced() {
    for body in [
        "class Box{var value as A or B;function initialize(){}}",
        "class Box{var value as Toybox.Lang.Number or Toybox.Lang.String;function initialize(){self.value=true;}}",
        "class Box{var value as A or B;function initialize(x as Toybox.Lang.Object){self.value=x;}}",
        "class Box{var value as A or B;function initialize(){var v=self.value;self.value=new A();}}",
        "class Box{var value as A or Toybox.Lang.String or Null;function initialize(){}function read(){return self.value.count();}}",
        "class Box{var value as Toybox.Lang.Array<Toybox.Lang.Number> or Null;function initialize(){}function read(){if(self.value!=null){return self.value[0];}return 0;}}",
    ] {
        assert!(
            checking::compile_sources(&[CLASSES, body]).is_err(),
            "{body}"
        );
    }
    let unit=compile_sources(&[CLASSES,"class Box{var value as A or B;function initialize(){self.value=new A();}function read() as A or B{return self.value;}}"]).unwrap();
    let text = unit.to_ir_text();
    for changed in [
        text.replace(
            ".field-value value source-union(source(A@Lang.Object)|source(B@Lang.Object))",
            ".field-value value source(A@Lang.Object)",
        ),
        text.replace("source(B@Lang.Object)", "source(Missing@Lang.Object)"),
        text.replace(
            ".field-value value source-union(source(A@Lang.Object)|source(B@Lang.Object))",
            ".field-value value tuple(integer)",
        ),
    ] {
        assert_ne!(text, changed);
        assert!(parse_unit_ir(&changed).is_err(), "{changed}");
    }
}

#[test]
fn nullable_container_fields_supply_context_without_widening_existing_containers() {
    for body in [
        "class Box{var a as Toybox.Lang.Array<Toybox.Lang.Number> or Null;var d as Toybox.Lang.Dictionary<Toybox.Lang.String,Toybox.Lang.Number> or Null;function initialize(){self.a=[];self.d={};}function clear(){self.a=null;self.d=null;}}",
        "class Box{var a as A or Toybox.Lang.Array<Toybox.Lang.Number> or Null;function initialize(){self.a=[];}function read(){var x=self.a;if(x instanceof A){return x.count();}if(x==null){return 0;}return x.size();}}",
    ] {
        let unit = compile_sources(&[CLASSES, body]).unwrap_or_else(|e| panic!("{body}: {e}"));
        assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    }
    assert!(compile_sources(&["class Box{var a as Toybox.Lang.Array<Toybox.Lang.Number> or Null;function initialize(a as Toybox.Lang.Array<Toybox.Lang.Long>){self.a=a;}}"]).is_err());
}

#[test]
fn field_writes_require_structural_witnesses_for_each_alternative() {
    let good = "class Good{function initialize(){}function compareTo(other as Toybox.Lang.Object) as Toybox.Lang.Number{return 0;}}class Token{function initialize(){}}";
    for body in [
        "class Box{var value as Token or Toybox.Lang.Comparable;function initialize(x as Good){self.value=x;}}",
        "class Box{var value as Toybox.Lang.Comparable or Toybox.Lang.String;function initialize(x as Good){self.value=x;}}",
        "class Box{var value as Toybox.Lang.Comparable or Toybox.Lang.String or Null;function initialize(){}function set(x as Good or Toybox.Lang.String or Null){self.value=x;}}",
    ] {
        let unit = compile_sources(&[good, body]).unwrap_or_else(|e| panic!("{body}: {e}"));
        assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
        let bad = good.replace(
            "function compareTo(other as Toybox.Lang.Object) as Toybox.Lang.Number{return 0;}",
            "",
        );
        assert!(compile_sources(&[&bad, body]).is_err(), "{body}");
    }
}

#[test]
fn field_domains_defaults_and_api_limits_are_rechecked_even_without_reads() {
    use ciq_compiler::{
        api::{ApiContext, ApiLevel},
        compile_sources_with_api, parse_unit_ir_with_api,
    };
    for body in [
        "class Box{var value as A or Toybox.Graphics.Dc or Null;function initialize(){}}",
        "class Box{var value as Toybox.Lang.String or Toybox.Graphics.Dc or Null;function initialize(){}}",
        "class Box{var value as Toybox.Lang.Number or Toybox.Lang.String = 1;function initialize(){}}",
        "class Box{var value as Toybox.Lang.Number or Toybox.Lang.String or Null = 1;function initialize(){}}",
    ] {
        assert!(compile_sources(&[CLASSES, body]).is_err(), "{body}");
    }
    let source = "class Box{var value as Toybox.Lang.ByteArray or Toybox.Lang.String or Null;function initialize(){}}";
    let unit = compile_sources(&[source]).unwrap();
    let low = ApiContext::new(ApiLevel(1, 0, 0), ApiLevel(1, 0, 0)).unwrap();
    assert!(compile_sources_with_api(&[source], low).is_err());
    assert!(parse_unit_ir_with_api(&unit.to_ir_text(), low).is_err());
}

#[test]
fn typed_field_and_read_contracts_normalize_like_saved_text() {
    use ciq_compiler::{CompilationUnit, FieldKind, Function, Operation, ReturnType, api::Type};
    let unit=compile_sources(&["class Box{var value as Toybox.Lang.Number or Toybox.Lang.String;function initialize(){self.value=42;}function read() as Toybox.Lang.Number or Toybox.Lang.String{return self.value;}}"]).unwrap();
    let raw = ReturnType::Union(Type::Union(vec![
        Type::named("Toybox.Lang.String"),
        Type::named("Toybox.Lang.Number"),
        Type::named("Toybox.Lang.String"),
    ]));
    let mut classes = unit.classes().to_vec();
    classes[0].fields[0].kind = FieldKind::Value {
        value_type: raw.clone(),
    };
    for method in &mut classes[0].methods {
        if method.name() == "read" {
            let mut operations = method.operations().to_vec();
            for located in &mut operations {
                if let Operation::SelfFieldRead { result_type, .. } = &mut located.operation {
                    *result_type = raw.clone();
                }
            }
            *method = Function::method_with_signature(
                "read".into(),
                vec![],
                method.return_type(),
                operations,
            )
            .unwrap();
        }
    }
    let typed = CompilationUnit::with_classes(unit.declarations().to_vec(), classes).unwrap();
    assert_eq!(unit, typed);
    assert_eq!(typed, parse_unit_ir(&typed.to_ir_text()).unwrap());
}
