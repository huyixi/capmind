import Foundation

public protocol MemoRepository: Sendable {
    func listMemos(page: Int, pageSize: Int, isTrash: Bool) async throws -> [MemoEntity]
    func searchMemos(query: String, limit: Int) async throws -> [MemoEntity]

    func createMemo(
        userID: String,
        text: String,
        imagePaths: [String],
        createdAt: Date,
        updatedAt: Date,
        clientID: String?
    ) async throws -> MemoEntity

    func updateMemo(
        id: String,
        userID: String,
        text: String,
        expectedVersion: String,
        imagePaths: [String]?
    ) async throws -> MemoEntity

    func deleteMemo(
        id: String,
        userID: String,
        expectedVersion: String,
        deletedAt: Date
    ) async throws -> MemoEntity?

    func restoreMemo(
        id: String,
        userID: String,
        expectedVersion: String,
        restoredAt: Date
    ) async throws -> MemoEntity?

    func fetchMemo(id: String, userID: String) async throws -> MemoEntity?
}
