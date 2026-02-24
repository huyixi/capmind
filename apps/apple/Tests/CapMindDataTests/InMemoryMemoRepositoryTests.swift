#if canImport(XCTest)
import Foundation
import XCTest
@testable import CapMindCore
@testable import CapMindData

final class InMemoryMemoRepositoryTests: XCTestCase {
    func testUpdateThrowsConflictWhenVersionMismatch() async throws {
        let repository = InMemoryMemoRepository()
        let now = Date()
        let created = try await repository.createMemo(
            userID: "u1",
            text: "hello",
            imagePaths: [],
            createdAt: now,
            updatedAt: now,
            clientID: nil
        )

        do {
            _ = try await repository.updateMemo(
                id: created.id,
                userID: "u1",
                text: "changed",
                expectedVersion: "99",
                imagePaths: []
            )
            XCTFail("Expected conflict")
        } catch let error as CapMindError {
            guard case .conflict = error else {
                return XCTFail("Expected conflict error")
            }
        }
    }

    func testDeleteReturnsNilOnVersionMismatch() async throws {
        let repository = InMemoryMemoRepository()
        let now = Date()
        let created = try await repository.createMemo(
            userID: "u1",
            text: "hello",
            imagePaths: [],
            createdAt: now,
            updatedAt: now,
            clientID: nil
        )

        let deleted = try await repository.deleteMemo(
            id: created.id,
            userID: "u1",
            expectedVersion: "99",
            deletedAt: Date()
        )

        XCTAssertNil(deleted)
    }
}
#endif
