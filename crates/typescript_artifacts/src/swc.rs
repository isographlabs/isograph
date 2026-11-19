//! The one module that names swc: converts `TypeScriptModule`s into swc nodes and prints them.

use isograph_compiler::ArtifactPath;
use prelude::Postfix;
use swc_common::comments::{Comment, CommentKind, Comments, SingleThreadedComments};
use swc_common::sync::Lrc;
use swc_common::{BytePos, DUMMY_SP, FileName, SourceMap, Span, SyntaxContext};
use swc_ecma_ast::{
    ArrayLit, ArrowExpr, ArrowFunctionBody, BindingIdent, Bool, CallExpr, Callee, Decl, EsVersion,
    ExportDecl, ExportDefaultExpr, Expr, ExprOrSpread, FnDecl, Function, FunctionBody, Ident,
    IdentName, ImportDecl, ImportDefaultSpecifier, ImportNamedSpecifier, ImportPhase,
    ImportSpecifier, KeyValueProp, Lit, Module, ModuleDecl, ModuleExportName, ModuleItem, NewExpr,
    Null, Number, ObjectLit, Param, ParenExpr, Pat, Prop, PropName, PropOrSpread, Stmt, Str,
    ThrowStmt, TplElement, TsConditionalType, TsEntityName, TsFnOrConstructorType, TsFnParam,
    TsFnType, TsInferType, TsIntersectionType, TsKeywordType, TsKeywordTypeKind, TsLit, TsLitType,
    TsParenthesizedType, TsPropertySignature, TsTplLitType, TsType, TsTypeAliasDecl, TsTypeAnn,
    TsTypeElement, TsTypeLit, TsTypeParam, TsTypeParamDecl, TsTypeParamInstantiation, TsTypeQuery,
    TsTypeQueryExpr, TsTypeRef, TsUnionOrIntersectionType, TsUnionType, VarDecl, VarDeclKind,
    VarDeclarator,
};
use swc_ecma_codegen::text_writer::{BindingStorage, JsWriter, ScopeKind, WriteJs};
use swc_ecma_codegen::{Config, Emitter};

use crate::imports::{artifact_import_identifier, artifact_import_specifier};
use crate::module::{
    ArtifactImport, ArtifactValue, DeclaredConst, FunctionParam, FunctionSignature, NamedImport,
    Optionality, TypeAlias, TypeParam, TypeScriptItem, TypeScriptKeyword, TypeScriptModule,
    TypeScriptType,
};

/// Gives a property's description a leading comment and returns the span to attach it to.
type CommentAt<'a> = &'a mut dyn FnMut(&str) -> Span;

/// The text of `module`, two-space indented.
pub(crate) fn print(module: &TypeScriptModule) -> String {
    let comments = SingleThreadedComments::default();
    let mut positions: u32 = 0;
    let body = {
        let mut comment_at = |description: &str| {
            positions += 1;
            let position = BytePos(positions);
            comments.add_leading(
                position,
                Comment {
                    kind: CommentKind::Block,
                    span: DUMMY_SP,
                    text: jsdoc_text(description).into(),
                },
            );
            Span::new(position, position)
        };
        module
            .items
            .iter()
            .map(|item| module_item(module.path.reference(), item, &mut comment_at))
            .collect()
    };
    let swc_module = Module {
        span: DUMMY_SP,
        body,
        shebang: None,
    };
    let source_map: Lrc<SourceMap> = Lrc::default();
    source_map.new_source_file(FileName::Anon.into(), " ".repeat(positions as usize + 1));
    let mut buffer = Vec::new();
    {
        let mut writer = JsWriter::new(source_map.clone(), "\n", &mut buffer, None);
        writer.set_indent_str("  ");
        let mut emitter = Emitter {
            cfg: Config::default().with_target(EsVersion::latest()),
            cm: source_map,
            comments: (&comments as &dyn Comments).wrap_some(),
            wr: OwnLineBlockComments {
                inner: writer,
                last_write: LastWrite::Other,
            },
        };
        emitter
            .emit_module(swc_module.reference())
            .expect("writing to a Vec does not fail");
    }
    String::from_utf8(buffer).expect("swc_ecma_codegen writes UTF-8")
}

/// The text between `/*` and `*/` of the JSDoc comment for `description`: one ` * ` line per
/// description line, then the space that aligns `*/` under the `*`s. `*/` in the description is
/// escaped.
fn jsdoc_text(description: &str) -> String {
    let escaped = description.replace("*/", "*\\/");
    let lines: String = escaped
        .split('\n')
        .map(|line| match line {
            "" => "\n *".to_owned(),
            line => format!("\n * {line}"),
        })
        .collect();
    format!("*{lines}\n ")
}

