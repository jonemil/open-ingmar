use ciq_compiler::{compile_sources, parse_unit_ir};
const PREFIX: &str = "using Toybox.Lang; typedef Numbers as Lang.Array<Lang.Number>; typedef Rows as Lang.Array<Numbers>; typedef Map as Lang.Dictionary<Lang.String,Lang.Number>; typedef Maps as Lang.Array<Map>;";
fn check(source: &str) -> ciq_compiler::CompilationUnit {
    let unit = compile_sources(&[PREFIX, source]).unwrap_or_else(|e| panic!("{source}\n{e:?}"));
    let text = unit.to_ir_text();
    assert_eq!(unit, parse_unit_ir(&text).unwrap(), "{text}");
    let asm = unit.lower();
    assert_eq!(asm, ciq_assembly::Program::parse(&asm.to_text()).unwrap());
    unit
}
#[test]
fn nested_contracts_survive_construction_storage_methods_and_shape_loss() {
    for s in [
        r#"function f() as Rows {return [[1,2],[],[3]];}"#,
        r#"function f(b as Lang.Boolean) as Rows {return b?[[1]]:[[2]];}"#,
        r#"function f(x as Rows) as Lang.Number {return x[0][0];}"#,
        r#"function f(x as Rows) {x[0]=[4,5]; x.add([]);x.addAll([[6]]);}"#,
        r#"function f(x as Rows) as Rows {return x.reverse().slice(null,1);}"#,
        r#"function f(x as Rows,y as Numbers) as Lang.Boolean {return x.remove(y);}"#,
        r#"function f(x as Rows,y as Numbers) as Lang.Boolean {return x.removeAll(y);}"#,
        r#"function f(x as Rows,y as Numbers) as Lang.Number {return x.indexOf(y);}"#,
        r#"function f() as Maps {return [{"a"=>1},{}];}"#,
        r#"function f(x as Maps) as Lang.Number or Null {x[0]["a"]=2;return x.reverse()[0]["a"];}"#,
        r#"function f(x as Maps) {x.add({});x.addAll([{"b"=>3}]);x[0]={"c"=>4};}"#,
        r#"function f(x as [Numbers],y as Rows) as Rows {x.addAll(y);return x;}"#,
        r#"function f(x as [Numbers],i as Lang.Number) as Rows {x[i]=[3];return x;}"#,
        r#"function f(x as [Map],i as Lang.Number) as Maps {x[i]={"a"=>1};return x;}"#,
        r#"function f() as Lang.Array<Lang.Array<Lang.Array<Lang.Number>>> {return [[[1]], [[]]];}"#,
        r#"function f() as Lang.Array<Numbers or Null> {return [[1],null,[]];}"#,
        r#"function f() as Lang.Array<Numbers or Null> {return new[2];}"#,
        r#"function f(x as [Numbers,Lang.String],i as Lang.Number) as Lang.Array<Numbers or Lang.String> {x[i]=[3];return x;}"#,
        r#"class Token {function value() as Lang.Number{return 41;}} function f(x as Lang.Array<[Token,Lang.String]>) as Lang.Number {return x[0][0].value();}"#,
        r#"class Box {var rows as Rows;function initialize(){self.rows=[[1]];}function f() as Lang.Number {return self.rows[0][0];}}"#,
    ] {
        check(s);
    }
}
#[test]
fn nested_writes_and_saved_contracts_remain_invariant_and_bounded() {
    for s in [
        r#"function f(x as Rows) {x[0]=["wrong"];}"#,
        r#"function f(x as Rows) {x.add([null]);}"#,
        r#"function f(x as Rows,y as Lang.Array<Lang.String>) {x.add(y);}"#,
        r#"function f(x as Rows,y as Lang.Array<Lang.Array<Lang.String>>) {x.addAll(y);}"#,
        r#"function f(x as Rows) as Lang.Array<Lang.Array<Lang.Object>> {return x;}"#,
        r#"function f(x as Maps) {x.add({"a"=>false});}"#,
        r#"function f() as Rows {return new[2];}"#,
    ] {
        assert!(compile_sources(&[PREFIX, s]).is_err(), "{s}");
    }
    let text = check("function f(x as Rows) as Lang.Number {return x[0][0]*2;}").to_ir_text();
    let forged = text.replace("nested(number-array)", "nested(string-array)");
    assert_ne!(text, forged);
    assert!(parse_unit_ir(&forged).is_err(), "{forged}");
    for bad in [
        "nested(integer)",
        "nested(number-array",
        "nested(number-array))",
    ] {
        assert!(parse_unit_ir(&text.replace("nested(number-array)", bad)).is_err());
    }
    let deep = format!(
        "{}number-array{}",
        "nested(".repeat(70),
        ")-array".repeat(70)
    );
    assert!(parse_unit_ir(&text.replace("nested(number-array)-array", &deep)).is_err());
}

#[test]
fn nested_observer_callbacks_cannot_skip_required_field_initialization() {
    let prefix = "using Toybox.Lang; class Token {function initialize(){}}";
    let source = "class Box {var token as Token;function initialize(rows as Lang.Array<Lang.Array<Lang.Object>>,token as Token){rows.toString();self.token=token;}}";
    let err = compile_sources(&[prefix, source]).unwrap_err().to_string();
    assert!(err.contains("shutdown"), "{err}");
    let safe = source.replace(
        "rows.toString();self.token=token;",
        "self.token=token;rows.toString();",
    );
    compile_sources(&[prefix, &safe]).unwrap();

    let source = source.replace(
        "Lang.Array<Lang.Array<Lang.Object>>",
        "Lang.Array<Lang.Array<Lang.Object> or Lang.Number>",
    );
    let err = compile_sources(&[prefix, &source]).unwrap_err().to_string();
    assert!(err.contains("shutdown"), "{err}");
}
