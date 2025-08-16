import XCTest
import SwiftTreeSitter
import TreeSitterIsograph

final class TreeSitterIsographTests: XCTestCase {
    func testCanLoadGrammar() throws {
        let parser = Parser()
        let language = Language(language: tree_sitter_isograph())
        XCTAssertNoThrow(try parser.setLanguage(language),
                         "Error loading Isograph grammar")
    }
}
