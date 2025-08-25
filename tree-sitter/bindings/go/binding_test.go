package tree_sitter_isograph_test

import (
	"testing"

	tree_sitter "github.com/tree-sitter/go-tree-sitter"
	tree_sitter_isograph "github.com/isographlabs/isograph/bindings/go"
)

func TestCanLoadGrammar(t *testing.T) {
	language := tree_sitter.NewLanguage(tree_sitter_isograph.Language())
	if language == nil {
		t.Errorf("Error loading Isograph grammar")
	}
}
