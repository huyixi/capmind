#if canImport(XCTest)
import XCTest
@testable import CapMindCore

final class MemoVersionTests: XCTestCase {
    func testNormalizeExpectedRejectsInvalidValue() {
        XCTAssertEqual(MemoVersion.normalizeExpected(nil), "0")
        XCTAssertEqual(MemoVersion.normalizeExpected("abc"), "0")
        XCTAssertEqual(MemoVersion.normalizeExpected("12"), "12")
    }

    func testNextVersionIncrements() {
        XCTAssertEqual(MemoVersion.next("0"), "1")
        XCTAssertEqual(MemoVersion.next("41"), "42")
    }
}
#endif
