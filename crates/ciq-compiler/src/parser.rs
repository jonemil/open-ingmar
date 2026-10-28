/// Classify source keywords independently of the implemented grammar subset.
/// IR/assembly identifiers have their own rules and do not use this restriction.
fn reserved_word(name: &str) -> bool {
    (name == crate::unit::FIELD_INITIALIZER || name.starts_with("__ciq_initialize_shared_"))
        || matches!(
            name,
            "alias"
                | "and"
                | "as"
                | "break"
                | "case"
                | "catch"
                | "class"
                | "const"
                | "continue"
                | "default"
                | "do"
                | "else"
                | "enum"
                | "extends"
                | "false"
                | "finally"
                | "for"
                | "function"
                | "has"
                | "hidden"
                | "if"
                | "import"
                | "instanceof"
                | "me"
                | "module"
                | "NaN"
                | "native"
                | "new"
                | "null"
                | "or"
                | "private"
                | "protected"
                | "public"
                | "return"
                | "self"
                | "static"
                | "switch"
                | "throw"
                | "true"
                | "try"
                | "typedef"
                | "using"
                | "var"
                | "while"
        )
}

use crate::{Diagnostic, Span};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

const NAN_LITERAL_BITS: u32 = 0x7fc00000;

// SourceClass bases in parsed annotations are placeholders. Unit declaration
// collection resolves them before signatures or bodies enter semantic IR.
pub(crate) struct ParsedFunction {
    pub name: String,
    pub parameters: Vec<String>,
    pub parameter_types: Vec<Option<crate::ParameterType>>,
    pub result_type: Option<crate::ReturnType>,
    pub statements: Vec<Statement>,
    pub body_span: Span,
}

pub(crate) enum Statement {
    MemberAssign {
        receiver: Expression,
        member: String,
        value: Expression,
        span: Span,
    },
    Update {
        target: Expression,
        increment: bool,
        span: Span,
    },
    PlaceCompound {
        target: Expression,
        kind: crate::BinaryOp,
        value: Expression,
        span: Span,
    },
    ArrayAssign {
        array: Expression,
        index: Expression,
        value: Expression,
        span: Span,
    },
    FieldAssign {
        name: String,
        value: Expression,
        span: Span,
    },
    Local(LocalDeclaration),
    Println {
        argument: Expression,
        span: Span,
    },
    BaseInitialize {
        span: Span,
        base: crate::ClassBase,
    },
    Assign {
        name: String,
        value: Expression,
        span: Span,
    },
    Call {
        module: String,
        member: String,
        arguments: Vec<Expression>,
        span: Span,
    },
    ValueCall {
        expression: Expression,
        span: Span,
    },
    NativeModuleCall {
        module: String,
        member: String,
        arguments: Vec<Expression>,
        span: Span,
    },
    Return {
        value: Option<Expression>,
        span: Span,
    },
    Break {
        span: Span,
    },
    Continue {
        span: Span,
    },
    Switch {
        selector: Expression,
        cases: Vec<SwitchCase>,
        span: Span,
    },
    Block {
        body: Vec<Statement>,
        span: Span,
    },
    For {
        initialize: Vec<Statement>,
        condition: Option<Expression>,
        update: Vec<Statement>,
        body: Vec<Statement>,
        span: Span,
    },
    DoWhile {
        body: Vec<Statement>,
        condition: Expression,
        span: Span,
    },
    While {
        condition: Expression,
        body: Vec<Statement>,
        span: Span,
    },
    If {
        condition: Expression,
        on_true: Vec<Statement>,
        on_false: Vec<Statement>,
        span: Span,
    },
}

impl Statement {
    /// Return this statement's immediate expressions, including case patterns.
    /// Nested statement bodies are visited separately by their control-flow owner.
    pub(crate) fn expressions(&self) -> Vec<&Expression> {
        match self {
            Self::MemberAssign {
                receiver, value, ..
            } => vec![receiver, value],
            Self::Update { target, .. } => vec![target],
            Self::PlaceCompound { target, value, .. } => vec![target, value],
            Self::ArrayAssign {
                array,
                index,
                value,
                ..
            } => vec![array, index, value],
            Self::FieldAssign { value, .. } | Self::Assign { value, .. } => vec![value],
            Self::Local(local) => local.initializer.iter().collect(),
            Self::Println { argument, .. } => vec![argument],
            Self::Call { arguments, .. } | Self::NativeModuleCall { arguments, .. } => {
                arguments.iter().collect()
            }
            Self::ValueCall { expression, .. } => vec![expression],
            Self::Return { value, .. } => value.iter().collect(),
            Self::If { condition, .. }
            | Self::While { condition, .. }
            | Self::DoWhile { condition, .. } => vec![condition],
            Self::For { condition, .. } => condition.iter().collect(),
            Self::Switch {
                selector, cases, ..
            } => std::iter::once(selector)
                .chain(cases.iter().filter_map(|case| {
                    if let CasePattern::Value(value) = &case.pattern {
                        Some(value)
                    } else {
                        None
                    }
                }))
                .collect(),
            Self::BaseInitialize { .. }
            | Self::Break { .. }
            | Self::Continue { .. }
            | Self::Block { .. } => vec![],
        }
    }

    pub fn span(&self) -> Span {
        match self {
            Self::Local(local) => local.span,
            Self::Update { span, .. } => *span,
            Self::MemberAssign { span, .. }
            | Self::FieldAssign { span, .. }
            | Self::ArrayAssign { span, .. }
            | Self::PlaceCompound { span, .. } => *span,
            Self::Println { span, .. }
            | Self::Assign { span, .. }
            | Self::Return { span, .. }
            | Self::Call { span, .. }
            | Self::ValueCall { span, .. }
            | Self::NativeModuleCall { span, .. }
            | Self::If { span, .. }
            | Self::Break { span }
            | Self::Continue { span }
            | Self::Switch { span, .. }
            | Self::Block { span, .. }
            | Self::While { span, .. }
            | Self::For { span, .. }
            | Self::DoWhile { span, .. } => *span,
            Self::BaseInitialize { span, .. } => *span,
        }
    }
}

pub(crate) struct LocalDeclaration {
    pub name: String,
    pub initializer: Option<Expression>,
    pub span: Span,
}

pub(crate) struct SwitchCase {
    pub pattern: CasePattern,
    pub body: Vec<Statement>,
    pub span: Span,
}

pub(crate) enum CasePattern {
    Value(Expression),
    Class(String),
    Default,
}

#[derive(Clone)]
pub(crate) struct Expression {
    pub kind: ExpressionKind,
    pub span: Span,
}
/// Zero-argument native constructor targets, separate from expression value types.
#[derive(Clone)]
pub(crate) enum NativeConstructor {
    Object(crate::NativeClass),
    WeakReference,
}

#[derive(Clone)]
pub(crate) enum ExpressionKind {
    MemberRead {
        receiver: Box<Expression>,
        member: String,
    },
    Symbol(String),
    DictionaryNew,
    DictionaryLiteral(Vec<(Expression, Expression)>),
    Conditional {
        condition: Box<Expression>,
        when_true: Box<Expression>,
        when_false: Box<Expression>,
    },
    // Builder-only incoming value at a typed join; consumed in its join block.
    Joined(crate::ValueId),
    // Builder-only numeric step; source updates are statements, not values.
    Step {
        operand: Box<Expression>,
        increment: bool,
    },
    SelfValue,
    SelfField(String),
    SharedField {
        module: Option<String>,
        field: String,
    },
    SourceNew {
        class: String,
        arguments: Vec<Expression>,
    },
    NativeConstant {
        module: String,
        member: String,
        qualifier: String,
    },
    NativeNew {
        class: NativeConstructor,
        qualifier: String,
    },
    ArrayLiteral(Vec<Expression>),
    ByteArrayLiteral(Vec<Expression>),
    ArrayAllocate(Box<Expression>),
    ByteArrayAllocate(Box<Expression>),
    ArrayRead {
        array: Box<Expression>,
        index: Box<Expression>,
    },
    // Builder-only read of an evaluated temporary; never parsed from source.
    Temporary(crate::LocalId),
    Logical {
        and: bool,
        left: Box<Expression>,
        right: Box<Expression>,
    },
    Char(u32),
    Positive(Box<Expression>),
    Negate(Box<Expression>),
    Not(Box<Expression>),
    Complement(Box<Expression>),
    Null,
    Boolean(bool),
    Integer(i32),
    Long(i64),
    Double(u64),
    Float(u32),
    String(String),
    Local(String),
    UnqualifiedCall {
        member: String,
        arguments: Vec<Expression>,
    },
    GlobalCall {
        member: String,
        arguments: Vec<Expression>,
    },
    // Fixed owner identity; lexical values cannot shadow this qualifier.
    OwnerCall {
        module: String,
        member: String,
        arguments: Vec<Expression>,
    },
    ModuleCall {
        module: String,
        member: String,
        arguments: Vec<Expression>,
    },
    ValueCall {
        receiver: Box<Expression>,
        member: String,
        arguments: Vec<Expression>,
    },
    Binary {
        kind: crate::BinaryOp,
        left: Box<Expression>,
        right: Box<Expression>,
    },
    Cast {
        operand: Box<Expression>,
        target: crate::api::Type,
    },
    InstanceOf {
        operand: Box<Expression>,
        class: String,
    },
    Has {
        receiver: Box<Expression>,
        symbol: Box<Expression>,
    },
    Compare {
        kind: crate::ComparisonOp,
        left: Box<Expression>,
        right: Box<Expression>,
    },
}

impl Expression {
    /// Visit an expression and its children without resolving names or types.
    /// The shared traversal lets source staging and parameter mutation discovery
    /// recognize calls inside ordinary expression trees consistently.
    pub(crate) fn visit<'a>(&'a self, visitor: &mut impl FnMut(&'a Self)) {
        use ExpressionKind as E;
        visitor(self);
        match &self.kind {
            E::MemberRead { receiver, .. } => receiver.visit(visitor),
            E::Conditional {
                condition,
                when_true,
                when_false,
            } => {
                condition.visit(visitor);
                when_true.visit(visitor);
                when_false.visit(visitor);
            }
            E::Step { operand, .. }
            | E::Cast { operand, .. }
            | E::InstanceOf { operand, .. }
            | E::Positive(operand)
            | E::Negate(operand)
            | E::Not(operand)
            | E::Complement(operand)
            | E::ArrayAllocate(operand)
            | E::ByteArrayAllocate(operand) => operand.visit(visitor),
            E::ArrayRead {
                array: left,
                index: right,
            }
            | E::Logical { left, right, .. }
            | E::Binary { left, right, .. }
            | E::Compare { left, right, .. }
            | E::Has {
                receiver: left,
                symbol: right,
            } => {
                left.visit(visitor);
                right.visit(visitor);
            }
            E::ValueCall {
                receiver,
                arguments,
                ..
            } => {
                receiver.visit(visitor);
                for argument in arguments {
                    argument.visit(visitor);
                }
            }
            E::SourceNew { arguments, .. }
            | E::UnqualifiedCall { arguments, .. }
            | E::GlobalCall { arguments, .. }
            | E::OwnerCall { arguments, .. }
            | E::ModuleCall { arguments, .. }
            | E::ArrayLiteral(arguments)
            | E::ByteArrayLiteral(arguments) => {
                for argument in arguments {
                    argument.visit(visitor);
                }
            }
            E::DictionaryLiteral(entries) => {
                for (key, value) in entries {
                    key.visit(visitor);
                    value.visit(visitor);
                }
            }
            E::Symbol(_)
            | E::DictionaryNew
            | E::Joined(_)
            | E::SelfValue
            | E::SelfField(_)
            | E::NativeConstant { .. }
            | E::NativeNew { .. }
            | E::Temporary(_)
            | E::Char(_)
            | E::Null
            | E::Boolean(_)
            | E::Integer(_)
            | E::Long(_)
            | E::Double(_)
            | E::Float(_)
            | E::String(_)
            | E::Local(_)
            | E::SharedField { .. } => {}
        }
    }

