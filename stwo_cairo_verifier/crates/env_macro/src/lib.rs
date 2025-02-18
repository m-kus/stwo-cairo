use cairo_lang_filesystem::ids::{FileKind, FileLongId, VirtualFile};
use cairo_lang_macro::{inline_macro, Diagnostic, ProcMacroResult, TokenStream};
use cairo_lang_parser::db::ParserGroup;
use cairo_lang_parser::utils::SimpleParserDatabase;
use cairo_lang_syntax::node::ast::{ArgClause, Expr, ExprInlineMacro, WrappedArgList};
use cairo_lang_utils::{Intern, Upcast};

/// Returns the value of an environment variable.
///
/// For example:
/// ```
/// let max_log_size = env!("MAX_LOG_SIZE");
/// ```
#[inline_macro]
pub fn env(token_stream: TokenStream) -> ProcMacroResult {
    let db = SimpleParserDatabase::default();
    // Get the ExprInlineMacro object so we can use the helper functions.
    let mac = parse_inline_macro(token_stream, &db);
    // Get the arguments of the macro. This macro expects a tuple as argument so we get the WrappedArgList::ParenthesizedArgList
    let macro_args = if let WrappedArgList::ParenthesizedArgList(args) = mac.arguments(db.upcast()) {
        args.arguments(db.upcast()).elements(db.upcast())
    } else {
        vec![]
    };

    if macro_args.len() != 1 {
        return ProcMacroResult::new(TokenStream::empty())
            .with_diagnostics(Diagnostic::error("Invalid number of arguments").into());
    }
    let env_variable_name = match get_arg_value(db.upcast(), &macro_args[0].arg_clause(db.upcast())) {
        Some(val) => val,
        None => {
            return ProcMacroResult::new(TokenStream::empty())
                .with_diagnostics(Diagnostic::error("Invalid environment variable name").into())
        }
    };

    let env_variable_value = match std::env::var(env_variable_name) {
        Ok(val) => val,
        Err(_) => {
            return ProcMacroResult::new(TokenStream::empty())
                .with_diagnostics(Diagnostic::error("Environment variable not set").into())
        }
    };

    ProcMacroResult::new(TokenStream::new(env_variable_value))
}

/// Return an [`ExprInlineMacro`] from the text received. The expected text is the macro arguments.
/// For example the initial macro text was `pow!(10, 3)`, the text in the token stream is only `(10, 3)`
fn parse_inline_macro(token_stream: impl ToString, db: &SimpleParserDatabase) -> ExprInlineMacro {
    // Create a virtual file that will be parsed.
    let file = FileLongId::Virtual(VirtualFile {
        parent: None,
        name: "parser_input".into(),
        content: format!("pow!{}", token_stream.to_string()).into(), // easiest workaround after change
        code_mappings: [].into(),
        kind: FileKind::Expr, // this part is different than db.parse_virtual
    })
    .intern(db);

    // Could fail if there was a parsing error but it shouldn't happen as the file has already
    // been parsed once to reach this macro.
    let node = db.file_expr_syntax(file).unwrap();

    let Expr::InlineMacro(inline_macro) = node else {
        panic!() // should not happen
    };

    inline_macro
}

/// Returns the value of a literal argument.
fn get_arg_value(db: &SimpleParserDatabase, arg_clause: &ArgClause) -> Option<String> {
    let base_expr = match arg_clause {
        ArgClause::Unnamed(arg_clause) => arg_clause.value(db.upcast()),
        _ => return None,
    };
    if let Expr::String(base_lit) = base_expr {
        base_lit.string_value(db.upcast())
    } else {
        None
    }
}