/// The last thing an `OwnLineBlockComments` wrote.
enum LastWrite {
    BlockCommentEnd,
    Other,
}

/// A writer that prints each block comment on its own lines: every newline in the comment text
/// is a line break at the current indent, and the space swc_ecma_codegen writes after `*/` is a
/// line break.
struct OwnLineBlockComments<W: WriteJs> {
    inner: W,
    last_write: LastWrite,
}

macro_rules! delegate_write {
    ($name:ident($($arg:ident: $ty:ty),*)) => {
        fn $name(&mut self, $($arg: $ty),*) -> swc_ecma_codegen::Result {
            self.last_write = LastWrite::Other;
            self.inner.$name($($arg),*)
        }
    };
}

impl<W: WriteJs> WriteJs for OwnLineBlockComments<W> {
    delegate_write!(increase_indent());
    delegate_write!(decrease_indent());
    delegate_write!(write_semi(span: Option<Span>));
    delegate_write!(write_keyword(span: Option<Span>, s: &'static str));
    delegate_write!(write_operator(span: Option<Span>, s: &str));
    delegate_write!(write_param(s: &str));
    delegate_write!(write_property(s: &str));
    delegate_write!(write_line());
    delegate_write!(write_lit(span: Span, s: &str));
    delegate_write!(write_str_lit(span: Span, s: &str));
    delegate_write!(write_str(s: &str));
    delegate_write!(write_symbol(span: Span, s: &str));
    delegate_write!(write_punct(span: Option<Span>, s: &'static str, commit_pending_semi: bool));
    delegate_write!(commit_pending_semi());
    delegate_write!(end_scope());
    delegate_write!(start_scope(
        name: Option<&str>,
        kind: ScopeKind,
        is_stack_frame: bool,
        is_hidden: bool,
        original_span: Option<Span>
    ));
    delegate_write!(add_scope_variable(
        name: &str,
        expression: Option<&str>,
        storage: BindingStorage
    ));

    fn write_space(&mut self) -> swc_ecma_codegen::Result {
        match std::mem::replace(&mut self.last_write, LastWrite::Other) {
            LastWrite::BlockCommentEnd => self.inner.write_line(),
            LastWrite::Other => self.inner.write_space(),
        }
    }

    fn write_comment(&mut self, s: &str) -> swc_ecma_codegen::Result {
        let mut lines = s.split('\n');
        lines
            .next()
            .map_or(().wrap_ok(), |first| self.inner.write_comment(first))?;
        lines.try_for_each(|line| {
            self.inner.write_line()?;
            self.inner.write_comment(line)
        })?;
        self.last_write = match s {
            "*/" => LastWrite::BlockCommentEnd,
            _ => LastWrite::Other,
        };
        ().wrap_ok()
    }

    fn care_about_srcmap(&self) -> bool {
        self.inner.care_about_srcmap()
    }

    fn add_srcmap(&mut self, pos: BytePos) -> swc_ecma_codegen::Result {
        self.inner.add_srcmap(pos)
    }

    fn can_ignore_invalid_unicodes(&mut self) -> bool {
        self.inner.can_ignore_invalid_unicodes()
    }

    fn has_scope_tracking(&self) -> bool {
        self.inner.has_scope_tracking()
    }
}

fn module_item(from: &ArtifactPath, item: &TypeScriptItem, comment_at: CommentAt) -> ModuleItem {
    match item {
        TypeScriptItem::DefaultImport(import) => default_import(from, import, TypeOnly::No),
        TypeScriptItem::TypeImport(import) => default_import(from, import, TypeOnly::Yes),
        TypeScriptItem::NamedTypeImport(import) => named_import(import, TypeOnly::Yes),
        TypeScriptItem::NamedImport(import) => named_import(import, TypeOnly::No),
        TypeScriptItem::TypeAlias(alias) => {
            ModuleItem::Stmt(Stmt::Decl(Decl::TsTypeAlias(type_alias(alias, comment_at))))
        }
        TypeScriptItem::ExportedTypeAlias(alias) => {
            ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(ExportDecl {
                span: DUMMY_SP,
                decl: Decl::TsTypeAlias(type_alias(alias, comment_at)),
            }))
        }
        TypeScriptItem::ExportedOverload(signature) => {
            exported_function(signature, None, comment_at)
        }
        TypeScriptItem::ExportedThrowingFunction(function) => exported_function(
            function.signature.reference(),
            FunctionBody {
                span: DUMMY_SP,
                stmts: Stmt::Throw(ThrowStmt {
                    span: DUMMY_SP,
                    arg: Expr::New(NewExpr {
                        span: DUMMY_SP,
                        ctxt: SyntaxContext::empty(),
                        callee: Expr::Ident(ident("Error")).boxed(),
                        args: ExprOrSpread {
                            spread: None,
                            expr: Expr::Lit(Lit::Str(string(function.message.as_str()))).boxed(),
                        }
                        .wrap_vec()
                        .wrap_some(),
                        type_args: None,
                    })
                    .boxed(),
                })
                .wrap_vec(),
            }
            .wrap_some(),
            comment_at,
        ),
        TypeScriptItem::ExportDefault(value) => {
            ModuleItem::ModuleDecl(ModuleDecl::ExportDefaultExpr(ExportDefaultExpr {
                span: DUMMY_SP,
                expr: expression(value).boxed(),
            }))
        }
        TypeScriptItem::DeclaredConst(declared) => {
            ModuleItem::Stmt(Stmt::Decl(Decl::Var(declared_const(declared, comment_at))))
        }
        TypeScriptItem::DeclaredThunk(declared) => {
            ModuleItem::Stmt(Stmt::Decl(Decl::Var(declared_thunk(declared, comment_at))))
        }
        TypeScriptItem::ExportDefaultBinding(binding) => {
            ModuleItem::ModuleDecl(ModuleDecl::ExportDefaultExpr(ExportDefaultExpr {
                span: DUMMY_SP,
                expr: Expr::Ident(ident(binding.as_str())).boxed(),
            }))
        }
    }
}

/// `const <binding>: <type> = <value>;`
fn declared_const(declared: &DeclaredConst, comment_at: CommentAt) -> Box<VarDecl> {
    VarDecl {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        kind: VarDeclKind::Const,
        declare: false,
        decls: VarDeclarator {
            span: DUMMY_SP,
            name: Pat::Ident(BindingIdent {
                id: ident(declared.binding.as_str()),
                type_ann: type_annotation(declared.type_.reference(), comment_at).wrap_some(),
            }),
            init: expression(declared.value.reference()).boxed().wrap_some(),
            definite: false,
        }
        .wrap_vec(),
    }
    .boxed()
}

/// `const <binding> = (): <type> => (<value>);`
fn declared_thunk(declared: &DeclaredConst, comment_at: CommentAt) -> Box<VarDecl> {
    VarDecl {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        kind: VarDeclKind::Const,
        declare: false,
        decls: VarDeclarator {
            span: DUMMY_SP,
            name: Pat::Ident(BindingIdent {
                id: ident(declared.binding.as_str()),
                type_ann: None,
            }),
            init: Expr::Arrow(ArrowExpr {
                span: DUMMY_SP,
                ctxt: SyntaxContext::empty(),
                params: Vec::new(),
                body: ArrowFunctionBody::Expr(
                    Expr::Paren(ParenExpr {
                        span: DUMMY_SP,
                        expr: expression(declared.value.reference()).boxed(),
                    })
                    .boxed(),
                )
                .boxed(),
                is_async: false,
                is_generator: false,
                type_params: None,
                return_type: type_annotation(declared.type_.reference(), comment_at).wrap_some(),
            })
            .boxed()
            .wrap_some(),
            definite: false,
        }
        .wrap_vec(),
    }
    .boxed()
}

fn type_alias(alias: &TypeAlias, comment_at: CommentAt) -> Box<TsTypeAliasDecl> {
    TsTypeAliasDecl {
        span: DUMMY_SP,
        declare: false,
        id: ident(alias.name.as_str()),
        type_params: type_param_decl(alias.type_params.as_slice(), comment_at),
        type_ann: ts_type(alias.aliased.reference(), comment_at),
    }
    .boxed()
}

fn named_import(import: &NamedImport, type_only: TypeOnly) -> ModuleItem {
    ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
        span: DUMMY_SP,
        specifiers: ImportSpecifier::Named(ImportNamedSpecifier {
            span: DUMMY_SP,
            local: ident(import.local.as_str()),
            imported: (import.imported != import.local)
                .then(|| ModuleExportName::Ident(ident(import.imported.as_str()))),
            is_type_only: false,
        })
        .wrap_vec(),
        src: string(import.specifier.as_str()).boxed(),
        type_only: match type_only {
            TypeOnly::Yes => true,
            TypeOnly::No => false,
        },
        with: None,
        phase: ImportPhase::Evaluation,
    }))
}

