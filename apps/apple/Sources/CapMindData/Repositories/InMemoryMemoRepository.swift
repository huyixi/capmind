import Foundation
import CapMindCore

public actor InMemoryMemoRepository: MemoRepository {
    private var memos: [String: MemoEntity] = [:]

    public init(seed: [MemoEntity] = []) {
        for memo in seed {
            memos[memo.id] = memo
        }
    }

    public func listMemos(page: Int, pageSize: Int, isTrash: Bool) async throws -> [MemoEntity] {
        let filtered = memos.values
            .filter { isTrash ? $0.deletedAt != nil : $0.deletedAt == nil }
            .sorted { $0.createdAt > $1.createdAt }

        let start = max(0, page * pageSize)
        guard start < filtered.count else { return [] }
        let end = min(filtered.count, start + pageSize)
        return Array(filtered[start..<end])
    }

    public func searchMemos(query: String, limit: Int) async throws -> [MemoEntity] {
        let normalized = query.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        guard !normalized.isEmpty else { return [] }

        return memos.values
            .filter { $0.deletedAt == nil }
            .filter { $0.text.lowercased().contains(normalized) }
            .sorted { $0.createdAt > $1.createdAt }
            .prefix(limit)
            .map { $0 }
    }

    public func createMemo(
        userID: String,
        text: String,
        imagePaths: [String],
        createdAt: Date,
        updatedAt: Date,
        clientID: String?
    ) async throws -> MemoEntity {
        let memo = MemoEntity(
            id: UUID().uuidString.lowercased(),
            clientID: clientID,
            userID: userID,
            text: text,
            images: imagePaths,
            createdAt: createdAt,
            updatedAt: updatedAt,
            version: "1",
            deletedAt: nil,
            serverVersion: "1",
            conflictType: nil
        )
        memos[memo.id] = memo
        return memo
    }

    public func updateMemo(
        id: String,
        userID: String,
        text: String,
        expectedVersion: String,
        imagePaths: [String]?
    ) async throws -> MemoEntity {
        guard var existing = memos[id], existing.userID == userID else {
            throw CapMindError.notFound
        }

        if existing.version != MemoVersion.normalizeExpected(expectedVersion) {
            throw CapMindError.conflict(serverMemo: existing)
        }

        existing.text = text
        if let imagePaths {
            existing.images = imagePaths
        }
        existing.updatedAt = Date()
        existing.version = MemoVersion.next(existing.version)
        existing.serverVersion = existing.version
        existing.conflictType = nil

        memos[id] = existing
        return existing
    }

    public func deleteMemo(
        id: String,
        userID: String,
        expectedVersion: String,
        deletedAt: Date
    ) async throws -> MemoEntity? {
        guard var existing = memos[id], existing.userID == userID else {
            throw CapMindError.notFound
        }

        if existing.version != MemoVersion.normalizeExpected(expectedVersion) {
            return nil
        }

        existing.deletedAt = deletedAt
        existing.updatedAt = deletedAt
        existing.version = MemoVersion.next(existing.version)
        existing.serverVersion = existing.version
        existing.conflictType = nil

        memos[id] = existing
        return existing
    }

    public func restoreMemo(
        id: String,
        userID: String,
        expectedVersion: String,
        restoredAt: Date
    ) async throws -> MemoEntity? {
        guard var existing = memos[id], existing.userID == userID else {
            throw CapMindError.notFound
        }

        if existing.version != MemoVersion.normalizeExpected(expectedVersion) {
            return nil
        }

        existing.deletedAt = nil
        existing.updatedAt = restoredAt
        existing.version = MemoVersion.next(existing.version)
        existing.serverVersion = existing.version
        existing.conflictType = nil

        memos[id] = existing
        return existing
    }

    public func fetchMemo(id: String, userID: String) async throws -> MemoEntity? {
        guard let memo = memos[id], memo.userID == userID else {
            return nil
        }
        return memo
    }
}