    /// Report whether this tree needs control flow or staged array construction.
    pub(crate) fn needs_materialization(&self) -> bool {
        match &self.kind {
            ExpressionKind::MemberRead { receiver, .. } => receiver.needs_materialization(),
            ExpressionKind::SourceNew { .. }
            | ExpressionKind::Conditional { .. }
            | ExpressionKind::Logical { .. }
            | ExpressionKind::ArrayLiteral(_)
            | ExpressionKind::ByteArrayLiteral(_)
            | ExpressionKind::ArrayAllocate(_)
            | ExpressionKind::ByteArrayAllocate(_)
            | ExpressionKind::DictionaryNew
            | ExpressionKind::DictionaryLiteral(_) => true,
            ExpressionKind::Cast { operand, .. }
            | ExpressionKind::InstanceOf { operand, .. }
            | ExpressionKind::Step { operand, .. } => operand.needs_materialization(),
            ExpressionKind::Not(operand)
            | ExpressionKind::Complement(operand)
            | ExpressionKind::Positive(operand)
            | ExpressionKind::Negate(operand) => operand.needs_materialization(),
            ExpressionKind::Binary { left, right, .. }
            | ExpressionKind::Has {
                receiver: left,
                symbol: right,
            }
            | ExpressionKind::Compare { left, right, .. } => {
                left.needs_materialization() || right.needs_materialization()
            }
            ExpressionKind::ValueCall {
                receiver,
                arguments,
                ..
            } => {
                receiver.needs_materialization()
                    || arguments.iter().any(Self::needs_materialization)
            }
            ExpressionKind::ArrayRead { array, index } => {
                array.needs_materialization() || index.needs_materialization()
            }
            ExpressionKind::UnqualifiedCall { arguments, .. }
            | ExpressionKind::GlobalCall { arguments, .. }
            | ExpressionKind::OwnerCall { arguments, .. }
            | ExpressionKind::ModuleCall { arguments, .. } => {
                arguments.iter().any(Self::needs_materialization)
            }
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind<'a> {
    Char(u32),
    Word(&'a str),
    Integer(&'a str),
    Long(&'a str),
    Real(&'a str, bool),
    String(&'a str),
    Punctuation(char),
    EqualEqual,
    NotEqual,
    LessEqual,
    GreaterEqual,
    ShiftLeft,
    ShiftRight,
    Compound(crate::BinaryOp),
    Step(bool),
    AndAnd,
    OrOr,
    End,
}

struct Token<'a> {
    kind: Kind<'a>,
    span: Span,
}

struct Parser<'a> {
    source: &'a str,
    source_index: usize,
    offset: usize,
    lookahead: Option<Token<'a>>,
    expressions: usize,
    names: crate::names::Names,
    native_base: Option<crate::ClassBase>,
    declarations: Rc<Declarations<'a>>,
    owner: Option<String>,
    resolving: Vec<String>,
    type_depth: usize,
}

struct DeclarationText<'a> {
    source: &'a str,
    source_index: usize,
    name_at: usize,
    target: usize,
    end: usize,
    names: crate::names::Names,
    owner: Option<String>,
}

#[derive(Default)]
pub(crate) struct Declarations<'a> {
    constants: BTreeMap<String, DeclarationText<'a>>,
    value_names: BTreeSet<String>,
    constant_values: RefCell<BTreeMap<String, crate::constants::Immediate>>,

    aliases: BTreeMap<String, DeclarationText<'a>>,
    classes: BTreeSet<String>,
    owners: BTreeSet<String>,
    resolved: RefCell<BTreeMap<String, crate::ParameterType>>,
}

impl Declarations<'_> {
    pub(crate) fn immediate_constants(&self) -> Rc<BTreeMap<String, crate::constants::Immediate>> {
        Rc::new(
            self.constant_values
                .borrow()
                .iter()
                .filter(|(key, _)| {
                    !key.split_once('.')
                        .is_some_and(|(owner, _)| self.classes.contains(owner))
                })
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
        )
    }

    pub(crate) fn validate_constant_members(
        &self,
        bases: &BTreeMap<String, crate::ClassBase>,
    ) -> Result<(), crate::SourceDiagnostic> {
        for (key, declaration) in &self.constants {
            if let Some((owner, name)) = key.split_once('.') {
                if bases.get(owner).is_some_and(|base| {
                    name == "initialize"
                        || crate::api::core().method(base.api_name(), name).is_some()
                }) {
                    return Err(crate::SourceDiagnostic {
                        source: declaration.source_index,
                        diagnostic: Diagnostic::at(
                            declaration.source,
                            declaration.name_at,
                            "constant conflicts with an inherited method or constructor",
                        ),
                    });
                }
            }
        }
        Ok(())
    }
    pub(crate) fn validate_api(
        &self,
        context: crate::api::ApiContext,
    ) -> Result<(), crate::SourceDiagnostic> {
        for (key, alias) in &self.aliases {
            let ty = self.resolved.borrow()[key].api_type();
            context
                .check_type(crate::api::core(), &ty)
                .map_err(|message| crate::SourceDiagnostic {
                    source: alias.source_index,
                    diagnostic: Diagnostic::at(alias.source, alias.name_at, message),
                })?;
        }
        for (key, declaration) in &self.constants {
            if !self.constant_values.borrow().contains_key(key) {
                continue;
            }
            context
                .check_type(
                    crate::api::core(),
                    &self.constant_values.borrow()[key].api_type(),
                )
                .map_err(|message| crate::SourceDiagnostic {
                    source: declaration.source_index,
                    diagnostic: Diagnostic::at(declaration.source, declaration.name_at, message),
                })?;
        }
        Ok(())
    }
}

impl<'a> Parser<'a> {
    fn new(source: &'a str, declarations: Rc<Declarations<'a>>) -> Self {
        Self {
            source,
            source_index: 0,
            offset: 0,
            lookahead: None,
            expressions: 0,
            names: crate::names::Names::default(),
            native_base: None,
            declarations,
            owner: None,
            resolving: Vec::new(),
            type_depth: 0,
        }
    }

    /// Parse a constant's optional type and initializer, leaving its list
    /// delimiter untouched. Missing initializers have the ordinary Null default.
    fn constant_tail(&mut self, name: String, span: Span) -> Result<ParsedConstant, Diagnostic> {
        let annotation = if self.peek()? == Kind::Word("as") {
            Some(self.structural_member_type(span.start, false)?)
        } else {
            None
        };
        let initializer = if self.peek()? == Kind::Punctuation('=') {
            self.token()?;
            self.expression(0)?
        } else {
            Expression {
                kind: ExpressionKind::Null,
                span,
            }
        };
        Ok(ParsedConstant {
            name,
            annotation,
            initializer,
            span,
        })
    }

    /// Parse one constant declaration list through the shared value/type grammar.
    /// Class fields are retained; only cached unannotated global literals vanish.
    fn constant_declarations(&mut self) -> Result<Vec<ParsedConstant>, Diagnostic> {
        self.expect(Kind::Word("const"), "const")?;
        let mut declarations = Vec::new();
        loop {
            let token = self.identifier()?;
            self.reject_binding(&token)?;
            let Kind::Word(name) = token.kind else {
                unreachable!()
            };
            let declaration = self.constant_tail(name.into(), token.span)?;
            let key = self
                .owner
                .as_ref()
                .map_or_else(|| name.to_owned(), |owner| format!("{owner}.{name}"));
            let class_field = self
                .owner
                .as_ref()
                .is_some_and(|owner| self.declarations.classes.contains(owner));
            if class_field
                || !self
                    .declarations
                    .constant_values
                    .borrow()
                    .contains_key(&key)
            {
                declarations.push(declaration);
            }
            match self.token()?.kind {
                Kind::Punctuation(',') => {}
                Kind::Punctuation(';') => return Ok(declarations),
                _ => return Err(self.error(self.offset, "expected ',' or ';' after constant")),
            }
        }
    }

    fn alias_key(&self, path: &str) -> Option<String> {
        let owned = self.owner.as_ref().map(|owner| format!("{owner}.{path}"));
        owned
            .filter(|key| self.declarations.aliases.contains_key(key))
            .or_else(|| {
                self.declarations
                    .aliases
                    .contains_key(path)
                    .then(|| path.into())
            })
    }

    /// Resolve an alias in its declaration's native-name and owner environment.
    /// Reuse annotation parsing so alias targets have no separate type grammar.
    fn alias_parameter(
        &self,
        path: &str,
        at: usize,
    ) -> Result<Option<crate::ParameterType>, Diagnostic> {
        let Some(key) = self.alias_key(path) else {
            return Ok(None);
        };
        if let Some(ty) = self.declarations.resolved.borrow().get(&key) {
            return Ok(Some(ty.clone()));
        }
        if let Some(index) = self.resolving.iter().rposition(|name| name == &key) {
            return Ok(Some(crate::ParameterType::Recursive(
                crate::api::Type::RecursionRef(self.resolving.len() - index - 1),
            )));
        }
        if self.resolving.len() >= 64 {
            return Err(self.error(at, "typedef expansion exceeds 64 levels"));
        }
        let alias = &self.declarations.aliases[&key];
        let mut parser = Self::new(alias.source, self.declarations.clone());
        parser.source_index = alias.source_index;
        parser.offset = alias.target;
        parser.names = alias.names.clone();
        parser.owner = alias.owner.clone();
        parser.resolving = self.resolving.clone();
        parser.resolving.push(key.clone());
        parser.type_depth = self.type_depth;
        let result = (|| {
            let ty = parser.parameter_annotation(alias.target)?;
            let mut ty = crate::unions::parameter(crate::recursive::bind(ty.exact_type()))
                .ok_or_else(|| parser.error(alias.target, "unsupported recursive typedef body"))?;
            if self.resolving.is_empty() {
                crate::recursive::validate(&ty.exact_type()).map_err(|message| {
                    parser.error(alias.target, format!("recursive typedef: {message}"))
                })?;
            }
            let end = parser.expect(Kind::Punctuation(';'), "';' after typedef")?;
            if end.span.start != alias.end {
                return Err(parser.error(end.span.start, "invalid typedef target"));
            }
            crate::structural::resolve_parameter(&mut ty, &mut |class| {
                if self.declarations.classes.contains(&class.name) {
                    Ok(())
                } else {
                    Err(parser.error(
                        alias.target,
                        format!("unknown typedef target class {:?}", class.name),
                    ))
                }
            })?;
            Ok(ty)
        })();
        match result {
            Ok(ty) => {
                if crate::recursive::validate(&ty.exact_type()).is_ok() {
                    self.declarations
                        .resolved
                        .borrow_mut()
                        .insert(key, ty.clone());
                }
                Ok(Some(ty))
            }
            Err(error) => Err(self.error(at, format!("typedef {key:?}: {}", error.message))),
        }
    }

    /// Skip a balanced body while collecting declarations, without interpreting it.
    fn skip_body(&mut self) -> Result<(), Diagnostic> {
        let mut parentheses = 0usize;
        let mut type_nesting = 0usize;
        let mut type_start = false;
        loop {
            if self.peek()? == Kind::Punctuation('{') {
                if parentheses > 0 || type_nesting > 0 || type_start {
                    self.skip_braces()?;
                    type_start = false;
                    continue;
                }
                break;
            }
            if self.peek()? == Kind::Word("interface") {
                self.token()?;
                self.skip_braces()?;
                type_start = false;
                continue;
            }
            match self.token()?.kind {
                Kind::Punctuation('(') => parentheses += 1,
                Kind::Punctuation(')') => parentheses = parentheses.saturating_sub(1),
                Kind::Punctuation('<' | '[') => type_nesting += 1,
                Kind::Punctuation('>' | ']') | Kind::GreaterEqual => {
                    type_nesting = type_nesting.saturating_sub(1);
                }
                Kind::ShiftRight => type_nesting = type_nesting.saturating_sub(2),
                Kind::Word("as" | "or") => {
                    type_start = true;
                    continue;
                }
                Kind::End => return Err(self.error(self.offset, "expected function body")),
                _ => {}
            }
            type_start = false;
        }
        self.skip_braces()
    }

    fn skip_braces(&mut self) -> Result<(), Diagnostic> {
        self.expect(Kind::Punctuation('{'), "'{'")?;
        let mut depth = 1usize;
        while depth != 0 {
            match self.token()?.kind {
                Kind::Punctuation('{') => depth += 1,
                Kind::Punctuation('}') => depth -= 1,
                Kind::End => return Err(self.error(self.offset, "unterminated body")),
                _ => {}
            }
        }
        Ok(())
    }

    fn skip_typedef(&mut self) -> Result<(), Diagnostic> {
        self.expect(Kind::Word("typedef"), "'typedef'")?;
        let name = self.identifier()?;
        let Kind::Word(name) = name.kind else {
            unreachable!()
        };
        // Collection already checked the full target, including nested member
        // semicolons. Advance past its declaration terminator, not the first ';'.
        let key = self.alias_key(name).expect("collected typedef");
        self.offset = self.declarations.aliases[&key].end + 1;
        self.lookahead = None;
        Ok(())
    }
}

/// Collect typedefs and scalar constants before parsing annotations and bodies.
/// Capture declaration environments, reject duplicates and invalid dependencies,
/// and validate unused declarations. Function bodies are parsed only by the ordinary parser.
pub(crate) fn collect_declarations<'a>(
    sources: &[&'a str],
) -> Result<Rc<Declarations<'a>>, crate::SourceDiagnostic> {
    let mut aliases = Declarations::default();
    for (source_index, source) in sources.iter().enumerate() {
        let mut parser = Parser::new(source, Rc::default());
        parser.source_index = source_index;
        let result = (|| {
            parser.imports()?;
            let mut outer_names = None;
            while parser.peek()? != Kind::End {
                match parser.peek()? {
                    Kind::Word("const") => {
                        parser.token()?;
                        loop {
                            let name = parser.identifier()?;
                            parser.reject_binding(&name)?;
                            let Kind::Word(name_text) = name.kind else {
                                unreachable!()
                            };
                            let key = parser.owner.as_ref().map_or_else(
                                || name_text.to_owned(),
                                |owner| format!("{owner}.{name_text}"),
                            );
                            // Collection precedes typedef resolution. Balance raw type
                            // tokens until '=', then balance only expression delimiters.
                            let target = name.span.end;
                            let mut depth = 0usize;
                            let mut angles = 0usize;
                            let mut initializer = false;
                            let mut type_context = Some(0usize);
                            let mut previous_word = None;
                            let (end, more) = loop {
                                let token = parser.token()?;
                                match token.kind {
                                    Kind::Punctuation(',' | ';') if depth == 0 && angles == 0 => {
                                        break (
                                            token.span.start,
                                            token.kind == Kind::Punctuation(','),
                                        );
                                    }
                                    Kind::Punctuation('(' | '[' | '{') => depth += 1,
                                    Kind::Punctuation(')' | ']' | '}') => {
                                        depth = depth.checked_sub(1).ok_or_else(|| {
                                            parser.error(
                                                token.span.start,
                                                "unbalanced constant declaration",
                                            )
                                        })?;
                                        if type_context.is_some_and(|start| depth < start) {
                                            type_context = None;
                                        }
                                    }
                                    Kind::Word("as") => type_context = Some(depth),
                                    Kind::Punctuation(',') if angles == 0 => type_context = None,
                                    Kind::Punctuation('<')
                                        if !initializer
                                            || angles != 0
                                            || (type_context.is_some()
                                                && matches!(
                                                    previous_word,
                                                    Some("Array" | "Dictionary")
                                                )) =>
                                    {
                                        angles += 1
                                    }
                                    Kind::Punctuation('>') if angles != 0 => angles -= 1,
                                    Kind::GreaterEqual if angles != 0 => {
                                        angles -= 1;
                                        if !initializer && depth == 0 && angles == 0 {
                                            initializer = true;
                                            type_context = None;
                                        }
                                    }
                                    Kind::ShiftRight
                                    | Kind::Compound(crate::BinaryOp::ShiftRight)
                                        if angles != 0 =>
                                    {
                                        angles = angles.saturating_sub(2);
                                        if !initializer
                                            && depth == 0
                                            && angles == 0
                                            && matches!(token.kind, Kind::Compound(_))
                                        {
                                            initializer = true;
                                            type_context = None;
                                        }
                                    }
                                    Kind::Punctuation('=') if depth == 0 && angles == 0 => {
                                        initializer = true;
                                        type_context = None;
                                    }
                                    Kind::End => {
                                        return Err(parser
                                            .error(target, "unterminated constant declaration"));
                                    }
                                    _ => {}
                                }
                                previous_word = match token.kind {
                                    Kind::Word(word) => Some(word),
                                    _ => None,
                                };
                                if depth + angles > 64 {
                                    return Err(parser.error(
                                        token.span.start,
                                        "constant declaration exceeds 64 levels",
                                    ));
                                }
                            };
                            if aliases.constants.len() >= 256 {
                                return Err(parser.error(
                                    name.span.start,
                                    "unit exceeds 256 constant declarations",
                                ));
                            }
                            if aliases
                                .constants
                                .insert(
                                    key.clone(),
                                    DeclarationText {
                                        source,
                                        source_index,
                                        name_at: name.span.start,
                                        target,
                                        end,
                                        names: parser.names.clone(),
                                        owner: parser.owner.clone(),
                                    },
                                )
                                .is_some()
                            {
                                return Err(parser.error(
                                    name.span.start,
                                    format!("duplicate constant {key:?}"),
                                ));
                            }
                            if !more {
                                break;
                            }
                        }
                    }
                    Kind::Word("typedef") => {
                        parser.token()?;
                        let name = parser.identifier()?;
                        let Kind::Word(name_text) = name.kind else {
                            unreachable!()
                        };
                        let key = parser.owner.as_ref().map_or_else(
                            || name_text.into(),
                            |owner| format!("{owner}.{name_text}"),
                        );
                        parser.expect(Kind::Word("as"), "'as' after typedef name")?;
                        let target = parser.offset;
                        let mut braces = 0usize;
                        let end = loop {
                            let token = parser.token()?;
                            match token.kind {
                                Kind::Punctuation(';') if braces == 0 => break token.span.start,
                                Kind::Punctuation('{') => braces += 1,
                                Kind::Punctuation('}') => {
                                    braces = braces.checked_sub(1).ok_or_else(|| {
                                        parser.error(token.span.start, "unbalanced typedef target")
                                    })?
                                }
                                Kind::End => {
                                    return Err(parser.error(
                                        token.span.start,
                                        "unsupported or unterminated typedef target",
                                    ));
                                }
                                _ => {}
                            }
                        };
                        if aliases.aliases.len() >= 256 {
                            return Err(parser
                                .error(name.span.start, "unit exceeds 256 typedef declarations"));
                        }
                        if aliases
                            .aliases
                            .insert(
                                key.clone(),
                                DeclarationText {
                                    source,
                                    source_index,
                                    name_at: name.span.start,
                                    target,
                                    end,
                                    names: parser.names.clone(),
                                    owner: parser.owner.clone(),
                                },
                            )
                            .is_some()
                        {
                            return Err(
                                parser.error(name.span.start, format!("duplicate typedef {key:?}"))
                            );
                        }
                    }
                    Kind::Word("module" | "class") if parser.owner.is_none() => {
                        let class = parser.token()?.kind == Kind::Word("class");
                        let name = parser.identifier()?;
                        let Kind::Word(name) = name.kind else {
                            unreachable!()
                        };
                        aliases.owners.insert(name.into());
                        if class {
                            aliases.classes.insert(name.into());
                        }
                        while parser.peek()? != Kind::Punctuation('{') {
                            if parser.token()?.kind == Kind::End {
                                return Err(parser.error(parser.offset, "expected owner body"));
                            }
                        }
                        parser.token()?;
                        parser.owner = Some(name.into());
                        outer_names = Some(parser.names.clone());
                        parser.imports()?;
                    }
                    Kind::Punctuation('}') if parser.owner.is_some() => {
                        parser.token()?;
                        parser.owner = None;
                        parser.names = outer_names.take().unwrap();
                    }
                    Kind::Word("function") => {
                        parser.token()?;
                        let name = parser.identifier()?;
                        let Kind::Word(name) = name.kind else {
                            unreachable!()
                        };
                        aliases.value_names.insert(
                            parser
                                .owner
                                .as_ref()
                                .map_or_else(|| name.to_owned(), |owner| format!("{owner}.{name}")),
                        );
                        parser.skip_body()?;
                    }
                    Kind::Word("var") if parser.owner.is_some() => {
                        parser.token()?;
                        let name = parser.identifier()?;
                        let Kind::Word(name) = name.kind else {
                            unreachable!()
                        };
                        aliases.value_names.insert(
                            parser
                                .owner
                                .as_ref()
                                .map_or_else(|| name.to_owned(), |owner| format!("{owner}.{name}")),
                        );
                        // The main parser checks the supported field syntax.
                        while parser.token()?.kind != Kind::Punctuation(';') {
                            if parser.offset >= source.len() {
                                return Err(parser.error(parser.offset, "unterminated field"));
                            }
                        }
                    }
                    _ => return Err(parser.error(parser.offset, "expected declaration")),
                }
            }
            Ok(())
        })();
        result.map_err(|diagnostic| crate::SourceDiagnostic {
            source: source_index,
            diagnostic,
        })?;
    }
    let aliases = Rc::new(aliases);
    for (key, alias) in &aliases.aliases {
        let mut parser = Parser::new(alias.source, aliases.clone());
        parser.source_index = alias.source_index;
        parser.owner = alias.owner.clone();
        let result = if aliases.owners.contains(key) {
            Err(parser.error(alias.name_at, "typedef conflicts with a class or module"))
        } else {
            parser.alias_parameter(key, alias.name_at).map(|_| ())
        };
        result.map_err(|diagnostic| crate::SourceDiagnostic {
            source: alias.source_index,
            diagnostic,
        })?;
    }
    // Cache direct non-heap global/class literals. Module members always need
    // shared storage: even a scalar literal read must activate its owner and
    // run other initializers. References/operators likewise retain real slots.
    for (key, declaration) in &aliases.constants {
        let mut parser = Parser::new(declaration.source, aliases.clone());
        parser.owner = declaration.owner.clone();
        parser.names = declaration.names.clone();
        parser.offset = declaration.target;
        let result = (|| {
            if aliases.owners.contains(key) || aliases.value_names.contains(key) {
                return Err(parser.error(
                    declaration.name_at,
                    format!("constant conflicts with value declaration {key:?}"),
                ));
            }
            let constant = parser.constant_tail(
                key.rsplit('.').next().unwrap().into(),
                Span {
                    start: declaration.name_at,
                    end: declaration.target,
                },
            )?;
            let delimiter = parser.token()?;
            if !matches!(delimiter.kind, Kind::Punctuation(',' | ';'))
                || delimiter.span.start != declaration.end
            {
                return Err(parser.error(declaration.name_at, "invalid constant declaration"));
            }
            if constant.annotation.is_none()
                && declaration
                    .owner
                    .as_ref()
                    .is_none_or(|owner| aliases.classes.contains(owner))
            {
                if let Some(value) = crate::constants::literal(&constant.initializer) {
                    aliases
                        .constant_values
                        .borrow_mut()
                        .insert(key.clone(), value);
                }
            }
            Ok(())
        })();
        result.map_err(|diagnostic| crate::SourceDiagnostic {
            source: declaration.source_index,
            diagnostic,
        })?;
    }
    Ok(aliases)
}

#[derive(Clone, Copy)]
enum IntegerWidth {
    Number,
    Long,
}

/// Parse decimal signed magnitudes or radix bit patterns at the selected width.
/// Nondecimal negation wraps at that width; decimal values must fit signed range.
fn integer_value(digits: &str, negative: bool, width: IntegerWidth) -> Option<i64> {
    let (digits, radix) = if let Some(hex) = digits.strip_prefix("0x") {
        (hex, 16)
    } else if digits.len() > 1 && digits.starts_with('0') {
        (digits, 8)
    } else {
        (digits, 10)
    };
    let raw = u64::from_str_radix(digits, radix).ok()?;
    if radix == 10 {
        let signed = if negative {
            -(raw as i128)
        } else {
            raw as i128
        };
        return match width {
            IntegerWidth::Number => i32::try_from(signed).ok().map(i64::from),
            IntegerWidth::Long => i64::try_from(signed).ok(),
        };
    }
    Some(match width {
        IntegerWidth::Number => {
            let value = u32::try_from(raw).ok()? as i32;
            i64::from(if negative {
                value.wrapping_neg()
            } else {
                value
            })
        }
        IntegerWidth::Long => {
            let value = raw as i64;
            if negative {
                value.wrapping_neg()
            } else {
                value
            }
        }
    })
}

impl<'a> Parser<'a> {
    /// Parse Dictionary arguments through the shared value-type grammar.
    fn dictionary_annotation(&mut self) -> Result<Option<crate::DictionaryType>, Diagnostic> {
        if self.peek()? != Kind::Punctuation('<') {
            return Ok(None);
        }
        self.token()?;
        let at = self.offset;
        let key = self.parameter_annotation(at)?;
        self.expect(Kind::Punctuation(','), "',' and a Dictionary value type")?;
        let value = self.parameter_annotation(self.offset)?;
        self.close_type_arguments()?;
        if key.source_contract().is_some() || value.source_contract().is_some() {
            return Err(self.error(at, "unsupported Dictionary source-class contract"));
        }
        crate::DictionaryType::from_api_type(&crate::api::Type::dictionary(
            key.api_type(),
            value.api_type(),
        ))
        .map(Some)
        .ok_or_else(|| self.error(at, "unsupported Dictionary key/value contract"))
    }

