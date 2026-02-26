#if canImport(XCTest)
import Foundation
import XCTest
@testable import CapMindCore

final class FlushOutboxUseCaseTests: XCTestCase {
    func testRunProcessesCreateAndUpdate() async throws {
        let outbox = MockOutboxRepository()
        let memo = MockMemoRepository()
        let image = MockImageRepository()

        _ = try await outbox.enqueue(
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

        _ = try await outbox.enqueue(
            OutboxDraft(
                type: .update,
                memoID: "memo-1",
                text: "world",
                expectedVersion: "2",
                createdAt: Date(),
                updatedAt: Date()
            )
        )

        let useCase = FlushOutboxUseCase(
            memoRepository: memo,
            outboxRepository: outbox,
            imageRepository: image
        )

        let result = await useCase.run(userID: "user-1")

        XCTAssertTrue(result.didSync)
        XCTAssertFalse(result.hadError)
        XCTAssertEqual(result.processedCount, 2)
        XCTAssertEqual(result.conflictCount, 0)

        let createCalls = await memo.createCalls
        XCTAssertEqual(createCalls.count, 1)
        XCTAssertEqual(createCalls.first?.text, "hello")
        XCTAssertEqual(createCalls.first?.images.first, "uploaded://user-1/0")

        let remaining = try await outbox.listOrdered()
        XCTAssertTrue(remaining.isEmpty)
    }

    func testRunRemovesConflictUpdate() async throws {
        let outbox = MockOutboxRepository()
        let memo = MockMemoRepository()
        await memo.setConflictOnUpdate(true)

        _ = try await outbox.enqueue(
            OutboxDraft(
                type: .update,
                memoID: "memo-1",
                text: "world",
                expectedVersion: "2",
                createdAt: Date(),
                updatedAt: Date()
            )
        )

        let useCase = FlushOutboxUseCase(
            memoRepository: memo,
            outboxRepository: outbox,
            imageRepository: MockImageRepository()
        )

        let result = await useCase.run(userID: "user-1")

        XCTAssertTrue(result.didSync)
        XCTAssertFalse(result.hadError)
        XCTAssertEqual(result.conflictCount, 1)

        let remaining = try await outbox.listOrdered()
        XCTAssertTrue(remaining.isEmpty)

        let createCalls = await memo.createCalls
        XCTAssertEqual(createCalls.count, 1)
        XCTAssertEqual(createCalls.first?.text, "world")
    }

    func testRunUpdateMergesRemoteAndUploadedImagePaths() async throws {
        let outbox = MockOutboxRepository()
        let memo = MockMemoRepository()
        let image = MockImageRepository()

        _ = try await outbox.enqueue(
            OutboxDraft(
                type: .update,
                memoID: "memo-1",
                text: "new value",
                localImageReferences: ["/tmp/local-a.png"],
                imagePaths: ["public/memo-images/u1/existing-a.png"],
                expectedVersion: "1",
                createdAt: Date(),
                updatedAt: Date()
            )
        )

        let useCase = FlushOutboxUseCase(
            memoRepository: memo,
            outboxRepository: outbox,
            imageRepository: image
        )

        let result = await useCase.run(userID: "user-1")

        XCTAssertTrue(result.didSync)
        XCTAssertFalse(result.hadError)
        XCTAssertEqual(result.conflictCount, 0)
        XCTAssertEqual(result.processedCount, 1)

        let updateCalls = await memo.updateCalls
        XCTAssertEqual(updateCalls.count, 1)
        XCTAssertEqual(
            updateCalls.first?.imagePaths ?? [],
            ["public/memo-images/u1/existing-a.png", "uploaded://user-1/0"]
        )
    }
}

private actor MockOutboxRepository: OutboxRepository {
    private var items: [OutboxItem] = []
    private var nextSequence: Int64 = 1

    func enqueue(_ draft: OutboxDraft) async throws -> OutboxItem {
        let item = OutboxItem(
            sequence: nextSequence,
            type: draft.type,
            memoID: draft.memoID,
            clientID: draft.clientID,
            text: draft.text,
            localImageReferences: draft.localImageReferences,
            imagePaths: draft.imagePaths,
            expectedVersion: draft.expectedVersion,
            createdAt: draft.createdAt,
            updatedAt: draft.updatedAt
        )
        nextSequence += 1
        items.append(item)
        return item
    }

    func listOrdered() async throws -> [OutboxItem] {
        items.sorted { $0.sequence < $1.sequence }
    }

    func remove(id: UUID) async throws {
        items.removeAll { $0.id == id }
    }

    func removePendingCreate(clientID: String) async throws {
        items.removeAll { $0.type == .create && $0.clientID == clientID }
    }

    func updatePendingCreate(clientID: String, text: String, updatedAt: Date) async throws -> Bool {
        guard let index = items.firstIndex(where: { $0.type == .create && $0.clientID == clientID }) else {
            return false
        }
        items[index].text = text
        items[index].updatedAt = updatedAt
        return true
    }
}

private actor MockMemoRepository: MemoRepository {
    struct CreateCall {
        let userID: String
        let text: String
        let images: [String]
    }

    struct UpdateCall {
        let id: String
        let userID: String
        let text: String
        let imagePaths: [String]
    }

    var createCalls: [CreateCall] = []
    var updateCalls: [UpdateCall] = []
    private var conflictOnUpdate = false

    func setConflictOnUpdate(_ value: Bool) {
        conflictOnUpdate = value
    }

    func listMemos(page: Int, pageSize: Int, isTrash: Bool) async throws -> [MemoEntity] {
        []
    }

    func searchMemos(query: String, limit: Int) async throws -> [MemoEntity] {
        []
    }

    func createMemo(userID: String, text: String, imagePaths: [String], createdAt: Date, updatedAt: Date, clientID: String?) async throws -> MemoEntity {
        createCalls.append(CreateCall(userID: userID, text: text, images: imagePaths))
        return MemoEntity(
            id: UUID().uuidString,
            clientID: clientID,
            userID: userID,
            text: text,
            images: imagePaths,
            createdAt: createdAt,
            updatedAt: updatedAt,
            version: "1"
        )
    }

    func updateMemo(id: String, userID: String, text: String, expectedVersion: String, imagePaths: [String]?) async throws -> MemoEntity {
        updateCalls.append(
            UpdateCall(
                id: id,
                userID: userID,
                text: text,
                imagePaths: imagePaths ?? []
            )
        )
        if conflictOnUpdate {
            throw CapMindError.conflict(serverMemo: nil, forkedMemo: nil)
        }

        return MemoEntity(
            id: id,
            userID: userID,
            text: text,
            images: imagePaths ?? [],
            createdAt: Date(),
            updatedAt: Date(),
            version: MemoVersion.next(expectedVersion)
        )
    }

    func deleteMemo(id: String, userID: String, expectedVersion: String, deletedAt: Date) async throws -> MemoEntity? {
        nil
    }

    func restoreMemo(id: String, userID: String, expectedVersion: String, restoredAt: Date) async throws -> MemoEntity? {
        nil
    }

    func fetchMemo(id: String, userID: String) async throws -> MemoEntity? {
        nil
    }
}

private struct MockImageRepository: ImageRepository {
    func uploadImages(userID: String, localReferences: [String]) async throws -> [String] {
        localReferences.enumerated().map { index, _ in "uploaded://\(userID)/\(index)" }
    }
}
#endif
