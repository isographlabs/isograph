// a tree-sitter scanner that manually parses newline.
// this way we can treat the newline as a meaningful token when needed,
// while treating it as an extra for the most cases.

#include "tree_sitter/parser.h"

enum TokenType { NEWLINE };

static inline void advance(TSLexer *lexer) { lexer->advance(lexer, false); }

static inline void skip(TSLexer *lexer) { lexer->advance(lexer, true); }

bool tree_sitter_isograph_external_scanner_scan(void *payload, TSLexer *lexer,
                                                const bool *valid_symbols) {
  if (valid_symbols[NEWLINE]) {
    int32_t lookahead;
    while ((lookahead = lexer->lookahead)) {
      if (lookahead == '\n') {
        advance(lexer);
        lexer->result_symbol = NEWLINE;
        return true;
      }
      return false;
    }
  }
  return false;
}

// this scanner is so simple that we don't even need to have a stateful payload
// so these are all empty

unsigned tree_sitter_isograph_external_scanner_serialize(void *payload,
                                                         char *buffer) {
  return 0;
}

void tree_sitter_isograph_external_scanner_deserialize(void *payload,
                                                       const char *buffer,
                                                       unsigned length) {}

void *tree_sitter_isograph_external_scanner_create() { return NULL; }

void tree_sitter_isograph_external_scanner_destroy(void *payload) {}
