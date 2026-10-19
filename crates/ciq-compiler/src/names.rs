//! Bounded native name resolution before semantic IR construction.
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum NativeModule {
    Lang,
    Application,
    System,
    WatchUi,
    Graphics,
}

impl NativeModule {
    /// Return the canonical module name used by the current semantic profile.
    pub fn name(self) -> &'static str {
        match self {
            Self::Lang => "Lang",
            Self::Application => "Application",
            Self::System => "System",
            Self::WatchUi => "WatchUi",
            Self::Graphics => "Graphics",
        }
    }
}

/// Lexical module bindings; imports additionally expose native class names.
/// No symbol IDs or executable import indexes belong in this environment.
#[derive(Clone, Default)]
pub(crate) struct Names {
    bindings: BTreeMap<String, NativeModule>,
    imported: BTreeSet<NativeModule>,
}

impl Names {
    /// Bind a supported Toybox module; imports also expose its supported classes.
    /// Reject unsupported modules and duplicate or reserved bindings.
    pub fn declare(&mut self, path: &str, alias: Option<&str>, import: bool) -> Result<(), String> {
        let module = match path {
            "Toybox.Lang" => NativeModule::Lang,
            "Toybox.Application" => NativeModule::Application,
            "Toybox.System" => NativeModule::System,
            "Toybox.WatchUi" => NativeModule::WatchUi,
            "Toybox.Graphics" => NativeModule::Graphics,
            _ => return Err(format!("unsupported native module {path:?}")),
        };
        let declaration = crate::api::core()
            .module(path)
            .expect("supported module must have a catalogue declaration");
        let canonical_name = declaration.name.rsplit('.').next().unwrap();
        let name = alias.unwrap_or(canonical_name);
        if matches!(
            name,
            "self" | "Toybox" | "AppBase" | "View" | "BehaviorDelegate" | "Object"
        ) || (name == "Lang" && module != NativeModule::Lang)
            || (name == "Application" && module != NativeModule::Application)
            || (name == "System" && module != NativeModule::System)
            || (name == "WatchUi" && module != NativeModule::WatchUi)
            || (name == "Graphics" && module != NativeModule::Graphics)
            || self.bound(name)
            || (import
                && self
                    .bindings
                    .keys()
                    .any(|binding| Self::known_type(module, binding).is_some()))
        {
            return Err(format!("conflicting native binding {name:?}"));
        }
        self.bindings.insert(name.into(), module);
        if import {
            self.imported.insert(module);
        }
        Ok(())
    }

    /// Check explicit module/class bindings before accepting a declaration name.
    pub fn bound(&self, name: &str) -> bool {
        self.bindings.contains_key(name)
            || self
                .imported
                .iter()
                .any(|module| Self::known_type(*module, name).is_some())
    }

    /// Resolve a catalogue type exported directly by a supported module.
    /// This recognizes declarations, without claiming executable support.
    fn known_type(module: NativeModule, name: &str) -> Option<String> {
        let path = format!("Toybox.{}.{name}", module.name());
        let api = crate::api::core();
        (api.class(&path).is_some()
            || api.interface(&path).is_some()
            || api.enum_level(&path).is_some()
            || api.type_alias(&path).is_some())
        .then_some(path)
    }

    /// Resolve imported bare types and qualified module aliases to canonical
    /// catalogue names. Return an error for ambiguous imported type names.
    pub(crate) fn type_name(&self, path: &str) -> Result<Option<String>, String> {
        if let Some((module, name)) = path.rsplit_once('.') {
            return Ok(self
                .module(module)
                .and_then(|module| Self::known_type(module, name)));
        }
        let mut found = self
            .imported
            .iter()
            .filter_map(|module| Self::known_type(*module, path));
        let name = found.next();
        if found.next().is_some() {
            return Err(format!("ambiguous imported native type {path:?}"));
        }
        Ok(name)
    }

    /// Resolve an alias or supported canonical module path to a native identity.
    pub fn module(&self, name: &str) -> Option<NativeModule> {
        self.bindings.get(name).copied().or(match name {
            "Lang" | "Toybox.Lang" => Some(NativeModule::Lang),
            "System" | "Toybox.System" => Some(NativeModule::System),
            "Application" | "Toybox.Application" => Some(NativeModule::Application),
            "WatchUi" | "Toybox.WatchUi" => Some(NativeModule::WatchUi),
            "Graphics" | "Toybox.Graphics" => Some(NativeModule::Graphics),
            _ => None,
        })
    }

    /// Resolve the Object base through the shared imported/qualified type names.
    pub fn object(&self, path: &str) -> bool {
        self.type_name(path).ok().flatten().as_deref() == Some("Toybox.Lang.Object")
    }

    /// Resolve the supported AppBase superclass through the shared type names.
    pub fn app_base(&self, path: &str) -> bool {
        self.type_name(path).ok().flatten().as_deref() == Some("Toybox.Application.AppBase")
    }

    /// Resolve the supported View base/constructor through the shared type names.
    pub fn view(&self, path: &str) -> bool {
        self.type_name(path).ok().flatten().as_deref() == Some("Toybox.WatchUi.View")
    }

    /// Resolve the supported delegate base/constructor through shared type names.
    pub fn behavior_delegate(&self, path: &str) -> bool {
        self.type_name(path).ok().flatten().as_deref() == Some("Toybox.WatchUi.BehaviorDelegate")
    }
}
