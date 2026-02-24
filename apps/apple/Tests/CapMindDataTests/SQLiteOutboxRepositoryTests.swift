#if canImport(XCTest) && canImport(SQLite3)
import Foundation
import XCTest
@testable import CapMindCore
@testable import CapMindData

final class SQLiteOutboxRepositoryTests: XCTestCase {
    func testPersistsItemsAcrossInstances() async throws {
        let tempDir = FileManager.default.temporaryDirectory
            .appendingPathComponent("capmind-sqlite-tests-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: tempDir, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: tempDir) }

        let dbURL = tempDir.appendingPathComponent("outbox.sqlite3")

        let first = try SQLiteOutboxRepository(fileURL: dbURL)
        _ = try await first.enqueue(
            OutboxDraft(
                type: .create,
                clientID: "local-1",
                text: "hello",
                localImageReferences: ["/tmp/a.png"],
                expectedVersion: "0",
                createdAt: Date(),
                updatedAt: Date()
            )
        )

        let second = try SQLiteOutboxRepository(fileURL: dbURL)
        let items = try await second.listOrdered()

        XCTAssertEqual(items.count, 1)
        XCTAssertEqual(items[0].type, .create)
        XCTAssertEqual(items[0].clientID, "local-1")
        XCTAssertEqual(items[0].text, "hello")
        XCTAssertEqual(items[0].localImageReferences, ["/tmp/a.png"])
    }

    func testUpdatePendingCreateReturnsTrueWhenRowExists() async throws {
        let tempDir = FileManager.default.temporaryDirectory
            .appendingPathComponent("capmind-sqlite-tests-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: tempDir, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: tempDir) }

        let dbURL = tempDir.appendingPathComponent("outbox.sqlite3")
        let repository = try SQLiteOutboxRepository(fileURL: dbURL)

        _ = try await repository.enqueue(
            OutboxDraft(
                type: .create,
                clientID: "local-1",
                text: "before",
                expectedVersion: "0",
                createdAt: Date(),
                updatedAt: Date()
            )
        )

        let updated = try await repository.updatePendingCreate(
            clientID: "local-1",
            text: "after",
            updatedAt: Date()
        )

        XCTAssertTrue(updated)
        let items = try await repository.listOrdered()
        XCTAssertEqual(items.first?.text, "after")
    }
}
#endif