/// Whether an import declaration is `import type`.
enum TypeOnly {
    Yes,
    No,
}

fn default_import(from: &ArtifactPath, import: &ArtifactImport, type_only: TypeOnly) -> ModuleItem {
    ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
        span: DUMMY_SP,
        specifiers: ImportSpecifier::Default(ImportDefaultSpecifier {
            span: DUMMY_SP,
            local: ident(artifact_import_identifier(import).as_str()),
        })
        .wrap_vec(),
        src: string(artifact_import_specifier(from, import).as_str()).boxed(),
        type_only: match type_only {
            TypeOnly::Yes => true,
            TypeOnly::No => false,
        },
        with: None,
        phase: ImportPhase::Evaluation,
    }))
}

fn exported_function(
    signature: &FunctionSignature,
    body: Option<FunctionBody>,
    comment_at: CommentAt,
) -> ModuleItem {
    ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(ExportDecl {
        span: DUMMY_SP,
        decl: Decl::Fn(FnDecl {
            ident: ident(signature.name.as_str()),
            declare: false,
            function: Function {
                params: signature
                    .params
                    .iter()
                    .map(|param| Param {
                        span: DUMMY_SP,
                        decorators: Vec::new(),
                        pat: Pat::Ident(binding(param, comment_at)),
                    })
                    .collect(),
                decorators: Vec::new(),
                span: DUMMY_SP,
                ctxt: SyntaxContext::empty(),
                body,
                is_generator: false,
                is_async: false,
                type_params: type_param_decl(signature.type_params.as_slice(), comment_at),
                return_type: type_annotation(signature.return_type.reference(), comment_at)
                    .wrap_some(),
                this_param: None,
            }
            .boxed(),
        }),
    }))
}