    fn array_annotation(&mut self) -> Result<crate::ArrayElement, Diagnostic> {
        self.expect(Kind::Punctuation('<'), "'<' and an Array element type")?;
        let at = self.offset;
        let element = self.parameter_annotation(at)?;
        self.close_type_arguments()?;
        if let Some(contract) = element.source_contract() {
            return Ok(crate::ArrayElement::Source(Box::new(contract)));
        }
        crate::ArrayElement::from_api_type(&element.api_type())
            .ok_or_else(|| self.error(at, "unsupported Array element annotation"))
    }
    /// Consume one generic closing bracket, leaving an adjacent bracket or
    /// assignment for its enclosing parser. Expression shifts keep their lexer
    /// tokens; only type-argument parsing splits `>>` and `>=` here.
    fn close_type_arguments(&mut self) -> Result<(), Diagnostic> {
        let token = self.token()?;
        if self.source.as_bytes().get(token.span.start) != Some(&b'>') {
            return Err(self.error(token.span.start, "expected '>'"));
        }
        self.offset = token.span.start + 1;
        self.lookahead = None;
        Ok(())
    }

    fn path(&mut self) -> Result<String, Diagnostic> {
        let token = self.identifier()?;
        let Kind::Word(name) = token.kind else {
            unreachable!()
        };
        let mut path = name.to_owned();
        while self.peek()? == Kind::Punctuation('.') {
            self.token()?;
            let token = self.identifier()?;
            let Kind::Word(name) = token.kind else {
                unreachable!()
            };
            path.push('.');
            path.push_str(name);
        }
        Ok(path)
    }

    /// Parse a type path and canonicalize its imported or aliased native name.
    /// Unresolved bare names remain available for unit source-class resolution.
    fn type_path(&mut self) -> Result<String, Diagnostic> {
        let at = self.offset;
        let path = self.path()?;
        Ok(self
            .names
            .type_name(&path)
            .map_err(|message| self.error(at, message))?
            .unwrap_or(path))
    }

    fn imports(&mut self) -> Result<(), Diagnostic> {
        while matches!(self.peek()?, Kind::Word("using" | "import")) {
            let token = self.token()?;
            let import = token.kind == Kind::Word("import");
            let path = self.path()?;
            let alias = if self.peek()? == Kind::Word("as") {
                if import {
                    return Err(self.error(token.span.start, "import aliases are unsupported"));
                }
                self.token()?;
                let token = self.identifier()?;
                let Kind::Word(name) = token.kind else {
                    unreachable!()
                };
                Some(name)
            } else {
                None
            };
            self.expect(Kind::Punctuation(';'), "';'")?;
            self.names
                .declare(&path, alias, import)
                .map_err(|message| self.error(token.span.start, message))?;
        }
        Ok(())
    }

    fn reject_binding(&self, token: &Token<'_>) -> Result<(), Diagnostic> {
        if let Kind::Word(name) = token.kind {
            if self.names.bound(name)
                || (self.native_base == Some(crate::ClassBase::LangObject) && name == "Object")
                || (self.native_base == Some(crate::ClassBase::ApplicationAppBase)
                    && name == "AppBase")
                || (self.native_base == Some(crate::ClassBase::WatchUiView) && name == "View")
                || (self.native_base == Some(crate::ClassBase::WatchUiBehaviorDelegate)
                    && name == "BehaviorDelegate")
            {
                return Err(self.error(
                    token.span.start,
                    format!("declaration shadows native binding {name:?}"),
                ));
            }
        }
        Ok(())
    }
    fn error(&self, offset: usize, message: impl Into<String>) -> Diagnostic {
        Diagnostic::at(self.source, offset, message)
    }

    fn read_token(&mut self) -> Result<Token<'a>, Diagnostic> {
        loop {
            while let Some(c) = self.source[self.offset..].chars().next() {
                if !c.is_whitespace() {
                    break;
                }
                self.offset += c.len_utf8();
            }
            let rest = &self.source[self.offset..];
            if rest.starts_with("//") {
                self.offset += rest.find('\n').unwrap_or(rest.len());
            } else if let Some(comment) = rest.strip_prefix("/*") {
                let end = comment
                    .find("*/")
                    .ok_or_else(|| self.error(self.offset, "unterminated block comment"))?;
                self.offset += end + 4;
            } else {
                break;
            }
        }
        let start = self.offset;
        let rest = &self.source[start..];
        let kind = match rest.chars().next() {
            None => Kind::End,
            Some(c) if c.is_ascii_alphabetic() || c == '_' => {
                let length = rest
                    .bytes()
                    .take_while(|b| b.is_ascii_alphanumeric() || *b == b'_')
                    .count();
                self.offset += length;
                Kind::Word(&rest[..length])
            }
            Some('\'') => {
                self.offset += 1;
                if self.source[self.offset..].starts_with(['\'', '\n', '\r']) {
                    return Err(self.error(
                        start,
                        "Char literals require one character without raw newlines",
                    ));
                }
                let mut characters = self.source[self.offset..].chars();
                let value = literal_character(&mut characters)
                    .map_err(|message| self.error(start, message))?;
                self.offset = self.source.len() - characters.as_str().len();
                if value > 0xffff {
                    return Err(self.error(
                        start,
                        "source Char literals currently require a BMP character",
                    ));
                }
                if !self.source[self.offset..].starts_with('\'') {
                    return Err(self.error(start, "Char literals require exactly one character"));
                }
                self.offset += 1;
                Kind::Char(value)
            }
            Some('"') => {
                self.offset += 1;
                loop {
                    let c = self.source[self.offset..]
                        .chars()
                        .next()
                        .ok_or_else(|| self.error(start, "unterminated string literal"))?;
                    self.offset += c.len_utf8();
                    if c == '"' {
                        break;
                    }
                    if matches!(c, '\n' | '\r') {
                        return Err(
                            self.error(start, "string literals cannot contain raw newlines")
                        );
                    }
                    if c == '\\' {
                        let escaped = self.source[self.offset..]
                            .chars()
                            .next()
                            .ok_or_else(|| self.error(start, "unfinished string escape"))?;
                        if matches!(escaped, '\n' | '\r') {
                            return Err(
                                self.error(start, "string literals cannot contain raw newlines")
                            );
                        }
                        self.offset += escaped.len_utf8();
                    }
                }
                Kind::String(&self.source[start..self.offset])
            }
            Some('0')
                if rest.starts_with("0x")
                    || rest.as_bytes().get(1).is_some_and(u8::is_ascii_digit) =>
            {
                let length = if let Some(hex) = rest.strip_prefix("0x") {
                    let count = hex.bytes().take_while(u8::is_ascii_hexdigit).count();
                    if count == 0 {
                        return Err(self.error(start, "hexadecimal literal requires digits"));
                    }
                    2 + count
                } else {
                    rest.bytes().take_while(u8::is_ascii_digit).count()
                };
                self.offset += length;
                if matches!(rest.as_bytes().get(length), Some(b'l' | b'L')) {
                    self.offset += 1;
                    Kind::Long(&rest[..length])
                } else {
                    Kind::Integer(&rest[..length])
                }
            }
            Some(c)
                if c.is_ascii_digit()
                    || (c == '.' && rest.as_bytes().get(1).is_some_and(u8::is_ascii_digit)) =>
            {
                let mut length = rest.bytes().take_while(u8::is_ascii_digit).count();
                let mut real = false;
                if rest.as_bytes().get(length) == Some(&b'.')
                    && rest
                        .as_bytes()
                        .get(length + 1)
                        .is_some_and(u8::is_ascii_digit)
                {
                    real = true;
                    length += 1;
                    length += rest[length..]
                        .bytes()
                        .take_while(u8::is_ascii_digit)
                        .count();
                }
                if matches!(rest.as_bytes().get(length), Some(b'e' | b'E')) {
                    real = true;
                    length += 1;
                    if matches!(rest.as_bytes().get(length), Some(b'+' | b'-')) {
                        length += 1;
                    }
                    let digits = rest[length..]
                        .bytes()
                        .take_while(u8::is_ascii_digit)
                        .count();
                    if digits == 0 {
                        return Err(self.error(start, "floating exponent requires digits"));
                    }
                    length += digits;
                }
                self.offset += length;
                if rest.as_bytes().get(length) == Some(&b'd') {
                    self.offset += 1;
                    Kind::Real(&rest[..length], true)
                } else if rest.as_bytes().get(length) == Some(&b'f') {
                    self.offset += 1;
                    Kind::Real(&rest[..length], false)
                } else if real {
                    Kind::Real(&rest[..length], false)
                } else if matches!(rest.as_bytes().get(length), Some(b'l' | b'L')) {
                    self.offset += 1;
                    Kind::Long(&rest[..length])
                } else {
                    Kind::Integer(&rest[..length])
                }
            }
            Some('<' | '>') if rest.starts_with("<<=") || rest.starts_with(">>=") => {
                self.offset += 3;
                Kind::Compound(if rest.starts_with("<<=") {
                    crate::BinaryOp::ShiftLeft
                } else {
                    crate::BinaryOp::ShiftRight
                })
            }
            Some(c @ ('+' | '-' | '*' | '/' | '%' | '&' | '|' | '^'))
                if rest.as_bytes().get(1) == Some(&b'=') =>
            {
                self.offset += 2;
                Kind::Compound(match c {
                    '+' => crate::BinaryOp::Add,
                    '-' => crate::BinaryOp::Subtract,
                    '*' => crate::BinaryOp::Multiply,
                    '/' => crate::BinaryOp::Divide,
                    '%' => crate::BinaryOp::Remainder,
                    '&' => crate::BinaryOp::BitAnd,
                    '|' => crate::BinaryOp::BitOr,
                    '^' => crate::BinaryOp::BitXor,
                    _ => unreachable!("recognized compound operator"),
                })
            }
            Some('-') if rest.starts_with("--") => {
                self.offset += 2;
                Kind::Step(false)
            }
            Some('+') if rest.starts_with("++") => {
                self.offset += 2;
                Kind::Step(true)
            }
            Some('&') if rest.starts_with("&&") => {
                self.offset += 2;
                Kind::AndAnd
            }
            Some('|') if rest.starts_with("||") => {
                self.offset += 2;
                Kind::OrOr
            }
            Some('!') if rest.starts_with("!=") => {
                self.offset += 2;
                Kind::NotEqual
            }
            Some('<') if rest.starts_with("<=") => {
                self.offset += 2;
                Kind::LessEqual
            }
            Some('>') if rest.starts_with(">=") => {
                self.offset += 2;
                Kind::GreaterEqual
            }
            Some('<') if rest.starts_with("<<") => {
                self.offset += 2;
                Kind::ShiftLeft
            }
            Some('>') if rest.starts_with(">>") => {
                self.offset += 2;
                Kind::ShiftRight
            }
            Some('=') if rest.starts_with("==") => {
                self.offset += 2;
                Kind::EqualEqual
            }
            Some(
                c @ ('(' | ')' | '{' | '}' | '[' | ']' | ';' | '+' | '-' | '=' | '.' | ',' | '>'
                | '<' | '!' | '*' | '/' | '%' | '&' | '|' | '^' | '~' | '?' | ':' | '$'),
            ) => {
                self.offset += 1;
                Kind::Punctuation(c)
            }
            Some(c) => return Err(self.error(start, format!("unsupported character {c:?}"))),
        };
        Ok(Token {
            kind,
            span: Span {
                start,
                end: self.offset,
            },
        })
    }

