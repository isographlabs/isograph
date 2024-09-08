use common_lang_types::Span;
use lsp_types::SemanticToken;

use crate::row_col_offset::{diff_to_end_of_slice, RowColDiff};

#[derive(Debug)]
pub(crate) enum SemanticTokenGeneratorState {
    InitialDiff(RowColDiff),
    LastSpan(Span),
}

#[derive(Debug)]
pub(crate) struct SemanticTokenGenerator<'a> {
    state: SemanticTokenGeneratorState,
    text: &'a str,
    tokens: Vec<SemanticToken>,
    final_diff: RowColDiff,
}

impl<'a> SemanticTokenGenerator<'a> {
    pub(crate) fn generate_semantic_token(&mut self, span: Span, token_type: u32) {
        let token = match self.state {
            SemanticTokenGeneratorState::InitialDiff(initial_diff) => {
                let new_diff = diff_to_end_of_slice(&self.text[0..(span.start as usize)]);
                let diff = initial_diff + new_diff;
                self.state = SemanticTokenGeneratorState::LastSpan(span);
                self.final_diff = self.final_diff + diff;
                SemanticToken {
                    delta_line: diff.delta_line(),
                    delta_start: diff.delta_start(),
                    length: span.len(),
                    token_type,
                    token_modifiers_bitset: 0,
                }
            }
            SemanticTokenGeneratorState::LastSpan(last_span) => {
                let diff = diff_to_end_of_slice(
                    &self.text[(last_span.start as usize)..(span.start as usize)],
                );
                self.final_diff = self.final_diff + diff;
                self.state = SemanticTokenGeneratorState::LastSpan(span);
                SemanticToken {
                    delta_line: diff.delta_line(),
                    delta_start: diff.delta_start(),
                    length: span.len(),
                    token_type,
                    token_modifiers_bitset: 0,
                }
            }
        };
        self.tokens.push(token);
    }

    pub(crate) fn new(text: &'a str, initial_diff: RowColDiff) -> Self {
        Self {
            state: SemanticTokenGeneratorState::InitialDiff(initial_diff),
            text,
            tokens: vec![],
            final_diff: RowColDiff::default(),
        }
    }

    pub(crate) fn consume(self) -> (Vec<SemanticToken>, RowColDiff) {
        (self.tokens, self.final_diff)
    }
}