fn expression(value: &ArtifactValue) -> Expr {
    match value {
        ArtifactValue::Null => Expr::Lit(Lit::Null(Null { span: DUMMY_SP })),
        ArtifactValue::Boolean(value) => Expr::Lit(Lit::Bool(Bool {
            span: DUMMY_SP,
            value: value.dereference(),
        })),
        ArtifactValue::Number(value) => Expr::Lit(Lit::Num(Number {
            span: DUMMY_SP,
            value: value.dereference(),
            raw: None,
        })),
        ArtifactValue::String(value) => Expr::Lit(Lit::Str(string(value.as_str()))),
        ArtifactValue::Array(items) => Expr::Array(ArrayLit {
            span: DUMMY_SP,
            elems: items
                .iter()
                .map(|item| {
                    ExprOrSpread {
                        spread: None,
                        expr: expression(item).boxed(),
                    }
                    .wrap_some()
                })
                .collect(),
        }),
        ArtifactValue::Object(entries) => Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: entries
                .iter()
                .map(|entry| {
                    PropOrSpread::Prop(
                        Prop::KeyValue(KeyValueProp {
                            key: property_name(entry.key.as_str()),
                            value: expression(entry.value.reference()).boxed(),
                        })
                        .boxed(),
                    )
                })
                .collect(),
        }),
        ArtifactValue::Import(import) => {
            Expr::Ident(ident(artifact_import_identifier(import).as_str()))
        }
        ArtifactValue::NamedImport(import) => Expr::Ident(ident(import.local.as_str())),
        ArtifactValue::ImportCallThunk(import) => Expr::Arrow(ArrowExpr {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            params: Vec::new(),
            body: ArrowFunctionBody::Expr(
                Expr::Call(CallExpr {
                    span: DUMMY_SP,
                    ctxt: SyntaxContext::empty(),
                    callee: Callee::Expr(
                        Expr::Ident(ident(artifact_import_identifier(import).as_str())).boxed(),
                    ),
                    args: Vec::new(),
                    type_args: None,
                })
                .boxed(),
            )
            .boxed(),
            is_async: false,
            is_generator: false,
            type_params: None,
            return_type: None,
        }),
    }
}

/// `key` bare when it is an identifier, quoted otherwise.
fn property_name(key: &str) -> PropName {
    let mut characters = key.chars();
    let is_identifier = characters
        .next()
        .is_some_and(|first| first == '_' || first == '$' || first.is_ascii_alphabetic())
        && characters.all(|rest| rest == '_' || rest == '$' || rest.is_ascii_alphanumeric());
    if is_identifier {
        PropName::Ident(IdentName::new(key.into(), DUMMY_SP))
    } else {
        PropName::Str(string(key))
    }
}