    fn token(&mut self) -> Result<Token<'a>, Diagnostic> {
        if let Some(token) = self.lookahead.take() {
            Ok(token)
        } else {
            self.read_token()
        }
    }
    fn peek(&mut self) -> Result<Kind<'a>, Diagnostic> {
        if self.lookahead.is_none() {
            self.lookahead = Some(self.read_token()?);
        }
        Ok(self.lookahead.as_ref().unwrap().kind)
    }
    fn node(&mut self, kind: ExpressionKind, span: Span) -> Result<Expression, Diagnostic> {
        fn receiver(kind: &ExpressionKind) -> Option<&Expression> {
            match kind {
                ExpressionKind::MemberRead { receiver, .. }
                | ExpressionKind::ValueCall { receiver, .. } => Some(receiver.as_ref()),
                ExpressionKind::ArrayRead { array, .. } => Some(array.as_ref()),
                _ => None,
            }
        }
        let mut current = &kind;
        let mut lookups = 0;
        while let Some(next) = receiver(current) {
            lookups += 1;
            if lookups > 64 {
                return Err(self.error(span.start, "expression nesting exceeds 64 levels"));
            }
            current = &next.kind;
        }
        self.expressions += 1;
        if self.expressions > crate::MAX_OPERATIONS {
            return Err(self.error(span.start, "expression exceeds 1024-operation limit"));
        }
        Ok(Expression { kind, span })
    }
    fn identifier(&mut self) -> Result<Token<'a>, Diagnostic> {
        let token = self.token()?;
        match token.kind {
            Kind::Word(name) if !reserved_word(name) => Ok(token),
            _ => Err(self.error(token.span.start, "expected a non-reserved identifier")),
        }
    }
    fn expression(&mut self, depth: usize) -> Result<Expression, Diagnostic> {
        let condition = self.logical(depth, false)?;
        if self.peek()? == Kind::Punctuation('?') {
            self.conditional(condition, depth)
        } else {
            Ok(condition)
        }
    }

    /// Parse right-associative conditional arms below logical OR precedence.
    fn conditional(
        &mut self,
        condition: Expression,
        depth: usize,
    ) -> Result<Expression, Diagnostic> {
        if depth >= 64 {
            return Err(self.error(condition.span.start, "expression nesting exceeds 64"));
        }
        self.token()?;
        let when_true = self.expression(depth + 1)?;
        self.expect(Kind::Punctuation(':'), "':'")?;
        let when_false = self.expression(depth + 1)?;
        let span = Span {
            start: condition.span.start,
            end: when_false.span.end,
        };
        self.node(
            ExpressionKind::Conditional {
                condition: Box::new(condition),
                when_true: Box::new(when_true),
                when_false: Box::new(when_false),
            },
            span,
        )
    }
    fn logical(&mut self, depth: usize, and: bool) -> Result<Expression, Diagnostic> {
        let operand = |parser: &mut Self| {
            if and {
                parser.comparison(depth)
            } else {
                parser.logical(depth, true)
            }
        };
        let mut left = operand(self)?;
        while matches!(
            (and, self.peek()?),
            (true, Kind::AndAnd | Kind::Word("and")) | (false, Kind::OrOr | Kind::Word("or"))
        ) {
            self.token()?;
            let right = operand(self)?;
            let span = Span {
                start: left.span.start,
                end: right.span.end,
            };
            left = self.node(
                ExpressionKind::Logical {
                    and,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            )?;
        }
        Ok(left)
    }
    fn comparison(&mut self, depth: usize) -> Result<Expression, Diagnostic> {
        let left = self.arithmetic(depth)?;
        if self.peek()? == Kind::Word("instanceof") {
            self.token()?;
            let class = self.runtime_class()?;
            let span = Span {
                start: left.span.start,
                end: self.offset,
            };
            return self.node(
                ExpressionKind::InstanceOf {
                    operand: Box::new(left),
                    class,
                },
                span,
            );
        }
        if self.peek()? == Kind::Word("has") {
            self.token()?;
            let right = self.arithmetic(depth)?;
            let span = Span {
                start: left.span.start,
                end: right.span.end,
            };
            return self.node(
                ExpressionKind::Has {
                    receiver: Box::new(left),
                    symbol: Box::new(right),
                },
                span,
            );
        }
        let kind = match self.peek()? {
            Kind::EqualEqual => crate::ComparisonOp::Equal,
            Kind::NotEqual => crate::ComparisonOp::NotEqual,
            Kind::LessEqual => crate::ComparisonOp::LessEqual,
            Kind::GreaterEqual => crate::ComparisonOp::GreaterEqual,
            Kind::Punctuation('<') => crate::ComparisonOp::Less,
            Kind::Punctuation('>') => crate::ComparisonOp::Greater,
            _ => return Ok(left),
        };
        self.token()?;
        let right = self.arithmetic(depth)?;
        let span = Span {
            start: left.span.start,
            end: right.span.end,
        };
        self.node(
            ExpressionKind::Compare {
                kind,
                left: Box::new(left),
                right: Box::new(right),
            },
            span,
        )
    }
    fn arithmetic(&mut self, depth: usize) -> Result<Expression, Diagnostic> {
        // Keep precedence handling iterative: another parser frame on every
        // operand would reduce the supported nested-call/parenthesis budget.
        let mut values = vec![self.unary(depth)?];
        let mut operators = Vec::<(crate::BinaryOp, u8)>::new();
        loop {
            if self.peek()? == Kind::Word("as") {
                let operand = values.pop().unwrap();
                values.push(self.cast_tail(operand, depth)?);
            }
            let (kind, precedence) = match self.peek()? {
                Kind::Punctuation('+') => (crate::BinaryOp::Add, 1),
                Kind::Punctuation('-') => (crate::BinaryOp::Subtract, 1),
                Kind::Punctuation('*') => (crate::BinaryOp::Multiply, 2),
                Kind::Punctuation('/') => (crate::BinaryOp::Divide, 2),
                Kind::Punctuation('%') => (crate::BinaryOp::Remainder, 2),
                Kind::Punctuation('&') => (crate::BinaryOp::BitAnd, 2),
                Kind::Punctuation('|') => (crate::BinaryOp::BitOr, 1),
                Kind::Punctuation('^') => (crate::BinaryOp::BitXor, 1),
                Kind::ShiftLeft => (crate::BinaryOp::ShiftLeft, 2),
                Kind::ShiftRight => (crate::BinaryOp::ShiftRight, 2),
                _ => break,
            };
            while operators
                .last()
                .is_some_and(|(_, previous)| *previous >= precedence)
            {
                let (previous, _) = operators.pop().unwrap();
                self.reduce_arithmetic(&mut values, previous)?;
            }
            self.token()?;
            operators.push((kind, precedence));
            values.push(self.unary(depth)?);
        }
        while let Some((kind, _)) = operators.pop() {
            self.reduce_arithmetic(&mut values, kind)?;
        }
        Ok(values.pop().unwrap())
    }

    /// Parse lexical assertions below unary/postfix and above binary operators.
    /// Parse lexical value assertions at cast precedence using annotation syntax.
    fn cast_tail(
        &mut self,
        mut expression: Expression,
        depth: usize,
    ) -> Result<Expression, Diagnostic> {
        let mut casts = depth;
        while self.peek()? == Kind::Word("as") {
            if casts >= 64 {
                return Err(self.error(
                    expression.span.start,
                    "expression nesting exceeds 64 levels",
                ));
            }
            casts += 1;
            let at = self.token()?.span.start;
            let parameter = self.type_annotation(at, true)?;
            let target = if let Some(contract) = parameter.source_contract() {
                if contract.classes().len() == 1 && contract.native().is_empty() {
                    let ty = crate::api::Type::named(&contract.classes()[0].name);
                    if contract.nullable() {
                        ty.with_null()
                    } else {
                        ty
                    }
                } else {
                    crate::api::Type::SourceValue(Box::new(contract))
                }
            } else {
                parameter.api_type()
            };
            if crate::casts::type_tag(&target).is_none()
                && crate::source_objects::target_contract(&target).is_none()
            {
                return Err(self.error(at, "unsupported lexical cast target"));
            }
            let span = Span {
                start: expression.span.start,
                end: self.offset,
            };
            expression = self.node(
                ExpressionKind::Cast {
                    operand: Box::new(expression),
                    target,
                },
                span,
            )?;
        }
        Ok(expression)
    }

    /// Combine the last two parsed operands into a spanned binary expression.
    /// The operator stack only requests reduction after both operands exist.
    fn reduce_arithmetic(
        &mut self,
        values: &mut Vec<Expression>,
        kind: crate::BinaryOp,
    ) -> Result<(), Diagnostic> {
        let right = values.pop().unwrap();
        let left = values.pop().unwrap();
        let span = Span {
            start: left.span.start,
            end: right.span.end,
        };
        values.push(self.node(
            ExpressionKind::Binary {
                kind,
                left: Box::new(left),
                right: Box::new(right),
            },
            span,
        )?);
        Ok(())
    }
    /// Parse Number complement as a spanned unary expression.
    /// Isolate construction so ordinary unary parsing keeps its stack budget.
    fn complement(&mut self, depth: usize) -> Result<Expression, Diagnostic> {
        if depth >= 64 {
            return Err(self.error(self.offset, "expression nesting exceeds 64"));
        }
        let sign = self.token()?;
        let operand = self.unary(depth + 1)?;
        let span = Span {
            start: sign.span.start,
            end: operand.span.end,
        };
        self.node(ExpressionKind::Complement(Box::new(operand)), span)
    }
    /// Parse unary plus with its operand span and the ordinary nesting limit.
    /// Keep construction outside unary parsing to preserve its stack budget.
    fn positive(&mut self, depth: usize) -> Result<Expression, Diagnostic> {
        let sign = self.token()?;
        if depth >= 64 {
            return Err(self.error(sign.span.start, "expression nesting exceeds 64 levels"));
        }
        let operand = self.primary(depth + 1)?;
        let span = Span {
            start: sign.span.start,
            end: operand.span.end,
        };
        self.node(ExpressionKind::Positive(Box::new(operand)), span)
    }
    fn unary(&mut self, depth: usize) -> Result<Expression, Diagnostic> {
        if matches!(self.peek()?, Kind::Punctuation('+' | '-' | '~' | '!')) {
            self.prefix_unary(depth)
        } else {
            self.primary(depth)
        }
    }

    // Keep prefix parsing off the stack of ordinary nested calls.
    fn prefix_unary(&mut self, depth: usize) -> Result<Expression, Diagnostic> {
        if self.peek()? == Kind::Punctuation('+') {
            return self.positive(depth);
        }
        if self.peek()? == Kind::Punctuation('~') {
            return self.complement(depth);
        }
        if self.peek()? == Kind::Punctuation('!') {
            let sign = self.token()?;
            if depth >= 64 {
                return Err(self.error(sign.span.start, "expression nesting exceeds 64 levels"));
            }
            let operand = self.unary(depth + 1)?;
            let span = Span {
                start: sign.span.start,
                end: operand.span.end,
            };
            return self.node(ExpressionKind::Not(Box::new(operand)), span);
        }
        if self.peek()? != Kind::Punctuation('-') {
            return self.primary(depth);
        }
        let sign = self.token()?;
        if depth >= 64 {
            return Err(self.error(sign.span.start, "expression nesting exceeds 64 levels"));
        }
        if let Kind::Real(digits, double) = self.peek()? {
            let magnitude = self.token()?;
            return self.real_literal(
                digits,
                double,
                Span {
                    start: sign.span.start,
                    end: magnitude.span.end,
                },
                true,
            );
        }
        if let Kind::Long(digits) = self.peek()? {
            let magnitude = self.token()?;
            return self.long_literal(
                digits,
                Span {
                    start: sign.span.start,
                    end: magnitude.span.end,
                },
                true,
            );
        }
        if self.peek()? == Kind::Word("NaN") {
            let magnitude = self.token()?;
            if self.peek()? == Kind::Punctuation('.') {
                return Err(self.error(
                    sign.span.start,
                    "parenthesize a negative literal before a member call",
                ));
            }
            // The SDK keeps the canonical positive NaN payload for -NaN too.
            return self.node(
                ExpressionKind::Float(NAN_LITERAL_BITS),
                Span {
                    start: sign.span.start,
                    end: magnitude.span.end,
                },
            );
        }
        if let Kind::Integer(digits) = self.peek()? {
            let magnitude = self.token()?;
            let value = integer_value(digits, true, IntegerWidth::Number).ok_or_else(|| {
                self.error(
                    sign.span.start,
                    "invalid Number literal or value exceeds 32-bit range",
                )
            })?;
            if self.peek()? == Kind::Punctuation('.') {
                return Err(self.error(
                    sign.span.start,
                    "parenthesize a negative literal before a member call",
                ));
            }
            return self.node(
                ExpressionKind::Integer(value as i32),
                Span {
                    start: sign.span.start,
                    end: magnitude.span.end,
                },
            );
        }
        if self.peek()? == Kind::Punctuation('-') {
            return Err(self.error(sign.span.start, "parenthesize repeated negation"));
        }
        let operand = self.primary(depth + 1)?;
        let span = Span {
            start: sign.span.start,
            end: operand.span.end,
        };
        self.node(ExpressionKind::Negate(Box::new(operand)), span)
    }
    /// Parse Dictionary entries; the destination contract validates nested values.
    fn dictionary_literal(&mut self, start: usize, depth: usize) -> Result<Expression, Diagnostic> {
        if self.peek()? == Kind::Punctuation('}') {
            let close = self.token()?;
            return self.node(
                ExpressionKind::DictionaryNew,
                Span {
                    start,
                    end: close.span.end,
                },
            );
        }
        let mut entries = Vec::new();
        loop {
            if depth >= 64 {
                return Err(self.error(start, "expression nesting exceeds 64 levels"));
            }
            let key = self.expression(depth + 1)?;
            self.expect(
                Kind::Punctuation('='),
                "'=>' between Dictionary key and value",
            )?;
            self.expect(
                Kind::Punctuation('>'),
                "'=>' between Dictionary key and value",
            )?;
            let value = self.expression(depth + 1)?;
            entries.push((key, value));
            if self.peek()? != Kind::Punctuation(',') {
                break;
            }
            self.token()?;
            if self.peek()? == Kind::Punctuation('}') {
                break;
            }
        }
        let close = self.expect(Kind::Punctuation('}'), "'}'")?;
        self.node(
            ExpressionKind::DictionaryLiteral(entries),
            Span {
                start,
                end: close.span.end,
            },
        )
    }

    /// Parse construction outside ordinary primary-expression frames so nested
    /// calls retain the supported expression-depth budget on the default stack.
    fn construct(&mut self, depth: usize) -> Result<Expression, Diagnostic> {
        let open = self.token()?;
        if self.peek()? == Kind::Punctuation('[') {
            self.token()?;
            if depth >= 64 {
                return Err(self.error(open.span.start, "expression nesting exceeds 64 levels"));
            }
            let length = self.expression(depth + 1)?;
            let close = self.expect(Kind::Punctuation(']'), "']'")?;
            let byte_array = self.peek()? == Kind::Word("b");
            let end = if byte_array {
                self.token()?.span.end
            } else {
                close.span.end
            };
            return self.node(
                if byte_array {
                    ExpressionKind::ByteArrayAllocate(Box::new(length))
                } else {
                    ExpressionKind::ArrayAllocate(Box::new(length))
                },
                Span {
                    start: open.span.start,
                    end,
                },
            );
        }
        let path = self.path()?;
        let weak = self.names.type_name(&path).ok().flatten().as_deref()
            == Some("Toybox.Lang.WeakReference");
        let native = self
            .names
            .view(&path)
            .then_some(crate::NativeClass::View)
            .or_else(|| {
                self.names
                    .behavior_delegate(&path)
                    .then_some(crate::NativeClass::BehaviorDelegate)
            });
        if native.is_none() && !weak && path.contains('.') {
            return Err(self.error(open.span.start, "unsupported qualified construction class"));
        }
        let (arguments, end) = self.call_arguments(open.span.start, depth)?;
        if (native.is_some() || weak) && !arguments.is_empty() {
            return Err(self.error(
                open.span.start,
                "supported native construction requires zero arguments",
            ));
        }
        self.node(
            if weak || native.is_some() {
                ExpressionKind::NativeNew {
                    class: if weak {
                        NativeConstructor::WeakReference
                    } else {
                        NativeConstructor::Object(native.unwrap())
                    },
                    qualifier: path.split('.').next().unwrap().into(),
                }
            } else {
                ExpressionKind::SourceNew {
                    class: path,
                    arguments,
                }
            },
            Span {
                start: open.span.start,
                end,
            },
        )
    }

    fn primary_base(&mut self, depth: usize) -> Result<Expression, Diagnostic> {
        let expression = if self.peek()? == Kind::Punctuation('[') {
            let open = self.token()?;
            if depth >= 64 {
                return Err(self.error(open.span.start, "expression nesting exceeds 64 levels"));
            }
            let mut elements = if self.peek()? == Kind::Punctuation(']') {
                Vec::new()
            } else {
                vec![self.expression(depth + 1)?]
            };
            while self.peek()? == Kind::Punctuation(',') {
                self.token()?;
                if self.peek()? == Kind::Punctuation(']') {
                    break;
                }
                elements.push(self.expression(depth + 1)?);
            }
            let close = self.expect(Kind::Punctuation(']'), "']'")?;
            let bytes = self.peek()? == Kind::Word("b");
            let end = if bytes {
                self.token()?.span.end
            } else {
                close.span.end
            };
            self.node(
                if bytes {
                    ExpressionKind::ByteArrayLiteral(elements)
                } else {
                    ExpressionKind::ArrayLiteral(elements)
                },
                Span {
                    start: open.span.start,
                    end,
                },
            )?
        } else if self.peek()? == Kind::Word("new") {
            self.construct(depth)?
        } else if self.peek()? == Kind::Punctuation('(') {
            let open = self.token()?;
            if depth >= 64 {
                return Err(self.error(open.span.start, "expression nesting exceeds 64 levels"));
            }
            let mut expression = self.expression(depth + 1)?;
            let close = self.expect(Kind::Punctuation(')'), "')'")?;
            expression.span = Span {
                start: open.span.start,
                end: close.span.end,
            };
            expression
        } else {
            self.atom(depth)?
        };
        Ok(expression)
    }

    fn primary(&mut self, depth: usize) -> Result<Expression, Diagnostic> {
        let expression = self.primary_base(depth)?;
        self.postfix(expression, depth)
    }

    /// Parse suffixes after the base expression has released its parsing frame.
    fn postfix(
        &mut self,
        mut expression: Expression,
        depth: usize,
    ) -> Result<Expression, Diagnostic> {
        let mut call_depth = depth;
        while matches!(
            self.peek()?,
            Kind::Punctuation('.') | Kind::Punctuation('[')
        ) {
            if self.peek()? == Kind::Punctuation('[') {
                let open = self.token()?;
                if call_depth >= 64 {
                    return Err(self.error(open.span.start, "expression nesting exceeds 64 levels"));
                }
                let index = self.expression(call_depth + 1)?;
                let end = self.expect(Kind::Punctuation(']'), "']'")?.span.end;
                let start = expression.span.start;
                call_depth += 1;
                expression = self.node(
                    ExpressionKind::ArrayRead {
                        array: Box::new(expression),
                        index: Box::new(index),
                    },
                    Span { start, end },
                )?;
                continue;
            }
            self.token()?;
            let token = self.identifier()?;
            let Kind::Word(member) = token.kind else {
                unreachable!()
            };
            let start = expression.span.start;
            if self.peek()? != Kind::Punctuation('(') {
                expression = self.node(
                    ExpressionKind::MemberRead {
                        receiver: Box::new(expression),
                        member: member.into(),
                    },
                    Span {
                        start,
                        end: token.span.end,
                    },
                )?;
                call_depth += 1;
                continue;
            }
            let (arguments, end) = self.call_arguments(start, call_depth)?;
            call_depth += 1;
            expression = self.node(
                ExpressionKind::ValueCall {
                    receiver: Box::new(expression),
                    member: member.into(),
                    arguments,
                },
                Span { start, end },
            )?;
        }
        Ok(expression)
    }

    /// Parse bounded positional arguments and return the closing source offset.
    fn call_arguments(
        &mut self,
        start: usize,
        depth: usize,
    ) -> Result<(Vec<Expression>, usize), Diagnostic> {
        self.expect(
            Kind::Punctuation('('),
            "'(' (member calls require parentheses)",
        )?;
        if depth >= 64 {
            return Err(self.error(start, "expression nesting exceeds 64 levels"));
        }
        let mut arguments = Vec::new();
        if self.peek()? != Kind::Punctuation(')') {
            loop {
                if arguments.len() >= crate::MAX_PARAMETERS {
                    return Err(self.error(start, "call exceeds 254-argument limit"));
                }
                arguments.push(self.expression(depth + 1)?);
                if self.peek()? != Kind::Punctuation(',') {
                    break;
                }
                self.token()?;
            }
        }
        let end = self.expect(Kind::Punctuation(')'), "')'")?.span.end;
        Ok((arguments, end))
    }

    /// Parse a decimal or radix Long token, retaining its signed 64-bit pattern.
    /// Negative literals require a grouped receiver, as for Number and Float.
    fn long_literal(
        &mut self,
        digits: &str,
        span: Span,
        negative: bool,
    ) -> Result<Expression, Diagnostic> {
        let value = integer_value(digits, negative, IntegerWidth::Long).ok_or_else(|| {
            self.error(
                span.start,
                "invalid Long literal or value exceeds 64-bit range",
            )
        })?;
        if negative && self.peek()? == Kind::Punctuation('.') {
            return Err(self.error(
                span.start,
                "parenthesize a negative literal before a member call",
            ));
        }
        self.node(ExpressionKind::Long(value), span)
    }

    /// Parse a finite decimal Float/Double literal, retaining its rounded bits and sign.
    fn real_literal(
        &mut self,
        digits: &str,
        double: bool,
        span: Span,
        negative: bool,
    ) -> Result<Expression, Diagnostic> {
        if negative && self.peek()? == Kind::Punctuation('.') {
            return Err(self.error(
                span.start,
                "parenthesize a negative literal before a member call",
            ));
        }
        let text = if negative {
            format!("-{digits}")
        } else {
            digits.to_owned()
        };
        let kind = if double {
            let value = text
                .parse::<f64>()
                .map_err(|_| self.error(span.start, "invalid Double literal"))?;
            if !value.is_finite() {
                return Err(self.error(span.start, "Double literal exceeds finite range"));
            }
            ExpressionKind::Double(value.to_bits())
        } else {
            let value = text
                .parse::<f32>()
                .map_err(|_| self.error(span.start, "invalid Float literal"))?;
            if !value.is_finite() {
                return Err(self.error(span.start, "Float literal exceeds finite range"));
            }
            ExpressionKind::Float(value.to_bits())
        };
        self.node(kind, span)
    }

    /// Parse a root-qualified function call outside recursive atom frames.
    fn global_atom(&mut self, start: usize, depth: usize) -> Result<Expression, Diagnostic> {
        self.expect(Kind::Punctuation('.'), "'.' after global root")?;
        let first = self.identifier()?;
        let Kind::Word(first) = first.kind else {
            unreachable!()
        };
        let (module, member) = if self.peek()? == Kind::Punctuation('.') {
            self.token()?;
            let member = self.identifier()?;
            let Kind::Word(member) = member.kind else {
                unreachable!()
            };
            (Some(first), member)
        } else {
            (None, first)
        };
        let (arguments, end) = self.call_arguments(start, depth)?;
        self.node(
            if let Some(module) = module {
                ExpressionKind::OwnerCall {
                    module: module.into(),
                    member: member.into(),
                    arguments,
                }
            } else {
                ExpressionKind::GlobalCall {
                    member: member.into(),
                    arguments,
                }
            },
            Span { start, end },
        )
    }

    fn atom(&mut self, depth: usize) -> Result<Expression, Diagnostic> {
        let token = self.token()?;
        match token.kind {
            Kind::Punctuation('$') => self.global_atom(token.span.start, depth),
            Kind::Punctuation(':') => {
                let word = self.identifier()?;
                let Kind::Word(name) = word.kind else {
                    unreachable!()
                };
                self.node(
                    ExpressionKind::Symbol(name.into()),
                    Span {
                        start: token.span.start,
                        end: word.span.end,
                    },
                )
            }
            Kind::Char(value) => self.node(ExpressionKind::Char(value), token.span),
            Kind::String(literal) => {
                let value = string_literal(literal)
                    .map_err(|message| self.error(token.span.start, message))?;
                self.node(ExpressionKind::String(value), token.span)
            }
            Kind::Long(digits) => self.long_literal(digits, token.span, false),
            Kind::Real(digits, double) => self.real_literal(digits, double, token.span, false),
            Kind::Integer(digits) => {
                let value =
                    integer_value(digits, false, IntegerWidth::Number).ok_or_else(|| {
                        self.error(
                            token.span.start,
                            "invalid Number literal or value exceeds 32-bit range",
                        )
                    })?;
                self.node(ExpressionKind::Integer(value as i32), token.span)
            }
            Kind::Word("NaN") => self.node(ExpressionKind::Float(NAN_LITERAL_BITS), token.span),
            Kind::Word("null") => self.node(ExpressionKind::Null, token.span),
            Kind::Punctuation('{') => self.dictionary_literal(token.span.start, depth),
            Kind::Word("true") => self.node(ExpressionKind::Boolean(true), token.span),
            Kind::Word("false") => self.node(ExpressionKind::Boolean(false), token.span),
            Kind::Word(name) if name == "self" || !reserved_word(name) => {
                self.named_atom(name, token.span, depth)
            }
            _ => Err(self.error(
                token.span.start,
                "expected an integer, local identifier, or parenthesized expression",
            )),
        }
    }

    /// Parse an identifier value or call. Resolve qualified targets before
    /// descending into arguments so target parsing does not grow recursive frames.
    fn named_atom(
        &mut self,
        name: &str,
        span: Span,
        depth: usize,
    ) -> Result<Expression, Diagnostic> {
        let mut expression = if self.peek()? == Kind::Punctuation('.') {
            self.qualified_atom(name, span.start)?
        } else if self.peek()? == Kind::Punctuation('(') {
            self.node(
                ExpressionKind::UnqualifiedCall {
                    member: name.into(),
                    arguments: Vec::new(),
                },
                span,
            )?
        } else if name == "self" {
            return self.node(ExpressionKind::SelfValue, span);
        } else {
            return self.node(ExpressionKind::Local(name.into()), span);
        };
        if let ExpressionKind::ModuleCall { arguments, .. }
        | ExpressionKind::UnqualifiedCall { arguments, .. }
        | ExpressionKind::ValueCall { arguments, .. } = &mut expression.kind
        {
            let (parsed, end) = self.call_arguments(span.start, depth)?;
            *arguments = parsed;
            expression.span.end = end;
        }
        Ok(expression)
    }

    /// Parse a qualified member access/call, without parsing recursive arguments.
    fn qualified_atom(&mut self, name: &str, start: usize) -> Result<Expression, Diagnostic> {
        self.token()?;
        let member = self.identifier()?;
        let member_end = member.span.end;
        let Kind::Word(member) = member.kind else {
            unreachable!()
        };
        if name == "self" && self.peek()? != Kind::Punctuation('(') {
            return self.node(
                ExpressionKind::SelfField(member.into()),
                Span {
                    start,
                    end: member_end,
                },
            );
        }
        let mut path = format!("{name}.{member}");
        while self.peek()? == Kind::Punctuation('.') {
            self.token()?;
            let token = self.identifier()?;
            let Kind::Word(part) = token.kind else {
                unreachable!()
            };
            path.push('.');
            path.push_str(part);
        }
        let (module, member) = path.rsplit_once('.').unwrap();
        if self.names.module(module).is_some_and(|native| {
            crate::api::core()
                .constant(&format!("Toybox.{}", native.name()), member)
                .is_some()
        }) && self.peek()? != Kind::Punctuation('(')
        {
            return self.node(
                ExpressionKind::NativeConstant {
                    module: format!("Toybox.{}", self.names.module(module).unwrap().name()),
                    member: member.into(),
                    qualifier: path.split('.').next().unwrap().into(),
                },
                Span {
                    start,
                    end: self.offset,
                },
            );
        }
        if name != "Toybox"
            && !self.names.bound(name)
            && self.names.module(module).is_none()
            && !self.names.app_base(module)
            && !self.names.view(module)
            && !self.names.behavior_delegate(module)
            && (self.peek()? != Kind::Punctuation('(') || module.contains('.'))
        {
            let mut parts = path.split('.');
            let mut receiver = self.node(
                ExpressionKind::Local(parts.next().unwrap().into()),
                Span {
                    start,
                    end: start + name.len(),
                },
            )?;
            let mut remaining = parts.peekable();
            while let Some(member) = remaining.next() {
                let kind = if remaining.peek().is_none() && self.peek()? == Kind::Punctuation('(') {
                    ExpressionKind::ValueCall {
                        receiver: Box::new(receiver),
                        member: member.into(),
                        arguments: Vec::new(),
                    }
                } else {
                    ExpressionKind::MemberRead {
                        receiver: Box::new(receiver),
                        member: member.into(),
                    }
                };
                receiver = self.node(
                    kind,
                    Span {
                        start,
                        end: self.offset,
                    },
                )?;
            }
            return Ok(receiver);
        }
        let module = if let Some(native) = self.names.module(module) {
            native.name()
        } else if self.native_base.is_some() && (module == "AppBase" || self.names.app_base(module))
        {
            "Application.AppBase"
        } else if self.native_base.is_some() && (module == "View" || self.names.view(module)) {
            "WatchUi.View"
        } else if self.native_base.is_some()
            && (module == "BehaviorDelegate" || self.names.behavior_delegate(module))
        {
            "WatchUi.BehaviorDelegate"
        } else if module.contains('.') {
            return Err(self.error(start, "unsupported qualified module path"));
        } else {
            module
        };
        self.node(
            ExpressionKind::ModuleCall {
                module: module.into(),
                member: member.into(),
                arguments: Vec::new(),
            },
            Span {
                start,
                end: self.offset,
            },
        )
    }

    /// Check a parsed assignment target without entering RHS parsing.
    /// Local targets must be bare; explicit self fields and array elements are accepted.
    fn assignment_target(&self, expression: &Expression, start: usize) -> Result<(), Diagnostic> {
        if let ExpressionKind::Local(name) = &expression.kind {
            if self.source[expression.span.start..expression.span.end] != *name {
                return Err(self.error(start, "assignment target must be a bare local identifier"));
            }
        }
        if !matches!(
            expression.kind,
            ExpressionKind::Local(_)
                | ExpressionKind::SelfField(_)
                | ExpressionKind::MemberRead { .. }
                | ExpressionKind::ArrayRead { .. }
        ) {
            return Err(self.error(
                start,
                "assignment target must be a local, self field, or array element",
            ));
        }
        Ok(())
    }

    /// Construct the checked assignment after its RHS and terminator are parsed.
    /// Keep statement construction outside the expression parser's stack budget.
    fn assignment_statement(
        &mut self,
        expression: Expression,
        assignment: Kind<'_>,
        value: Expression,
        span: Span,
    ) -> Result<Statement, Diagnostic> {
        let statement = if let Kind::Compound(kind) = assignment {
            self.compound_statement(expression, kind, value, span)?
        } else {
            match expression.kind {
                ExpressionKind::ArrayRead { array, index } => Statement::ArrayAssign {
                    array: *array,
                    index: *index,
                    value,
                    span,
                },
                ExpressionKind::SelfField(name) => Statement::FieldAssign { name, value, span },
                ExpressionKind::MemberRead { receiver, member } => Statement::MemberAssign {
                    receiver: *receiver,
                    member,
                    value,
                    span,
                },
                ExpressionKind::Local(name) => Statement::Assign { name, value, span },
                _ => unreachable!("checked assignment target"),
            }
        };
        Ok(statement)
    }

    /// Construct a compound local/field read and write, or defer an indexed target.
    /// Source lowering captures array locations and checks operand/result contracts.
    fn compound_statement(
        &mut self,
        target: Expression,
        kind: crate::BinaryOp,
        value: Expression,
        span: Span,
    ) -> Result<Statement, Diagnostic> {
        let (name, field) = match &target.kind {
            ExpressionKind::ArrayRead { .. } | ExpressionKind::MemberRead { .. } => {
                return Ok(Statement::PlaceCompound {
                    target,
                    kind,
                    value,
                    span,
                });
            }
            ExpressionKind::Local(name) => (name.clone(), false),
            ExpressionKind::SelfField(name) => (name.clone(), true),
            _ => unreachable!("checked compound target"),
        };
        let value_span = Span {
            start: target.span.start,
            end: value.span.end,
        };
        let value = self.node(
            ExpressionKind::Binary {
                kind,
                left: Box::new(target),
                right: Box::new(value),
            },
            value_span,
        )?;
        Ok(if field {
            Statement::FieldAssign { name, value, span }
        } else {
            Statement::Assign { name, value, span }
        })
    }

    /// Parse a prefix statement update outside the recursive block parser frame.
    fn prefix_update(
        &mut self,
        start: usize,
        increment: bool,
        terminator: Option<char>,
    ) -> Result<Statement, Diagnostic> {
        self.token()?;
        let target = self.expression(0)?;
        self.finish_update(target, increment, start, terminator)
    }

    /// Check an update target and require its statement terminator.
    fn finish_update(
        &mut self,
        target: Expression,
        increment: bool,
        start: usize,
        terminator: Option<char>,
    ) -> Result<Statement, Diagnostic> {
        self.assignment_target(&target, start)?;
        let end = self.statement_end(terminator)?;
        Ok(Statement::Update {
            target,
            increment,
            span: Span { start, end },
        })
    }

    /// Consume a required statement delimiter, or leave a header delimiter for
    /// the list parser. Return the statement's source end position.
    fn statement_end(&mut self, terminator: Option<char>) -> Result<usize, Diagnostic> {
        if let Some(terminator) = terminator {
            let mut end = self
                .expect(Kind::Punctuation(terminator), "statement terminator")?
                .span
                .end;
            if terminator == ';' {
                while self.peek()? == Kind::Punctuation(';') {
                    end = self.token()?.span.end;
                }
            }
            return Ok(end);
        }
        if !matches!(self.peek()?, Kind::Punctuation(',' | ';' | ')')) {
            return Err(self.error(self.offset, "expected loop-clause delimiter"));
        }
        Ok(self.lookahead.as_ref().unwrap().span.start)
    }

    /// Parse one inferred local with an optional initializer.
    fn local_declaration(
        &mut self,
        start: usize,
        terminator: Option<char>,
    ) -> Result<Statement, Diagnostic> {
        let token = self.identifier()?;
        self.reject_binding(&token)?;
        let Kind::Word(name) = token.kind else {
            unreachable!()
        };
        if self.peek()? == Kind::Word("as") {
            return Err(self.error(
                start,
                "local variable types are inferred; explicit local annotations are unsupported",
            ));
        }
        let initializer = if self.peek()? == Kind::Punctuation('=') {
            self.token()?;
            Some(self.expression(0)?)
        } else {
            None
        };
        let end = self.statement_end(terminator)?;
        Ok(Statement::Local(LocalDeclaration {
            name: name.into(),
            initializer,
            span: Span { start, end },
        }))
    }

    /// Parse declarations or assignment/update effects in a for header, leaving
    /// nested expression commas to their expression parsers. Consume its ending
    /// semicolon or parenthesis; declaration lists share one var keyword.
    fn for_header(
        &mut self,
        terminator: char,
        allow_declarations: bool,
    ) -> Result<Vec<Statement>, Diagnostic> {
        let mut statements = Vec::new();
        if self.peek()? == Kind::Punctuation(terminator) {
            self.token()?;
            return Ok(statements);
        }
        if allow_declarations && self.peek()? == Kind::Word("var") {
            return self.local_declarations(terminator, false);
        }
        let mut start = self.lookahead.as_ref().unwrap().span.start;
        loop {
            let statement = self.simple_statement(None)?;
            if !matches!(
                statement,
                Statement::Assign { .. }
                    | Statement::FieldAssign { .. }
                    | Statement::ArrayAssign { .. }
                    | Statement::PlaceCompound { .. }
                    | Statement::Update { .. }
            ) {
                return Err(self.error(start, "for headers require declarations, assignments, or updates; standalone calls are unsupported"));
            }
            statements.push(statement);
            if self.peek()? != Kind::Punctuation(',') {
                self.expect(Kind::Punctuation(terminator), "loop-clause terminator")?;
                return Ok(statements);
            }
            self.token()?;
            self.peek()?;
            start = self.lookahead.as_ref().unwrap().span.start;
        }
    }

    /// Parse inferred locals sharing one var keyword. A normal declaration
    /// consumes repeated semicolons; a for initializer consumes one separator.
    fn local_declarations(
        &mut self,
        terminator: char,
        repeated: bool,
    ) -> Result<Vec<Statement>, Diagnostic> {
        let mut start = self.expect(Kind::Word("var"), "'var'")?.span.start;
        let mut declarations = Vec::new();
        loop {
            let mut declaration = self.local_declaration(start, None)?;
            if self.peek()? != Kind::Punctuation(',') {
                let end = if repeated {
                    self.statement_end(Some(terminator))?
                } else {
                    self.expect(Kind::Punctuation(terminator), "declaration-list terminator")?
                        .span
                        .end
                };
                let Statement::Local(local) = &mut declaration else {
                    unreachable!()
                };
                local.span.end = end;
                declarations.push(declaration);
                return Ok(declarations);
            }
            declarations.push(declaration);
            self.token()?;
            self.peek()?;
            start = self.lookahead.as_ref().unwrap().span.start;
        }
    }

    /// Parse an assignment, update, or call. A missing
    /// terminator leaves the delimiter for the for-header list parser.
    fn simple_statement(&mut self, terminator: Option<char>) -> Result<Statement, Diagnostic> {
        let start = {
            self.peek()?;
            self.lookahead.as_ref().unwrap().span.start
        };
        if self.peek()? == Kind::Word("var") {
            return Err(self.error(start, "declarations are not allowed in this clause"));
        }
        if let Kind::Step(increment) = self.peek()? {
            return self.prefix_update(start, increment, terminator);
        }
        Ok({
            let expression = self.expression(0)?;
            if let Kind::Step(increment) = self.peek()? {
                self.token()?;
                self.finish_update(expression, increment, start, terminator)?
            } else if matches!(self.peek()?, Kind::Punctuation('=') | Kind::Compound(_)) {
                let assignment = self.token()?.kind;
                self.assignment_target(&expression, start)?;
                let value = self.expression(0)?;
                let end = self.statement_end(terminator)?;
                self.assignment_statement(expression, assignment, value, Span { start, end })?
            } else {
                let end = self.statement_end(terminator)?;
                if matches!(
                    expression.kind,
                    ExpressionKind::ValueCall { .. }
                        | ExpressionKind::UnqualifiedCall { .. }
                        | ExpressionKind::GlobalCall { .. }
                        | ExpressionKind::OwnerCall { .. }
                ) {
                    Statement::ValueCall {
                        expression,
                        span: Span { start, end },
                    }
                } else {
                    let ExpressionKind::ModuleCall {
                        module,
                        member,
                        mut arguments,
                    } = expression.kind
                    else {
                        return Err(self.error(
                            start,
                            "only assignment and calls are supported as expression statements",
                        ));
                    };
                    if matches!(
                        module.as_str(),
                        "Application.AppBase" | "WatchUi.View" | "WatchUi.BehaviorDelegate"
                    ) {
                        if member != "initialize" || !arguments.is_empty() {
                            return Err(self.error(
                                start,
                                "only a zero-argument native base initialize call is supported",
                            ));
                        }
                        Statement::BaseInitialize {
                            span: Span { start, end },
                            base: if module == "Application.AppBase" {
                                crate::ClassBase::ApplicationAppBase
                            } else if module == "WatchUi.View" {
                                crate::ClassBase::WatchUiView
                            } else {
                                crate::ClassBase::WatchUiBehaviorDelegate
                            },
                        }
                    } else if module == "System" {
                        if member == "println" {
                            if arguments.len() != 1 {
                                return Err(
                                    self.error(start, "expected System.println with one argument")
                                );
                            }
                            Statement::Println {
                                argument: arguments.remove(0),
                                span: Span { start, end },
                            }
                        } else {
                            Statement::NativeModuleCall {
                                module: "Toybox.System".into(),
                                member,
                                arguments,
                                span: Span { start, end },
                            }
                        }
                    } else if module == "WatchUi" {
                        Statement::NativeModuleCall {
                            module: "Toybox.WatchUi".into(),
                            member,
                            arguments,
                            span: Span { start, end },
                        }
                    } else {
                        Statement::Call {
                            module,
                            member,
                            arguments,
                            span: Span { start, end },
                        }
                    }
                }
            }
        })
    }

    /// Parse a runtime class target shared by expressions and switch type cases.
    fn runtime_class(&mut self) -> Result<String, Diagnostic> {
        let at = self.offset;
        let class = self.type_path()?;
        if self.alias_key(&class).is_some() {
            return Err(self.error(at, "typedef names are not runtime classes"));
        }
        if class.contains('.') && crate::native_bindings::lang_class(&class).is_none() {
            return Err(self.error(at, "instanceof requires a supported runtime class"));
        }
        Ok(class)
    }

    /// Parse ordered case groups with shared switch scope. Labels end the
    /// preceding sequence; nested blocks and switches consume their own labels.
    fn switch_statement(&mut self, depth: usize) -> Result<Statement, Diagnostic> {
        let start = self.expect(Kind::Word("switch"), "'switch'")?.span.start;
        self.expect(Kind::Punctuation('('), "'('")?;
        let selector = self.expression(0)?;
        self.expect(Kind::Punctuation(')'), "')'")?;
        self.expect(Kind::Punctuation('{'), "'{' after switch selector")?;
        let mut cases = Vec::new();
        let mut default = false;
        while self.peek()? != Kind::Punctuation('}') {
            let label = self.token()?;
            self.expressions += 1;
            if self.expressions > crate::MAX_OPERATIONS {
                return Err(self.error(label.span.start, "function exceeds 1024-operation limit"));
            }
            let pattern = match label.kind {
                Kind::Word("case") => {
                    if self.peek()? == Kind::Word("instanceof") {
                        self.token()?;
                        CasePattern::Class(self.runtime_class()?)
                    } else {
                        CasePattern::Value(self.expression(0)?)
                    }
                }
                Kind::Word("default") if !default => {
                    default = true;
                    CasePattern::Default
                }
                Kind::Word("default") => {
                    return Err(self.error(label.span.start, "duplicate switch default"));
                }
                _ => return Err(self.error(label.span.start, "expected switch case or default")),
            };
            let colon = self.expect(Kind::Punctuation(':'), "':' after switch label")?;
            let body = self.statements(depth + 1, true)?;
            let end = body.last().map_or(colon.span.end, |s| s.span().end);
            cases.push(SwitchCase {
                pattern,
                body,
                span: Span {
                    start: label.span.start,
                    end,
                },
            });
        }
        let end = self.token()?.span.end;
        if cases.is_empty() {
            return Err(self.error(start, "switch requires at least one case or default"));
        }
        Ok(Statement::Switch {
            selector,
            cases,
            span: Span { start, end },
        })
    }

    /// Parse an if/else chain, representing each else-if as a nested false arm.
    /// Each link counts toward the shared nesting and source-operation limits.
    fn if_statement(&mut self, depth: usize) -> Result<Statement, Diagnostic> {
        let start = self.expect(Kind::Word("if"), "'if'")?.span.start;
        self.expressions += 1;
        if self.expressions > crate::MAX_OPERATIONS {
            return Err(self.error(start, "function exceeds 1024-operation limit"));
        }
        self.expect(Kind::Punctuation('('), "'('")?;
        let condition = self.expression(0)?;
        self.expect(Kind::Punctuation(')'), "')'")?;
        let (on_true, true_span) = self.block(depth + 1)?;
        let (on_false, end) = if self.peek()? == Kind::Word("else") {
            self.token()?;
            if self.peek()? == Kind::Word("if") {
                let nested = self.if_statement(depth + 1)?;
                let end = nested.span().end;
                (vec![nested], end)
            } else {
                let (body, span) = self.block(depth + 1)?;
                (body, span.end)
            }
        } else {
            (Vec::new(), true_span.end)
        };
        Ok(Statement::If {
            condition,
            on_true,
            on_false,
            span: Span { start, end },
        })
    }

    /// Parse a braced statement sequence, bounding control-flow nesting and total
    /// source operations before semantic construction.
    fn block(&mut self, depth: usize) -> Result<(Vec<Statement>, Span), Diagnostic> {
        let start = self
            .expect(Kind::Punctuation('{'), "'{' (blocks require braces)")?
            .span
            .start;
        let statements = self.statements(depth, false)?;
        let end = self.expect(Kind::Punctuation('}'), "'}'")?.span.end;
        Ok((statements, Span { start, end }))
    }

    /// Parse statements until a closing brace or, within a switch, its next
    /// label. All constructs share scope-independent nesting and node limits.
    fn statements(&mut self, depth: usize, case_body: bool) -> Result<Vec<Statement>, Diagnostic> {
        if depth > 64 {
            return Err(self.error(self.offset, "control-flow nesting exceeds 64 levels"));
        }
        let mut statements = Vec::new();
        while self.peek()? != Kind::Punctuation('}')
            && !(case_body && matches!(self.peek()?, Kind::Word("case" | "default")))
        {
            let start = self.lookahead.as_ref().unwrap().span.start;
            // A print's call node or assignment target already counts its sink.
            if matches!(
                self.peek()?,
                Kind::Word(
                    "var" | "return" | "while" | "for" | "do" | "break" | "continue" | "switch"
                ) | Kind::Punctuation('{')
            ) {
                self.expressions += 1;
            }
            if self.expressions > crate::MAX_OPERATIONS {
                return Err(self.error(start, "function exceeds 1024-operation limit"));
            }
            if self.peek()? == Kind::Word("var") {
                statements.extend(self.local_declarations(';', true)?);
                continue;
            }
            let statement = match self.peek()? {
                Kind::Punctuation('{') => {
                    let (body, mut span) = self.block(depth + 1)?;
                    if self.peek()? == Kind::Punctuation(';') {
                        span.end = self.statement_end(Some(';'))?;
                    }
                    Statement::Block { body, span }
                }
                Kind::Word("return") => {
                    self.token()?;
                    let value = if self.peek()? == Kind::Punctuation(';') {
                        None
                    } else {
                        Some(self.expression(0)?)
                    };
                    let end = self.expect(Kind::Punctuation(';'), "';'")?.span.end;
                    Statement::Return {
                        value,
                        span: Span { start, end },
                    }
                }
                Kind::Word("break" | "continue") => {
                    let keyword = self.token()?.kind;
                    let end = self.expect(Kind::Punctuation(';'), "';'")?.span.end;
                    if keyword == Kind::Word("break") {
                        Statement::Break {
                            span: Span { start, end },
                        }
                    } else {
                        Statement::Continue {
                            span: Span { start, end },
                        }
                    }
                }
                Kind::Word("for") => {
                    self.token()?;
                    self.expect(Kind::Punctuation('('), "'('")?;
                    let initialize = self.for_header(';', true)?;
                    let condition = if self.peek()? == Kind::Punctuation(';') {
                        None
                    } else {
                        Some(self.expression(0)?)
                    };
                    self.expect(Kind::Punctuation(';'), "';' after for condition")?;
                    let update = self.for_header(')', false)?;
                    let (body, body_span) = self.block(depth + 1)?;
                    Statement::For {
                        initialize,
                        condition,
                        update,
                        body,
                        span: Span {
                            start,
                            end: body_span.end,
                        },
                    }
                }
                Kind::Word("do") => {
                    self.token()?;
                    let (body, _) = self.block(depth + 1)?;
                    self.expect(Kind::Word("while"), "'while' after do body")?;
                    self.expect(Kind::Punctuation('('), "'('")?;
                    let condition = self.expression(0)?;
                    self.expect(Kind::Punctuation(')'), "')'")?;
                    let end = self.statement_end(Some(';'))?;
                    Statement::DoWhile {
                        body,
                        condition,
                        span: Span { start, end },
                    }
                }
                Kind::Word("while") => {
                    self.token()?;
                    self.expect(Kind::Punctuation('('), "'('")?;
                    let condition = self.expression(0)?;
                    self.expect(Kind::Punctuation(')'), "')'")?;
                    let (body, body_span) = self.block(depth + 1)?;
                    Statement::While {
                        condition,
                        body,
                        span: Span {
                            start,
                            end: body_span.end,
                        },
                    }
                }
                Kind::Word("if") => self.if_statement(depth)?,
                Kind::Word("switch") => self.switch_statement(depth)?,
                _ => self.simple_statement(Some(';'))?,
            };
            statements.push(statement);
        }
        Ok(statements)
    }

    fn expect(&mut self, expected: Kind<'a>, description: &str) -> Result<Token<'a>, Diagnostic> {
        let token = self.token()?;
        if token.kind != expected {
            return Err(self.error(token.span.start, format!("expected {description}")));
        }
        Ok(token)
    }
}

impl Parser<'_> {
    /// Parse source interface requirements without executable member bodies.
    /// Preserve source identities using the ordinary parameter/result contracts.
    fn structural_annotation(&mut self) -> Result<crate::StructuralInterface, Diagnostic> {
        let start = self
            .expect(Kind::Word("interface"), "'interface'")?
            .span
            .start;
        self.expect(Kind::Punctuation('{'), "'{' after interface")?;
        let mut shape = crate::StructuralInterface {
            declaration: None,
            fields: BTreeMap::new(),
            methods: BTreeMap::new(),
        };
        while self.peek()? != Kind::Punctuation('}') {
            if shape.fields.len() + shape.methods.len() >= 256 {
                return Err(self.error(start, "interface exceeds 256 members"));
            }
            let kind = self.token()?;
            if !matches!(kind.kind, Kind::Word("var" | "function")) {
                return Err(
                    self.error(kind.span.start, "expected interface var or function member")
                );
            }
            let name = self.identifier()?;
            let Kind::Word(name) = name.kind else {
                unreachable!()
            };
            if shape.fields.contains_key(name) || shape.methods.contains_key(name) {
                return Err(self.error(kind.span.start, "duplicate interface member"));
            }
            if kind.kind == Kind::Word("var") {
                let ty = self.structural_member_type(start, false)?;
                shape.fields.insert(name.into(), ty);
            } else {
                self.expect(Kind::Punctuation('('), "'('")?;
                let mut parameters = Vec::new();
                let mut names = BTreeSet::new();
                if self.peek()? != Kind::Punctuation(')') {
                    loop {
                        if parameters.len() >= crate::MAX_PARAMETERS {
                            return Err(
                                self.error(start, "interface method exceeds 254 parameters")
                            );
                        }
                        let argument = self.identifier()?;
                        let Kind::Word(argument) = argument.kind else {
                            unreachable!()
                        };
                        if !names.insert(argument) {
                            return Err(self.error(start, "duplicate interface parameter"));
                        }
                        let ty = self.structural_member_type(start, false)?;
                        parameters.push(
                            ty.source_contract()
                                .map(crate::SourceUnion::parameter)
                                .or_else(|| crate::unions::parameter(ty.api_type()))
                                .ok_or_else(|| {
                                    self.error(start, "unsupported interface parameter")
                                })?,
                        );
                        if self.peek()? != Kind::Punctuation(',') {
                            break;
                        }
                        self.token()?;
                    }
                }
                self.expect(Kind::Punctuation(')'), "')'")?;
                let result = self.structural_member_type(start, true)?;
                shape
                    .methods
                    .insert(name.into(), crate::StructuralMethod { parameters, result });
            }
            self.expect(Kind::Punctuation(';'), "';' after interface member")?;
        }
        self.token()?;
        if shape.fields.values().any(crate::field_flow::positional) {
            shape.declaration = Some(crate::StructuralDeclaration {
                source: self.source_index,
                offset: start,
            });
        }
        shape
            .validate()
            .map_err(|message| self.error(start, message))?;
        Ok(shape)
    }

    fn structural_member_type(
        &mut self,
        at: usize,
        result: bool,
    ) -> Result<crate::ReturnType, Diagnostic> {
        if self.peek()? != Kind::Word("as") {
            return Ok(crate::ReturnType::Any);
        }
        self.token()?;
        if result && self.peek()? == Kind::Word("Void") {
            self.token()?;
            return Ok(crate::ReturnType::Void);
        }
        let parameter = self.parameter_annotation(at)?;
        parameter
            .source_contract()
            .map(crate::SourceUnion::result)
            .or_else(|| crate::unions::result(parameter.api_type()))
            .ok_or_else(|| self.error(at, "unsupported interface member type"))
    }

    /// Parse one existing signature atom, including bounded container parameters.
    fn parameter_atom(&mut self) -> Result<crate::ParameterType, Diagnostic> {
        let at = self.offset;
        if self.peek()? == Kind::Punctuation('{') {
            self.token()?;
            let mut fields = BTreeMap::new();
            while self.peek()? != Kind::Punctuation('}') {
                self.expect(Kind::Punctuation(':'), "literal Symbol field")?;
                let Kind::Word(name) = self.identifier()?.kind else {
                    unreachable!()
                };
                let name = name.to_owned();
                self.expect(Kind::Word("as"), "'as' and a record field type")?;
                let field = self.parameter_annotation(self.offset)?;
                let ty = field
                    .source_contract()
                    .map(|c| crate::api::Type::SourceValue(Box::new(c)))
                    .unwrap_or_else(|| field.api_type());
                if fields.insert(name, ty).is_some() {
                    return Err(self.error(at, "duplicate record key"));
                }
                if self.peek()? != Kind::Punctuation(',') {
                    break;
                }
                self.token()?;
            }
            self.expect(Kind::Punctuation('}'), "'}'")?;
            crate::records::validate(&fields).map_err(|message| self.error(at, message))?;
            return Ok(crate::ParameterType::Options(fields));
        }
        if self.peek()? == Kind::Punctuation('[') {
            self.token()?;
            let mut slots = Vec::new();
            loop {
                let slot = self.parameter_annotation(self.offset)?;
                slots.push(
                    slot.source_contract()
                        .map(crate::SourceUnion::result)
                        .or_else(|| crate::unions::result(slot.api_type()))
                        .ok_or_else(|| self.error(at, "unsupported tuple slot contract"))?,
                );
                if self.peek()? != Kind::Punctuation(',') {
                    break;
                }
                self.token()?;
            }
            self.expect(Kind::Punctuation(']'), "']'")?;
            return crate::TupleType::new(slots)
                .map(crate::ParameterType::Tuple)
                .map_err(|message| self.error(at, message));
        }
        if self.peek()? == Kind::Word("interface") {
            return self
                .structural_annotation()
                .map(|shape| crate::ParameterType::Structural(Box::new(shape)));
        }
        if self.peek()? == Kind::Word("Method") {
            self.token()?;
            if self.peek()? != Kind::Punctuation('(') {
                return Ok(crate::ParameterType::Method);
            }
            self.token()?;
            let mut parameters = Vec::new();
            let mut names = BTreeSet::new();
            if self.peek()? != Kind::Punctuation(')') {
                loop {
                    let name = self.identifier()?;
                    let Kind::Word(name) = name.kind else {
                        unreachable!()
                    };
                    if !names.insert(name) {
                        return Err(self.error(at, "duplicate callback parameter"));
                    }
                    let ty = self.structural_member_type(at, false)?;
                    parameters.push(
                        ty.source_contract()
                            .map(crate::SourceUnion::parameter)
                            .or_else(|| crate::unions::parameter(ty.api_type()))
                            .ok_or_else(|| self.error(at, "unsupported callback parameter"))?,
                    );
                    if parameters.len() > crate::MAX_PARAMETERS {
                        return Err(self.error(at, "callback exceeds 254 parameters"));
                    }
                    if self.peek()? != Kind::Punctuation(',') {
                        break;
                    }
                    self.token()?;
                }
            }
            self.expect(Kind::Punctuation(')'), "')'")?;
            let result = self.structural_member_type(at, true)?;
            let signature = crate::StructuralMethod { parameters, result };
            crate::callbacks::validate(&signature).map_err(|message| self.error(at, message))?;
            return Ok(crate::ParameterType::Callback(Box::new(signature)));
        }
        let path = self.path()?;
        if let Some(ty) = self.alias_parameter(&path, at)? {
            return Ok(ty);
        }
        let path = self
            .names
            .type_name(&path)
            .map_err(|message| self.error(at, message))?
            .unwrap_or(path);
        let ty = match path.as_str() {
            "Null" => crate::ParameterType::Null,
            "Toybox.Lang.Number" => crate::ParameterType::Integer,
            "Toybox.Lang.Object" => crate::ParameterType::LangObject,
            "Toybox.Lang.WeakReference" => crate::ParameterType::WeakReference,
            "Toybox.Lang.ByteArray" => crate::ParameterType::ByteArray,
            "Toybox.Lang.Comparable" => {
                crate::ParameterType::Interface(crate::InterfaceType::Comparable)
            }
            "Toybox.Lang.Comparator" => {
                crate::ParameterType::Interface(crate::InterfaceType::Comparator)
            }
            "Toybox.Lang.Method" => crate::ParameterType::Method,
            "Toybox.Lang.Symbol" => crate::ParameterType::Symbol,
            "Toybox.Lang.Long" => crate::ParameterType::Long,
            "Toybox.Lang.Double" => crate::ParameterType::Double,
            "Toybox.Lang.Float" => crate::ParameterType::Float,
            "Toybox.Lang.Numeric" => crate::ParameterType::Numeric,
            "Toybox.Lang.Integer" => crate::ParameterType::IntegerAlias,
            "Toybox.Lang.Decimal" => crate::ParameterType::Decimal,
            "Toybox.Lang.NumberFormat" => crate::ParameterType::NumberFormat,
            "Toybox.Lang.Endian" => crate::ParameterType::Endian,
            "Toybox.Lang.String" => crate::ParameterType::String,
            "Toybox.Lang.Boolean" => crate::ParameterType::Boolean,
            "Toybox.Lang.Char" => crate::ParameterType::Char,
            "Toybox.Lang.Dictionary" => self
                .dictionary_annotation()?
                .map(crate::ParameterType::TypedDictionary)
                .unwrap_or(crate::ParameterType::Dictionary),
            "Toybox.WatchUi.View" => crate::ParameterType::Object(crate::NativeClass::View),
            "Toybox.Application.AppBase" => {
                crate::ParameterType::Object(crate::NativeClass::AppBase)
            }
            "Toybox.WatchUi.BehaviorDelegate" => {
                crate::ParameterType::Object(crate::NativeClass::BehaviorDelegate)
            }
            "Toybox.Graphics.Dc" => crate::ParameterType::Object(crate::NativeClass::Dc),
            "Toybox.Lang.Array" => crate::ParameterType::Array(self.array_annotation()?),
            name if !name.contains('.')
                && !matches!(
                    name,
                    "Number"
                        | "Long"
                        | "String"
                        | "Boolean"
                        | "Char"
                        | "Array"
                        | "Object"
                        | "View"
                        | "Dc"
                        | "Void"
                ) =>
            {
                crate::ParameterType::SourceObject(crate::SourceClass {
                    name: name.into(),
                    base: crate::ClassBase::LangObject,
                })
            }
            _ => return Err(self.error(at, "unsupported parameter type annotation")),
        };
        Ok(ty)
    }

    /// Parse `or` alternatives, normalizing native unions without erasing a
    /// source class into its native base. Source and native alternatives retain distinct contracts.
    fn parameter_annotation(&mut self, at: usize) -> Result<crate::ParameterType, Diagnostic> {
        self.type_annotation(at, false)
    }

    /// Share value-type syntax between declarations and assertions. In an
    /// assertion, expression-only tokens after `or` retain logical precedence.
    fn type_annotation(
        &mut self,
        at: usize,
        assertion: bool,
    ) -> Result<crate::ParameterType, Diagnostic> {
        if self.type_depth >= 64 {
            return Err(self.error(at, "type annotation exceeds 64 levels"));
        }
        self.type_depth += 1;
        let result = self.parameter_annotation_inner(at, assertion);
        self.type_depth -= 1;
        result
    }

    fn parameter_annotation_inner(
        &mut self,
        at: usize,
        assertion: bool,
    ) -> Result<crate::ParameterType, Diagnostic> {
        let first = self.parameter_atom()?;
        if self.peek()? != Kind::Word("or") {
            return Ok(first);
        }
        let mut parts = vec![first];
        while self.peek()? == Kind::Word("or") {
            let saved = self.token()?.span.start;
            if assertion
                && !matches!(self.peek()?,Kind::Word(word) if !matches!(word,"true"|"false"|"not"|"new"))
                && !matches!(self.peek()?, Kind::Punctuation('{' | '['))
            {
                self.offset = saved;
                self.lookahead = None;
                break;
            }
            if parts.len() >= crate::MAX_PARAMETERS {
                return Err(self.error(at, "type union exceeds 254 alternatives"));
            }
            parts.push(self.parameter_atom()?);
        }
        let mut sources = Vec::new();
        let mut native = Vec::new();
        for part in &parts {
            if let Some(contract) = part.source_contract() {
                sources.extend(contract.classes().iter().cloned());
                native.extend(contract.native().iter().cloned());
                if contract.nullable() {
                    native.push(crate::api::Type::Null);
                }
            } else {
                native.push(part.api_type());
            }
        }
        if !sources.is_empty() {
            return crate::SourceUnion::with_native(sources, native, false)
                .map(crate::SourceUnion::parameter)
                .map_err(|message| self.error(at, message));
        }
        crate::unions::parameter(crate::unions::normalize(
            parts.iter().map(crate::ParameterType::api_type).collect(),
        ))
        .ok_or_else(|| self.error(at, "unsupported native union annotation"))
    }
    fn function(&mut self) -> Result<ParsedFunction, Diagnostic> {
        self.expressions = 0;
        self.expect(Kind::Word("function"), "'function'")?;
        let name = self.identifier()?;
        self.reject_binding(&name)?;
        let Kind::Word(identifier) = name.kind else {
            unreachable!()
        };
        self.expect(Kind::Punctuation('('), "'('")?;
        let mut parameters = Vec::new();
        let mut parameter_types = Vec::new();
        if self.peek()? != Kind::Punctuation(')') {
            loop {
                let parameter = self.identifier()?;
                self.reject_binding(&parameter)?;
                let Kind::Word(name) = parameter.kind else {
                    unreachable!()
                };
                if parameters.len() >= crate::MAX_PARAMETERS {
                    return Err(
                        self.error(parameter.span.start, "function exceeds 254-parameter limit")
                    );
                }
                if parameters.iter().any(|p| p == name) {
                    return Err(self.error(
                        parameter.span.start,
                        format!("duplicate parameter {name:?}"),
                    ));
                }
                parameters.push(name.to_owned());
                parameter_types.push(if self.peek()? == Kind::Word("as") {
                    self.token()?;
                    let at = self.offset;
                    Some(self.parameter_annotation(at)?)
                } else {
                    None
                });
                if self.peek()? != Kind::Punctuation(',') {
                    break;
                }
                self.token()?;
            }
        }
        self.expect(Kind::Punctuation(')'), "')'")?;
        let result_type = if self.peek()? == Kind::Word("as") {
            self.token()?;
            let at = self.offset;
            if self.peek()? == Kind::Word("Void") {
                self.token()?;
                Some(crate::ReturnType::Void)
            } else {
                let parameter = self.parameter_annotation(at)?;
                Some(match parameter {
                    crate::ParameterType::SourceUnion(contract) => contract.result(),
                    crate::ParameterType::SourceObject(class) => {
                        crate::ReturnType::SourceObject(class)
                    }
                    crate::ParameterType::NullableSourceObject(class) => {
                        crate::ReturnType::NullableSourceObject(class)
                    }
                    other => crate::unions::result(other.api_type())
                        .ok_or_else(|| self.error(at, "unsupported result type annotation"))?,
                })
            }
        } else {
            None
        };
        let (statements, body_span) = self.block(0)?;
        Ok(ParsedFunction {
            name: identifier.to_owned(),
            parameters,
            parameter_types,
            result_type,
            statements,
            body_span,
        })
    }
}

