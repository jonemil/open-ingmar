use ciq_compiler::{
    FieldKind, NumericAlias,
    api::{ApiContext, ApiLevel},
    application::{ConsoleApplication, build_console_application},
    compile_sources, compile_sources_with_api, parse_unit_ir, parse_unit_ir_with_api,
};

#[test]
fn numeric_fields_preserve_complete_stages_and_null_metadata() {
    let context = ApiContext::new(ApiLevel(5, 2, 0), ApiLevel(2, 1, 0)).unwrap();
    let unit = compile_sources_with_api(
        &[include_str!(
            "../../../examples/compiler-numeric-fields/app.mc"
        )],
        context,
    )
    .unwrap();
    let text = unit.to_ir_text();
    let resumed = parse_unit_ir_with_api(&text, context).unwrap();
    assert_eq!(unit, resumed);
    for (alias, tag) in [
        (NumericAlias::Numeric, "numeric"),
        (NumericAlias::Integer, "integer-union"),
        (NumericAlias::Decimal, "decimal"),
    ] {
        assert!(unit.classes()[0].fields.iter().any(|field| field.kind
            == FieldKind::Numeric {
                alias,
                nullable: false
            }));
        assert!(text.contains(&format!(" {tag}\n")));
        assert!(text.contains(&format!(" nullable-{tag}\n")));
    }
    let asm = unit.lower();
    assert_eq!(asm, ciq_assembly::Program::parse(&asm.to_text()).unwrap());
    let options = ConsoleApplication {
        id: [93; 16],
        entry: "MinimalConsoleApp.onStart".into(),
    };
    let prg = build_console_application(&unit, &options).unwrap();
    assert_eq!(prg, build_console_application(&resumed, &options).unwrap());
    let records =
        ciq_prg::data::classes::records(prg.unique_section(ciq_prg::DATA).unwrap().payload())
            .unwrap();
    let fields = records
        .iter()
        .flat_map(|record| match record {
            ciq_prg::data::classes::Record::Class(class) => class.fields.as_slice(),
            _ => &[],
        })
        .filter(|field| field.kind == 0 && field.symbol_id() < 0x0080_0000)
        .collect::<Vec<_>>();
    assert_eq!(fields.len(), 6);
    assert!(fields.iter().all(|field| field.value == 0));
}

#[test]
fn aliases_imports_and_owner_signatures_survive_unit_reload() {
    for (name, tag) in [
        ("Numeric", "numeric"),
        ("Integer", "integer-union"),
        ("Decimal", "decimal"),
    ] {
        let source = format!(
            "using Toybox.Lang as L; class C {{var n as L.{name};var maybe as L.{name} or Null;function initialize(n as L.{name}){{self.n=n;}} function read() as L.{name} {{return self.n;}} function write(n as L.{name}) as Void {{self.n=n;}} function optional() as L.{name} or Null {{return self.maybe;}}}}"
        );
        let unit = compile_sources(&[&source]).unwrap();
        let text = unit.to_ir_text();
        assert_eq!(unit, parse_unit_ir(&text).unwrap());
        for edited in [
            text.replace(&format!(".field-numeric n {tag}\n"), ""),
            text.replace(
                &format!(".field-numeric n {tag}"),
                &format!(".field-numeric n nullable-{tag}"),
            ),
            text.replace(&format!(" n result={tag}"), " n result=integer"),
            text.replace(
                &format!(".field-numeric maybe nullable-{tag}"),
                &format!(".field-numeric maybe {tag}"),
            ),
            text.replace(
                &format!(".field-numeric n {tag}"),
                ".field-numeric n integer",
            ),
            text.replace(
                &format!(".field-numeric n {tag}"),
                ".field-numeric n nullable-nullable-numeric",
            ),
        ] {
            assert_ne!(text, edited);
            assert!(parse_unit_ir(&edited).is_err(), "accepted {edited}");
        }
    }
}