/// `ty` as an swc type, parenthesized when it is a union, intersection, function, or
/// conditional: the precedence positions an operand of those occupies.
fn operand(ty: &TypeScriptType, comment_at: CommentAt) -> Box<TsType> {
    match ty {
        TypeScriptType::Union(_)
        | TypeScriptType::Intersection(_)
        | TypeScriptType::Function(_)
        | TypeScriptType::Conditional(_) => TsType::TsParenthesizedType(TsParenthesizedType {
            span: DUMMY_SP,
            type_ann: ts_type(ty, comment_at),
        })
        .boxed(),
        TypeScriptType::Keyword(_)
        | TypeScriptType::Reference(_)
        | TypeScriptType::StringLiteral(_)
        | TypeScriptType::TemplateLiteral(_)
        | TypeScriptType::Infer(_)
        | TypeScriptType::TypeOf(_)
        | TypeScriptType::Object(_)
        | TypeScriptType::Parenthesized(_) => ts_type(ty, comment_at),
    }
}

fn ts_type(ty: &TypeScriptType, comment_at: CommentAt) -> Box<TsType> {
    match ty {
        TypeScriptType::Keyword(keyword) => TsType::TsKeywordType(TsKeywordType {
            span: DUMMY_SP,
            kind: keyword_kind(keyword.dereference()),
        }),
        TypeScriptType::Reference(reference) => TsType::TsTypeRef(TsTypeRef {
            span: DUMMY_SP,
            type_name: TsEntityName::Ident(ident(reference.name.as_str())),
            type_params: type_args(reference.args.as_slice(), comment_at),
        }),
        TypeScriptType::StringLiteral(value) => TsType::TsLitType(TsLitType {
            span: DUMMY_SP,
            lit: TsLit::Str(string(value.as_str())),
        }),
        TypeScriptType::TemplateLiteral(template) => TsType::TsLitType(TsLitType {
            span: DUMMY_SP,
            lit: TsLit::Tpl(TsTplLitType {
                span: DUMMY_SP,
                types: template
                    .spans
                    .iter()
                    .map(|span| ts_type(span.type_.reference(), comment_at))
                    .collect(),
                quasis: std::iter::once(template.head.as_str())
                    .chain(template.spans.iter().map(|span| span.text.as_str()))
                    .enumerate()
                    .map(|(index, text)| TplElement {
                        span: DUMMY_SP,
                        tail: index == template.spans.len(),
                        cooked: text.wrap_some().map(Into::into),
                        raw: template_raw(text).into(),
                    })
                    .collect(),
            }),
        }),
        TypeScriptType::Union(members) => {
            TsType::TsUnionOrIntersectionType(TsUnionOrIntersectionType::TsUnionType(TsUnionType {
                span: DUMMY_SP,
                types: members
                    .iter()
                    .map(|member| operand(member, comment_at))
                    .collect(),
            }))
        }
        TypeScriptType::Intersection(members) => TsType::TsUnionOrIntersectionType(
            TsUnionOrIntersectionType::TsIntersectionType(TsIntersectionType {
                span: DUMMY_SP,
                types: members
                    .iter()
                    .map(|member| operand(member, comment_at))
                    .collect(),
            }),
        ),
        TypeScriptType::Conditional(conditional) => TsType::TsConditionalType(TsConditionalType {
            span: DUMMY_SP,
            check_type: operand(conditional.check.reference(), comment_at),
            extends_type: operand(conditional.extends.reference(), comment_at),
            true_type: ts_type(conditional.true_type.reference(), comment_at),
            false_type: ts_type(conditional.false_type.reference(), comment_at),
        }),
        TypeScriptType::Infer(name) => TsType::TsInferType(TsInferType {
            span: DUMMY_SP,
            type_param: type_param(
                TypeParam {
                    name: name.clone(),
                    constraint: None,
                    default: None,
                }
                .reference(),
                comment_at,
            ),
        }),
        TypeScriptType::TypeOf(identifier) => TsType::TsTypeQuery(TsTypeQuery {
            span: DUMMY_SP,
            expr_name: TsTypeQueryExpr::TsEntityName(TsEntityName::Ident(ident(
                identifier.as_str(),
            ))),
            type_args: None,
        }),
        TypeScriptType::Object(properties) => TsType::TsTypeLit(TsTypeLit {
            span: DUMMY_SP,
            members: properties
                .iter()
                .map(|property| {
                    TsTypeElement::TsPropertySignature(TsPropertySignature {
                        span: property
                            .description
                            .as_ref()
                            .map_or(DUMMY_SP, |description| comment_at(description.as_str())),
                        readonly: true,
                        key: Expr::Ident(ident(property.key.as_str())).boxed(),
                        computed: false,
                        optional: match property.optionality {
                            Optionality::Required => false,
                            Optionality::Optional => true,
                        },
                        type_ann: type_annotation(property.type_.reference(), comment_at)
                            .wrap_some(),
                    })
                })
                .collect(),
        }),
        TypeScriptType::Parenthesized(inner) => TsType::TsParenthesizedType(TsParenthesizedType {
            span: DUMMY_SP,
            type_ann: ts_type(inner, comment_at),
        }),
        TypeScriptType::Function(function) => {
            TsType::TsFnOrConstructorType(TsFnOrConstructorType::TsFnType(TsFnType {
                span: DUMMY_SP,
                params: function
                    .params
                    .iter()
                    .map(|param| TsFnParam::Ident(binding(param, comment_at)))
                    .collect(),
                type_params: type_param_decl(function.type_params.as_slice(), comment_at),
                type_ann: type_annotation(function.return_type.reference(), comment_at),
            }))
        }
    }
    .boxed()
}

