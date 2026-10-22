use ciq_compiler::{compile_sources, parse_unit_ir};

#[test]
fn structural_array_elements_and_dictionary_contents_keep_their_contracts() {
    let source = r#"
        typedef Answer as interface {function number() as Toybox.Lang.Number;};
        class First { function number() as Toybox.Lang.Number {return 41;} }
        class Second {function number() as Toybox.Lang.Number {return 42;} }
        function array() as Toybox.Lang.Array<Answer> {return [new First(),new Second()];}
        function use() as Toybox.Lang.Number {
            var values=array();
            values.add(new First());
            values[0]=new Second();
            return values[0].number();
        }
        function dictionary() as Toybox.Lang.Dictionary<Answer,Answer or Null> {
            return {new First()=>new Second()};
        }
        function read(key as Answer) as Toybox.Lang.Number {
            var values=dictionary();
            values[key]=new First();
            var value=values[key];
            if(value!=null){return value.number();}
            return 0;
        }
    "#;
    let unit = compile_sources(&[source]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    assert!(unit.lower().to_text().contains("getsv"));
    let bad = source
        .replace("values.add(new First());", "values.add(new Wrong());")
        .to_owned()
        + "class Wrong {}";
    assert!(compile_sources(&[&bad]).is_err());
}

#[test]
fn every_container_write_and_dictionary_key_read_requires_a_witness() {
    let prefix = r#"
        typedef Answer as interface {function number() as Toybox.Lang.Number;};
        class Good {function number() as Toybox.Lang.Number {return 1;}}
        class Wrong {}
    "#;
    for body in [
        "function f() as Toybox.Lang.Array<Answer> {return [new Wrong()];}",
        "function f(a as Toybox.Lang.Array<Answer>) {a[0]=new Wrong();}",
        "function f(a as Toybox.Lang.Array<Answer>) {a.add(new Wrong());}",
        "function f() as Toybox.Lang.Dictionary<Answer,Answer> {return {new Wrong()=>new Good()};}",
        "function f() as Toybox.Lang.Dictionary<Answer,Answer> {return {new Good()=>new Wrong()};}",
        "function f(d as Toybox.Lang.Dictionary<Answer,Answer>) {d[new Wrong()]=new Good();}",
        "function f(d as Toybox.Lang.Dictionary<Answer,Answer>) {d[new Good()]=new Wrong();}",
        "function f(d as Toybox.Lang.Dictionary<Answer,Answer>) {return d[new Wrong()];}",
        "function f(d as Toybox.Lang.Dictionary<Answer,Answer>) {d.put(new Wrong(),new Good());}",
        "function f(d as Toybox.Lang.Dictionary<Answer,Answer>) {return d.hasKey(new Wrong());}",
    ] {
        assert!(compile_sources(&[prefix, body]).is_err(), "{body}");
    }
    let dynamic = "function f(a as Toybox.Lang.Array<Answer>, d as Toybox.Lang.Dictionary<Answer,Answer>, value) {a[0]=value;d[value]=value;}";
    let unit = compile_sources(&[prefix, dynamic]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
}

#[test]
fn nullable_elements_and_native_constants_preserve_saved_types() {
    let source = r#"
        typedef Text as interface {function toString() as Toybox.Lang.String;};
        function array() as Toybox.Lang.Array<Text> {return [1,2];}
        function dictionary() as Toybox.Lang.Dictionary<Toybox.Lang.Number,Text> {return {1=>2};}
        function allocated() as Toybox.Lang.Array<Text or Null> {return new [2];}
        function use() as Toybox.Lang.String {
            var a=allocated();a[0]=1;var value=a[0];a[0]=null;
            if(value!=null){return value.toString();}return "null";
        }
    "#;
    let unit = compile_sources(&[source]).unwrap();
    let text = unit.to_ir_text();
    assert!(text.contains(".array-constant"));
    assert!(text.contains(".dictionary-constant"));
    assert_eq!(unit, parse_unit_ir(&text).unwrap());
    assert!(
        compile_sources(&[&source
            .replace("return new [2];", "return [null];")
            .replace("Text or Null>", "Text>")])
        .is_err()
    );
    assert!(compile_sources(&[&source.replace("Array<Text or Null>", "Array<Text>")]).is_err());
}

#[test]
fn structural_contents_are_invariant_and_nested_tag_separators_are_balanced() {
    let source = r#"
        typedef Small as interface {function size() as Toybox.Lang.Number;};
        typedef Large as interface {function size() as Toybox.Lang.Number;function extra() as Toybox.Lang.Number;};
        function array(a as Toybox.Lang.Array<Large>) as Toybox.Lang.Array<Small> {return a;}
    "#;
    assert!(compile_sources(&[source]).is_err());
    let source = r#"
        typedef Key as interface {function read(d as Toybox.Lang.Dictionary<Toybox.Lang.Number,Toybox.Lang.Number>) as Toybox.Lang.Number;};
        function identity(d as Toybox.Lang.Dictionary<Key,Key or Null>) as Toybox.Lang.Dictionary<Key,Key or Null> {return d;}
    "#;
    let unit = compile_sources(&[source]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    assert!(
        compile_sources(&[
            &source.replace("Dictionary<Key,Key or Null>", "Dictionary<Key or Null,Key>")
        ])
        .is_err()
    );
}

#[test]
fn source_classes_nested_inside_container_interfaces_resolve_across_files() {
    let source = r#"
        typedef Read as interface {function token() as Token;};
        function array(a as Toybox.Lang.Array<Read>) as Token {return a[0].token();}
        function dictionary(d as Toybox.Lang.Dictionary<Toybox.Lang.String,Read>) as Token or Null {
            var read=d["token"];if(read!=null){return read.token();}return null;
        }
    "#;
    let token =
        "class Token extends Toybox.WatchUi.View {function initialize(){View.initialize();}}";
    let unit = compile_sources(&[source, token]).unwrap();
    let text = unit.to_ir_text();
    assert!(text.contains("source(Token@WatchUi.View)"));
    assert_eq!(unit, parse_unit_ir(&text).unwrap());
    assert!(
        parse_unit_ir(&text.replace(
            "m(token()=source(Token@WatchUi.View))",
            "m(token()=source(Token@Lang.Object))"
        ))
        .is_err()
    );
    assert!(compile_sources(&[source]).is_err());
}

#[test]
fn native_container_witnesses_check_method_api_introductions() {
    use ciq_compiler::api::{ApiContext, ApiLevel};
    let early = ApiContext::new(ApiLevel(4, 0, 0), ApiLevel(4, 0, 0)).unwrap();
    let current = ApiContext::new(ApiLevel(5, 0, 0), ApiLevel(5, 0, 0)).unwrap();
    let prefix = "typedef Ordered as interface {function compareTo(x as Toybox.Lang.Object) as Toybox.Lang.Number;};";
    for body in [
        "function f() as Toybox.Lang.Array<Ordered> {return [1];}",
        "function f() as Toybox.Lang.Dictionary<Toybox.Lang.Number,Ordered> {return {1=>2};}",
        "function f(a as Toybox.Lang.Array<Ordered>) {a.add(1);}",
        "function f(a as Toybox.Lang.Array<Ordered>) {a[0]=1;}",
        "function f(d as Toybox.Lang.Dictionary<Ordered,Ordered>) {d.put(1,2);}",
    ] {
        assert!(
            ciq_compiler::compile_sources_with_api(&[prefix, body], early).is_err(),
            "{body}"
        );
        let unit = ciq_compiler::compile_sources_with_api(&[prefix, body], current).unwrap();
        assert!(unit.validate_api(early).is_err(), "{body}");
        assert!(
            ciq_compiler::parse_unit_ir_with_api(&unit.to_ir_text(), early).is_err(),
            "{body}"
        );
    }
}

#[test]
fn typed_array_operations_reject_invalid_structural_member_contracts() {
    use ciq_compiler::{
        ArrayElement, Function, LocatedOperation, Operation, ReturnType, StructuralInterface,
        ValueId,
    };
    let shape = StructuralInterface {
        fields: [("value".into(), ReturnType::Void)].into(),
        methods: Default::default(),
    };
    let element = ArrayElement::Structural {
        shape: Box::new(shape),
        nullable: false,
    };
    let operations = vec![
        LocatedOperation {
            operation: Operation::ArrayNew {
                result: ValueId(0),
                element_type: element,
            },
            span: None,
        },
        LocatedOperation {
            operation: Operation::Return { value: ValueId(0) },
            span: None,
        },
    ];
    assert!(Function::with_parameter_types("bad".into(), vec![], operations).is_err());
}

#[test]
fn union_container_receivers_check_each_structural_contents_contract() {
    let source = r#"
        typedef A as interface {function first() as Toybox.Lang.Number;};
        typedef B as interface {function second() as Toybox.Lang.Number;};
        class Both {function first() as Toybox.Lang.Number{return 1;}function second() as Toybox.Lang.Number{return 2;}}
        class OnlyFirst {function first() as Toybox.Lang.Number{return 1;}}
        function arrays(values as Toybox.Lang.Array<A> or Toybox.Lang.Array<B>) {values[0]=new Both();}
        function dictionaries(values as Toybox.Lang.Dictionary<A,A> or Toybox.Lang.Dictionary<B,B>) {values[new Both()]=new Both();return values[new Both()];}
    "#;
    let unit = compile_sources(&[source]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    for bad in [
        source.replace("values[0]=new Both()", "values[0]=new OnlyFirst()"),
        source.replace("values[new Both()]=", "values[new OnlyFirst()]="),
        source.replace(
            "return values[new Both()]",
            "return values[new OnlyFirst()]",
        ),
    ] {
        assert!(compile_sources(&[&bad]).is_err(), "{bad}");
    }
}

#[test]
fn nullable_container_references_retain_nullable_interface_contents() {
    let source = r#"
        typedef Text as interface {function toString() as Toybox.Lang.String;};
        function array(values as Toybox.Lang.Array<Text or Null> or Null) as Toybox.Lang.Array<Text or Null> or Null {return values;}
        function dictionary(values as Toybox.Lang.Dictionary<Text,Text or Null> or Null) as Toybox.Lang.Dictionary<Text,Text or Null> or Null {return values;}
    "#;
    let unit = compile_sources(&[source]).unwrap();
    assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
}