#[test]
fn numeric_alias_fields_require_complete_initialization_and_snapshot_guards() {
    for name in ["Numeric", "Integer", "Decimal"] {
        for body in [
            "var n as Lang.TYPE; function f() as Void {}",
            "var n as Lang.TYPE; function initialize() {}",
            "var n as Lang.TYPE=null; function initialize() {}",
            "var n as Lang.TYPE=1; function f() as Void {}",
            "var n as Lang.TYPE or Null=1; function f() as Void {}",
            "var n as Lang.TYPE; function initialize(n as Lang.TYPE or Null) {self.n=n;}",
            "var n as Lang.TYPE; function initialize(n as Lang.TYPE) {self.n=self.n;}",
            "var n as Lang.TYPE; function initialize(n as Lang.TYPE) {if(true){self.n=n;}}",
            "var n as Lang.TYPE; function initialize(n as Lang.TYPE) {self.f();self.n=n;} function f() as Void {}",
            "var n as Lang.TYPE or Null; function f() as Lang.TYPE {return self.n+1;}",
            "var n as Lang.TYPE or Null; function f() as Lang.TYPE {if(self.n!=null){return self.n+1;}return 0 as Lang.TYPE;}",
            "var n as Lang.TYPE; function initialize(n as Lang.TYPE){self.n=n;} function f() as Void {self.n=null;}",
        ] {
            let source = format!("class C {{{}}}", body.replace("TYPE", name));
            assert!(compile_sources(&[&source]).is_err(), "accepted {source}");
        }
        let source = format!(
            "class C {{var n as Lang.{name} or Null;function f() as Lang.{name} or Null {{var snapshot=self.n;if(snapshot!=null){{self.n=null;return snapshot+1;}}return null;}}}}"
        );
        let unit = compile_sources(&[&source]).unwrap();
        assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    }
    for (name, concrete) in [
        ("Numeric", "Number"),
        ("Integer", "Long"),
        ("Decimal", "Float"),
    ] {
        let source = format!(
            "class C {{var n as Lang.{name} or Null;function set(n as Lang.{concrete} or Null) as Void {{self.n=n;}}}}"
        );
        let unit = compile_sources(&[&source]).unwrap();
        assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    }
    for (name, expr) in [
        ("Integer", "1.5"),
        ("Integer", "1.5d"),
        ("Decimal", "1"),
        ("Decimal", "1l"),
        ("Numeric", "true"),
        ("Numeric", "'A'"),
    ] {
        let source =
            format!("class C {{var n as Lang.{name};function initialize(){{self.n={expr};}}}}");
        assert!(compile_sources(&[&source]).is_err(), "accepted {source}");
    }
}

#[test]
fn numeric_alias_fields_share_initialization_rules_across_supported_bases() {
    let early = ApiContext::new(ApiLevel(1, 0, 0), ApiLevel(1, 0, 0)).unwrap();
    let source = "class C {var n as Lang.Numeric or Null;function f() as Lang.Numeric or Null {return self.n;}}";
    let unit = compile_sources_with_api(&[source], early).unwrap();
    assert_eq!(
        unit,
        parse_unit_ir_with_api(&unit.to_ir_text(), early).unwrap()
    );
    for (base, initialize) in [
        ("Toybox.Lang.Object", ""),
        ("Toybox.Application.AppBase", "AppBase.initialize();"),
        ("Toybox.WatchUi.View", "View.initialize();"),
        (
            "Toybox.WatchUi.BehaviorDelegate",
            "BehaviorDelegate.initialize();",
        ),
    ] {
        let source = format!(
            "class C extends {base} {{var n as Lang.Numeric;function initialize(){{{initialize}self.n=1;}}function f() as Lang.Numeric {{return self.n;}}}}"
        );
        let unit = compile_sources(&[&source]).unwrap_or_else(|e| panic!("{source}: {e}"));
        assert_eq!(unit, parse_unit_ir(&unit.to_ir_text()).unwrap());
    }
}
