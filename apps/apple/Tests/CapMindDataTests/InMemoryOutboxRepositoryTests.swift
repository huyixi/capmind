#if canImport(XCTest)
import Foundation
import XCTest
@testable import CapMindCore
@testable import CapMindData

final class InMemoryOutboxRepositoryTests: XCTestCase {
    func testEnqueuePreservesSequenceOrder() async throws {
        let repository = InMemoryOutboxRepository()

        _ = try await repository.enqueue(
            OutboxDraft(
                type: .create,
                clientID: "local-1",
                text: "a",
                createdAt: Date(),
                updatedAt: Date()
            )
        )

        _ = try await repository.enqueue(
            OutboxDraft(
                type: .create,
                clientID: "local-2",
                text: "b",
                createdAt: Date(),
                updatedAt: Date()
            )
        )

        let items = try await repository.listOrdered()
        XCTAssertEqual(items.count, 2)
        XCTAssertLessThan(items[0].sequence, items[1].sequence)
    }

    func testUpdatePendingCreate() async throws {
        let repository = InMemoryOutboxRepository()

        _ = try await repository.enqueue(
            OutboxDraft(
                type: .create,
                clientID: "local-1",
                text: "a",
                createdAt: Date(),
                updatedAt: Date()
            )
        )

        let updated = try await repository.updatePendingCreate(
            clientID: "local-1",
            text: "updated",
            updatedAt: Date()
        )

        XCTAssertTrue(updated)
        let items = try await repository.listOrdered()
        XCTAssertEqual(items.first?.text, "updated")
    }
}
#endif