pub(crate) struct ParsedUnit {
    pub shared: Vec<(Option<String>, ParsedConstant)>,
    pub functions: Vec<(Option<String>, ParsedFunction)>,
    pub classes: Vec<ParsedClass>,
}
pub(crate) struct ParsedConstant {
    pub name: String,
    pub annotation: Option<crate::ReturnType>,
    pub initializer: Expression,
    pub span: Span,
}
pub(crate) struct ParsedClass {
    pub constants: Vec<ParsedConstant>,
    pub name: String,
    pub name_span: Span,
    pub base: crate::ClassBase,
    pub fields: Vec<crate::InstanceField>,
    pub methods: Vec<ParsedFunction>,
}

/// Parse one source file into owned declarations, retaining function bodies.
/// Resolves file-local native names without binding user calls or selecting
/// VM instructions. Both fragment and compilation-unit inputs use this parser.
pub(crate) fn declarations<'a>(
    source: &'a str,
    declarations: Rc<Declarations<'a>>,
    source_index: usize,
) -> Result<ParsedUnit, Diagnostic> {
    let mut parser = Parser::new(source, declarations);
    parser.source_index = source_index;
    let mut unit = ParsedUnit {
        shared: Vec::new(),
        functions: Vec::new(),
        classes: Vec::new(),
    };
    let mut count = 0;
    parser.imports()?;
    while parser.peek()? != Kind::End {
        if parser.peek()? == Kind::Word("const") {
            unit.shared.extend(
                parser
                    .constant_declarations()?
                    .into_iter()
                    .map(|value| (None, value)),
            );
            continue;
        }
        if parser.peek()? == Kind::Word("typedef") {
            parser.skip_typedef()?;
            continue;
        }
        if matches!(parser.peek()?, Kind::Word("module" | "class")) {
            let is_class = parser.token()?.kind == Kind::Word("class");
            let token = parser.identifier()?;
            parser.reject_binding(&token)?;
            let Kind::Word(name) = token.kind else {
                unreachable!()
            };
            let mut class_base = None;
            if is_class {
                let base = if parser.peek()? == Kind::Word("extends") {
                    parser.token()?;
                    parser.path()?
                } else {
                    "Toybox.Lang.Object".into()
                };
                class_base = if parser.names.object(&base) {
                    Some(crate::ClassBase::LangObject)
                } else if parser.names.app_base(&base) {
                    Some(crate::ClassBase::ApplicationAppBase)
                } else if parser.names.view(&base) {
                    Some(crate::ClassBase::WatchUiView)
                } else if parser.names.behavior_delegate(&base) {
                    Some(crate::ClassBase::WatchUiBehaviorDelegate)
                } else {
                    None
                };
                if class_base.is_none() {
                    return Err(parser.error(
                        token.span.start,
                        format!("unsupported or unresolved native superclass {base:?}"),
                    ));
                }
            } else if name == "System" {
                return Err(parser.error(token.span.start, "System is reserved for SDK intrinsics"));
            }
            parser.expect(Kind::Punctuation('{'), "'{'")?;
            let outer_names = parser.names.clone();
            parser.owner = Some(name.into());
            parser.native_base = class_base;
            parser.imports()?;
            let mut methods = Vec::new();
            let mut fields = Vec::new();
            let mut constants = Vec::new();
            while parser.peek()? != Kind::Punctuation('}') {
                if parser.peek()? == Kind::Word("const") {
                    let declarations = parser.constant_declarations()?;
                    if is_class {
                        constants.extend(declarations);
                    } else {
                        unit.shared.extend(
                            declarations
                                .into_iter()
                                .map(|value| (Some(name.into()), value)),
                        );
                    }
                    continue;
                }
                if parser.peek()? == Kind::Word("typedef") {
                    parser.skip_typedef()?;
                    continue;
                }
                if is_class && parser.peek()? == Kind::Word("var") {
                    parser.token()?;
                    let token = parser.identifier()?;
                    parser.reject_binding(&token)?;
                    let Kind::Word(name) = token.kind else {
                        unreachable!()
                    };
                    let kind = if parser.peek()? == Kind::Word("as") {
                        parser.token()?;
                        let parameter = parser.parameter_annotation(token.span.start)?;
                        let result = parameter
                            .source_contract()
                            .map(crate::SourceUnion::result)
                            .or_else(|| crate::unions::result(parameter.api_type()))
                            .ok_or_else(|| {
                                parser.error(token.span.start, "unsupported field type")
                            })?;
                        let mut kind = crate::FieldKind::from_result(result).ok_or_else(|| {
                            parser.error(token.span.start, "unsupported field type")
                        })?;
                        if parser.peek()? == Kind::Punctuation('=') {
                            parser.token()?;
                            if let crate::FieldKind::Symbol { initial, nullable } = &mut kind {
                                let value = parser.expression(0)?;
                                match value.kind {
                                    ExpressionKind::Symbol(name) => *initial=Some(name),
                                    ExpressionKind::Null if *nullable => {},
                                    _ => return Err(parser.error(value.span.start,"Symbol fields require a literal Symbol or permitted null default")),
                                }
                            } else {
                                if !crate::native::accepts(
                                    &parameter.api_type(),
                                    &crate::api::Type::Null,
                                ) {
                                    return Err(parser.error(
                                        token.span.start,
                                        "non-null fields must be assigned in initialize",
                                    ));
                                }
                                parser
                                    .expect(Kind::Word("null"), "literal null field initializer")?;
                            }
                        }
                        parser.expect(Kind::Punctuation(';'), "';' after field declaration")?;
                        kind
                    } else {
                        parser.expect(Kind::Punctuation('='), "'=' and a Number literal")?;
                        let value = parser.expression(0)?;
                        let kind = match value.kind {
                            ExpressionKind::Integer(initial) => crate::FieldKind::Number(initial),
                            ExpressionKind::Symbol(name) => crate::FieldKind::Symbol {
                                initial: Some(name),
                                nullable: false,
                            },
                            _ => return Err(parser.error(
                                value.span.start,
                                "instance fields require a Number or Symbol literal initializer",
                            )),
                        };
                        parser.expect(Kind::Punctuation(';'), "';'")?;
                        kind
                    };
                    fields.push(crate::InstanceField {
                        name: name.into(),
                        kind,
                    });
                    if fields.len() > 256 {
                        return Err(
                            parser.error(token.span.start, "class exceeds 256 instance fields")
                        );
                    }
                    continue;
                }
                methods.push(parser.function()?);
                count += 1;
                if count > crate::unit::MAX_FUNCTIONS {
                    return Err(parser.error(
                        token.span.start,
                        "compilation unit exceeds 256-function limit",
                    ));
                }
            }
            parser.token()?;
            parser.names = outer_names;
            parser.owner = None;
            parser.native_base = None;
            let has_constants = parser
                .declarations
                .constants
                .keys()
                .any(|key| key.starts_with(&format!("{name}.")));
            let has_aliases = parser
                .declarations
                .aliases
                .keys()
                .any(|key| key.starts_with(&format!("{name}.")));
            if methods.is_empty()
                && class_base != Some(crate::ClassBase::LangObject)
                && !has_aliases
                && !has_constants
            {
                return Err(parser.error(token.span.start, "empty modules/classes are unsupported"));
            }
            if is_class {
                unit.classes.push(ParsedClass {
                    constants,
                    name: name.into(),
                    name_span: token.span,
                    base: class_base.unwrap(),
                    fields,
                    methods,
                });
            } else {
                unit.functions
                    .extend(methods.into_iter().map(|f| (Some(name.into()), f)));
            }
        } else {
            unit.functions.push((None, parser.function()?));
            count += 1;
        }
        if count > crate::unit::MAX_FUNCTIONS {
            return Err(parser.error(parser.offset, "compilation unit exceeds 256-function limit"));
        }
    }
    Ok(unit)
}

