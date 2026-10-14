//! Bounded native name resolution before semantic IR construction.
use std::collections::BTreeMap;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeModule {
    Application,
    System,
    WatchUi,
    Graphics,
}

impl NativeModule {
    /// Return the canonical module name used by the current semantic profile.
    pub fn name(self) -> &'static str {
        match self {
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
    app_base: bool,
    view: bool,
    behavior_delegate: bool,
}

impl Names {
    /// Bind a supported Toybox module; imports also expose its supported classes.
    /// Reject unsupported modules and duplicate or reserved bindings.
    pub fn declare(&mut self, path: &str, alias: Option<&str>, import: bool) -> Result<(), String> {
        let module = match path {
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
            "self" | "Toybox" | "AppBase" | "View" | "BehaviorDelegate"
        ) || (name == "Application" && module != NativeModule::Application)
            || (name == "System" && module != NativeModule::System)
            || (name == "WatchUi" && module != NativeModule::WatchUi)
            || (name == "Graphics" && module != NativeModule::Graphics)
            || self.bindings.contains_key(name)
        {
            return Err(format!("conflicting native binding {name:?}"));
        }
        self.bindings.insert(name.into(), module);
        self.app_base |= import && module == NativeModule::Application;
        self.view |= import && module == NativeModule::WatchUi;
        self.behavior_delegate |= import && module == NativeModule::WatchUi;
        Ok(())
    }

    /// Check explicit module/class bindings before accepting a declaration name.
    pub fn bound(&self, name: &str) -> bool {
        self.bindings.contains_key(name)
            || (self.app_base && name == "AppBase")
            || (self.view && name == "View")
            || (self.behavior_delegate && name == "BehaviorDelegate")
    }

    /// Resolve an alias or supported canonical module path to a native identity.
    pub fn module(&self, name: &str) -> Option<NativeModule> {
        self.bindings.get(name).copied().or(match name {
            "System" | "Toybox.System" => Some(NativeModule::System),
            "Application" | "Toybox.Application" => Some(NativeModule::Application),
            "WatchUi" | "Toybox.WatchUi" => Some(NativeModule::WatchUi),
            "Graphics" | "Toybox.Graphics" => Some(NativeModule::Graphics),
            _ => None,
        })
    }

    /// Resolve the supported superclass spellings, including imported bare names.
    pub fn app_base(&self, path: &str) -> bool {
        if path == "AppBase" {
            return self.app_base;
        }
        path.rsplit_once('.').is_some_and(|(module, member)| {
            member == "AppBase" && self.module(module) == Some(NativeModule::Application)
        })
    }

    /// Resolve the bounded native View constructor, including imported bare names.
    pub fn view(&self, path: &str) -> bool {
        if path == "View" {
            return self.view;
        }
        path.rsplit_once('.').is_some_and(|(module, member)| {
            member == "View" && self.module(module) == Some(NativeModule::WatchUi)
        })
    }
    /// Resolve the supported behavior delegate base through WatchUi bindings.
    pub fn behavior_delegate(&self, path: &str) -> bool {
        if path == "BehaviorDelegate" {
            return self.behavior_delegate;
        }
        path.rsplit_once('.').is_some_and(|(module, member)| {
            member == "BehaviorDelegate" && self.module(module) == Some(NativeModule::WatchUi)
        })
    }
}
