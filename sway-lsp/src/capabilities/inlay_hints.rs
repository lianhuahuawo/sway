use crate::{
    core::{session::Session, token::TypedAstToken},
    utils::common::get_range_from_span,
};
use sway_core::{language::ty::TyDeclaration, type_system::TypeInfo};
use sway_types::Spanned;
use tower_lsp::lsp_types::{self, Range, Url};

// Future PR's will add more kinds
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InlayKind {
    TypeHint,
}

#[derive(Debug)]
pub struct InlayHint {
    pub range: Range,
    pub kind: InlayKind,
    pub label: String,
}

pub(crate) fn inlay_hints(
    session: &Session,
    uri: &Url,
    range: &Range,
) -> Option<Vec<lsp_types::InlayHint>> {
    // 1. Loop through all our tokens and filter out all tokens that aren't TypedVariableDeclaration tokens
    // 2. Also filter out all tokens that have a span that fall outside of the provided range
    // 3. Filter out all variable tokens that have a type_ascription
    // 4. Look up the type id for the remaining tokens
    // 5. Convert the type into a string
    let config = &session.config.read().inlay_hints;
    if !config.type_hints {
        return None;
    }

    let hints: Vec<lsp_types::InlayHint> = session
        .tokens_for_file(uri)
        .iter()
        .filter_map(|item| {
            let token = item.value();
            token.typed.as_ref().and_then(|t| match t {
                TypedAstToken::TypedDeclaration(TyDeclaration::VariableDeclaration(var_decl)) => {
                    match var_decl.type_ascription_span {
                        Some(_) => None,
                        None => {
                            let var_range = get_range_from_span(&var_decl.name.span());
                            if var_range.start >= range.start && var_range.end <= range.end {
                                Some(var_decl.clone())
                            } else {
                                None
                            }
                        }
                    }
                }
                _ => None,
            })
        })
        .filter_map(|var| {
            let type_info = sway_core::type_system::look_up_type_id(var.type_ascription);
            match type_info {
                TypeInfo::Unknown | TypeInfo::UnknownGeneric { .. } => None,
                _ => Some(var),
            }
        })
        .map(|var| {
            let range = crate::utils::common::get_range_from_span(&var.name.span());
            let kind = InlayKind::TypeHint;
            let label = format!("{}", var.type_ascription);
            let inlay_hint = InlayHint { range, kind, label };
            self::inlay_hint(config.render_colons, inlay_hint)
        })
        .collect();

    Some(hints)
}

pub(crate) fn inlay_hint(render_colons: bool, inlay_hint: InlayHint) -> lsp_types::InlayHint {
    lsp_types::InlayHint {
        position: match inlay_hint.kind {
            // after annotated thing
            InlayKind::TypeHint => inlay_hint.range.end,
        },
        label: lsp_types::InlayHintLabel::String(match inlay_hint.kind {
            InlayKind::TypeHint if render_colons => format!(": {}", inlay_hint.label),
            _ => inlay_hint.label,
        }),
        kind: match inlay_hint.kind {
            InlayKind::TypeHint => Some(lsp_types::InlayHintKind::TYPE),
        },
        tooltip: None,
        padding_left: Some(match inlay_hint.kind {
            InlayKind::TypeHint => !render_colons,
        }),
        padding_right: Some(match inlay_hint.kind {
            InlayKind::TypeHint => false,
        }),
        text_edits: None,
        data: None,
    }
}