/// Decode one source literal character or escape to its scalar/code-unit value.
/// Unicode escapes contain exactly four hexadecimal digits; surrogate code
/// units remain available for Char literals and String pair validation.
fn literal_character(characters: &mut std::str::Chars<'_>) -> Result<u32, &'static str> {
    let value = characters.next().ok_or("unterminated source literal")?;
    if value != '\\' {
        return Ok(u32::from(value));
    }
    Ok(
        match characters
            .next()
            .ok_or("unfinished source literal escape")?
        {
            'b' => 8,
            'f' => 12,
            'n' => 10,
            'r' => 13,
            't' => 9,
            '\'' => 39,
            '"' => 34,
            '\\' => 92,
            'u' => {
                let mut value = 0;
                for _ in 0..4 {
                    let digit = characters
                        .next()
                        .filter(char::is_ascii_hexdigit)
                        .and_then(|c| c.to_digit(16))
                        .ok_or("Unicode escape requires four hexadecimal digits")?;
                    value = value * 16 + digit;
                }
                value
            }
            _ => return Err("unsupported source literal escape"),
        },
    )
}

/// Decode a complete quoted source String token to UTF-8, rejecting unpaired
/// surrogate escapes instead of replacing them. Escapes are not recursive.
fn string_literal(literal: &str) -> Result<String, &'static str> {
    let mut characters = literal[1..literal.len() - 1].chars();
    let mut value = String::new();
    while !characters.as_str().is_empty() {
        let mut scalar = literal_character(&mut characters)?;
        if (0xd800..=0xdbff).contains(&scalar) {
            if !characters.as_str().starts_with("\\u") {
                return Err("String high surrogate requires a low-surrogate escape");
            }
            let low = literal_character(&mut characters)?;
            if !(0xdc00..=0xdfff).contains(&low) {
                return Err("String high surrogate requires a low-surrogate escape");
            }
            scalar = 0x10000 + ((scalar - 0xd800) << 10) + low - 0xdc00;
        }
        value.push(char::from_u32(scalar).ok_or("String literal contains an unpaired surrogate")?);
    }
    Ok(value)
}