/// A template literal part's raw text: backslashes, backticks, and `${` escaped.
fn template_raw(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('`', "\\`")
        .replace("${", "\\${")
}

fn keyword_kind(keyword: TypeScriptKeyword) -> TsKeywordTypeKind {
    match keyword {
        TypeScriptKeyword::String => TsKeywordTypeKind::TsStringKeyword,
        TypeScriptKeyword::Number => TsKeywordTypeKind::TsNumberKeyword,
        TypeScriptKeyword::Boolean => TsKeywordTypeKind::TsBooleanKeyword,
        TypeScriptKeyword::Null => TsKeywordTypeKind::TsNullKeyword,
        TypeScriptKeyword::Any => TsKeywordTypeKind::TsAnyKeyword,
        TypeScriptKeyword::Unknown => TsKeywordTypeKind::TsUnknownKeyword,
        TypeScriptKeyword::Never => TsKeywordTypeKind::TsNeverKeyword,
        TypeScriptKeyword::Void => TsKeywordTypeKind::TsVoidKeyword,
        TypeScriptKeyword::Object => TsKeywordTypeKind::TsObjectKeyword,
    }
}

fn type_args(
    args: &[TypeScriptType],
    comment_at: CommentAt,
) -> Option<Box<TsTypeParamInstantiation>> {
    if args.is_empty() {
        return None;
    }
    TsTypeParamInstantiation {
        span: DUMMY_SP,
        params: args.iter().map(|arg| ts_type(arg, comment_at)).collect(),
    }
    .boxed()
    .wrap_some()
}

fn type_param_decl(params: &[TypeParam], comment_at: CommentAt) -> Option<Box<TsTypeParamDecl>> {
    if params.is_empty() {
        return None;
    }
    TsTypeParamDecl {
        span: DUMMY_SP,
        params: params
            .iter()
            .map(|param| type_param(param, comment_at))
            .collect(),
    }
    .boxed()
    .wrap_some()
}

fn type_param(param: &TypeParam, comment_at: CommentAt) -> TsTypeParam {
    TsTypeParam {
        span: DUMMY_SP,
        name: ident(param.name.as_str()),
        is_in: false,
        is_out: false,
        is_const: false,
        constraint: param
            .constraint
            .as_ref()
            .map(|constraint| ts_type(constraint, comment_at)),
        default: param
            .default
            .as_ref()
            .map(|default| ts_type(default, comment_at)),
    }
}

fn binding(param: &FunctionParam, comment_at: CommentAt) -> BindingIdent {
    BindingIdent {
        id: ident(param.name.as_str()),
        type_ann: type_annotation(param.type_.reference(), comment_at).wrap_some(),
    }
}

fn type_annotation(ty: &TypeScriptType, comment_at: CommentAt) -> Box<TsTypeAnn> {
    TsTypeAnn {
        span: DUMMY_SP,
        type_ann: ts_type(ty, comment_at),
    }
    .boxed()
}

fn ident(name: &str) -> Ident {
    Ident::new_no_ctxt(name.into(), DUMMY_SP)
}

fn string(value: &str) -> Str {
    Str {
        span: DUMMY_SP,
        value: value.into(),
        raw: None,
    }
}

#[cfg(test)]
mod tests {
    use isograph_compiler::ArtifactPath;
    use prelude::Postfix;

