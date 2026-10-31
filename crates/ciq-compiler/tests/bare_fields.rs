use ciq_compiler::compile_sources;
mod common;

#[test]
fn bare_fields_preserve_owners_and_complete_pipeline() {
    let sources = [
        include_str!("../../../examples/compiler-bare-fields/app.mc"),
        include_str!("../../../examples/compiler-bare-fields/box.mc"),
    ];
    let unit = compile_sources(&sources).unwrap();
    let text = unit.to_ir_text();
    assert!(text.contains(".read-self"));
    assert!(text.contains(".assign-self VALUE"));
    common::application(
        &unit,
        &ciq_compiler::application::ConsoleApplication {
            id: 0xc48b23478de04626a83175b99c6de201u128.to_be_bytes(),
            entry: "MinimalConsoleApp.onStart".into(),
        },
    );
    common::pipeline(compile_sources(&[sources[1], sources[0]]).unwrap());
}

#[test]
fn bare_and_explicit_field_operations_have_identical_vm_lowering() {
    let bare = "class C{var n as Toybox.Lang.Number=1;function f() as Toybox.Lang.Number{n=2;n+=3;++n;n--;return n;}}";
    let explicit = bare
        .replace("{n=", "{self.n=")
        .replace(";n+=", ";self.n+=")
        .replace("++n", "++self.n")
        .replace(";n--", ";self.n--")
        .replace("return n;", "return self.n;");
    let bare = compile_sources(&[bare]).unwrap();
    let explicit = compile_sources(&[&explicit]).unwrap();
    assert_eq!(bare.lower(), explicit.lower());
    common::pipeline(bare);
    common::pipeline(explicit);
}

#[test]
fn field_names_shadow_outer_bindings_before_contract_inference() {
    for source in [
        "const X=99;class C{var X=1;function f(){X=\"field\";return X.length();}}",
        "const X=99;class C{var A=X;var X=\"later\";}",
        "class C{var n as Toybox.Lang.Number;function initialize(n as Toybox.Lang.Number){self.n=n;}function f(){n++;}}",
        "class T{var value=1;}module item{const value=99;}class C{var item as T=new T();function f(){item.value=3;item.value++;return item.value;}}",
        "class C{var a as Toybox.Lang.Array<Toybox.Lang.Number> = [1];function f(){a[0]++;a.add(2);return a[0];}}",
        "class C{const X=1;function f(X as Toybox.Lang.Number){X++;return X;}function g(){var X=2;X=3;return X;}}",
        "class item{enum{value=99}}class T{var value=1;}class C{var item as T=new T();function f(){item.value=3;item.value++;return item.value;}}",
    ] {
        common::pipeline(compile_sources(&[source]).unwrap_or_else(|e| panic!("{source}: {e}")));
    }
    for source in [
        "const X=99;class C{var A=X;var X=1+2;}",
        "const X=99;class C{var X=Y;var Y=X;}",
        "class C{var n as Toybox.Lang.Number;function initialize(){System.println(n);n=1;}}",
        "class C{var n as Toybox.Lang.Number=1;function f(){n=\"bad\";}}",
        "class C{const X=1;function f(){X=2;}}",
        "class C{const X=1;function f(){X++;}}",
        "function missing(){return 99;}class C{var missing=1;function f(){return missing();}}",
    ] {
        assert!(compile_sources(&[source]).is_err(), "{source}");
    }
}

#[test]
fn pending_fields_do_not_borrow_outer_values_or_placeholder_number_types() {
    let unit = compile_sources(&["const X=99;class C{const A=X;var X=\"later\";}"]).unwrap();
    assert!(unit.to_ir_text().contains(".field-constant A any"));
    common::pipeline(unit);
}