    use super::print;
    use crate::module::{
        ArtifactImport, ArtifactModule, ArtifactObjectEntry, ArtifactValue, DefaultExportType,
        NamedImport, Optionality, PropertySignature, TemplateLiteralSpan, TemplateLiteralType,
        TypeAlias, TypeScriptItem, TypeScriptKeyword, TypeScriptModule, TypeScriptType,
        module_of_artifact,
    };

    fn path(text: &str) -> ArtifactPath {
        ArtifactPath(text.to_owned())
    }

    fn entry(key: &str, value: ArtifactValue) -> ArtifactObjectEntry {
        ArtifactObjectEntry {
            key: key.to_owned(),
            value,
        }
    }

    fn printed(value: ArtifactValue) -> String {
        print(
            module_of_artifact(
                ArtifactModule {
                    path: path("Query/A/entrypoint.ts"),
                    default_export: value,
                    default_export_type: DefaultExportType::Inferred,
                }
                .reference(),
            )
            .reference(),
        )
    }

    fn printed_alias(aliased: TypeScriptType) -> String {
        print(
            TypeScriptModule {
                path: path("iso.ts"),
                items: TypeScriptItem::TypeAlias(TypeAlias {
                    name: "X".to_owned(),
                    type_params: Vec::new(),
                    aliased,
                })
                .wrap_vec(),
            }
            .reference(),
        )
    }

    fn import(text: &str) -> ArtifactValue {
        ArtifactValue::Import(ArtifactImport(path(text)))
    }

    #[test]
    fn each_distinct_import_is_declared_once_in_path_order() {
        let text = printed(ArtifactValue::Object(vec![
            entry("b", import("Query/A/query_text.ts")),
            entry("a", import("Query/A/normalization_ast.ts")),
            entry("again", import("Query/A/query_text.ts")),
        ]));
        let imports: Vec<&str> = text
            .lines()
            .filter(|line| line.starts_with("import "))
            .collect();
        assert_eq!(
            imports,
            [
                "import Query__A__normalization_ast from \"./normalization_ast\";",
                "import Query__A__query_text from \"./query_text\";",
            ]
        );
        assert!(text.contains("b: Query__A__query_text"), "{text}");
        assert!(text.contains("again: Query__A__query_text"), "{text}");
        assert!(text.contains("a: Query__A__normalization_ast"), "{text}");
    }

    #[test]
    fn a_named_import_prints_its_local_name_and_one_value_import() {
        let named = |imported: &str, local: &str| NamedImport {
            specifier: "x".to_owned(),
            imported: imported.to_owned(),
            local: local.to_owned(),
        };
        let renamed = print(
            TypeScriptModule {
                path: path("Query/A/resolver_reader.ts"),
                items: vec![
                    TypeScriptItem::NamedImport(named("a", "b")),
                    TypeScriptItem::ExportDefault(ArtifactValue::Object(
                        entry("value", ArtifactValue::NamedImport(named("a", "b"))).wrap_vec(),
                    )),
                ],
            }
            .reference(),
        );
        assert!(
            renamed.contains("import { a as b } from \"x\";"),
            "{renamed}"
        );
        assert!(renamed.contains("value: b"), "{renamed}");
        assert!(!renamed.contains("import type"), "{renamed}");
        let same = print(
            TypeScriptModule {
                path: path("Query/A/resolver_reader.ts"),
                items: TypeScriptItem::NamedImport(named("a", "a")).wrap_vec(),
            }
            .reference(),
        );
        assert!(same.contains("import { a } from \"x\";"), "{same}");
    }

    #[test]
    fn an_import_call_thunk_calls_the_import_only_when_called() {
        let text = printed(ArtifactValue::Object(
            entry(
                "x",
                ArtifactValue::ImportCallThunk(ArtifactImport(path("Query/A/resolver_reader.ts"))),
            )
            .wrap_vec(),
        ));
        assert!(
            text.contains("x: ()=>Query__A__resolver_reader()"),
            "{text}"
        );
        assert!(
            text.contains("import Query__A__resolver_reader from \"./resolver_reader\";"),
            "{text}"
        );
    }

    #[test]
    fn printing_a_module_twice_is_the_same_text() {
        let value = || {
            ArtifactValue::Object(vec![
                entry("x", import("Pet/B/y.ts")),
                entry("y", ArtifactValue::Number(1.5)),
            ])
        };
        assert_eq!(printed(value()), printed(value()));
    }

    #[test]
    fn a_string_with_quotes_backslashes_newlines_and_unicode_is_escaped() {
        let text = printed(ArtifactValue::String("a\"b'c\\d\ne é".to_owned()));
        let literal = text
            .strip_prefix("export default ")
            .and_then(|rest| rest.strip_suffix(";\n"))
            .expect("the module is one default export");
        assert!(
            literal.starts_with('"') && literal.ends_with('"'),
            "{literal}"
        );
        assert!(literal.contains("\\\""), "{literal}");
        assert!(literal.contains("\\\\"), "{literal}");
        assert!(literal.contains("\\n"), "{literal}");
        assert!(!literal.contains('\n'), "{literal}");
        assert!(literal.contains('é'), "{literal}");
    }

    #[test]
    fn nested_arrays_and_objects_print_nested_with_scalars() {
        let text = printed(ArtifactValue::Array(vec![
            ArtifactValue::Null,
            ArtifactValue::Boolean(true),
            ArtifactValue::Number(2.0),
            ArtifactValue::Array(ArtifactValue::Boolean(false).wrap_vec()),
            ArtifactValue::Object(entry("not-an-identifier", ArtifactValue::Null).wrap_vec()),
        ]));
        let flat: String = text.split_whitespace().collect();
        assert_eq!(
            flat,
            "exportdefault[null,true,2,[false],{\"not-an-identifier\":null}];"
        );
    }

    #[test]
    fn a_union_inside_an_intersection_is_parenthesized() {
        let text = printed_alias(TypeScriptType::Intersection(vec![
            TypeScriptType::reference("A"),
            TypeScriptType::Union(vec![
                TypeScriptType::reference("B"),
                TypeScriptType::reference("C"),
            ]),
        ]));
        assert_eq!(text, "type X = A & (B | C);\n");
    }

    #[test]
    fn template_literal_text_escapes_backticks_backslashes_and_substitutions() {
        let text = printed_alias(TypeScriptType::TemplateLiteral(TemplateLiteralType {
            head: "`\\${".to_owned(),
            spans: TemplateLiteralSpan {
                type_: TypeScriptType::reference("T"),
                text: "end".to_owned(),
            }
            .wrap_vec(),
        }));
        assert_eq!(text, "type X = `\\`\\\\\\${${T}end`;\n");
    }

    fn property(key: &str, type_: TypeScriptType, description: Option<&str>) -> PropertySignature {
        PropertySignature {
            key: key.to_owned(),
            optionality: Optionality::Required,
            type_,
            description: description.map(str::to_owned),
        }
    }

    fn described_string(description: &str) -> TypeScriptType {
        TypeScriptType::Object(
            property(
                "a",
                TypeScriptType::Keyword(TypeScriptKeyword::String),
                description.wrap_some(),
            )
            .wrap_vec(),
        )
    }

    #[test]
    fn a_description_at_depth_one_is_a_jsdoc_comment_on_its_own_lines() {
        let text = printed_alias(described_string("Replace a pet's tagline."));
        assert_eq!(
            text,
            "type X = {\n  /**\n   * Replace a pet's tagline.\n   */\n  readonly a: string;\n};\n"
        );
    }

    #[test]
    fn a_description_at_depth_three_is_indented_to_its_property() {
        let nest =
            |inner: TypeScriptType| TypeScriptType::Object(property("o", inner, None).wrap_vec());
        let text = printed_alias(nest(nest(described_string("Deep."))));
        assert_eq!(
            text,
            concat!(
                "type X = {\n",
                "  readonly o: {\n",
                "    readonly o: {\n",
                "      /**\n",
                "       * Deep.\n",
                "       */\n",
                "      readonly a: string;\n",
                "    };\n",
                "  };\n",
                "};\n",
            )
        );
    }

    #[test]
    fn each_line_of_a_multi_line_description_is_prefixed() {
        let text = printed_alias(described_string("First.\n\nThird."));
        assert_eq!(
            text,
            "type X = {\n  /**\n   * First.\n   *\n   * Third.\n   */\n  readonly a: string;\n};\n"
        );
    }

    #[test]
    fn an_empty_description_is_one_empty_comment_line() {
        let text = printed_alias(described_string(""));
        assert_eq!(
            text,
            "type X = {\n  /**\n   *\n   */\n  readonly a: string;\n};\n"
        );
    }

    #[test]
    fn a_comment_terminator_in_a_description_is_escaped() {
        let text = printed_alias(described_string("a */ b"));
        assert_eq!(
            text,
            "type X = {\n  /**\n   * a *\\/ b\n   */\n  readonly a: string;\n};\n"
        );
    }
}
