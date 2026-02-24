import Foundation
import CapMindCore

public struct SupabaseMemoRepository: MemoRepository {
    private let client: any SupabaseMemoClientProtocol

    public init(client: any SupabaseMemoClientProtocol) {
        self.client = client
    }

    public func listMemos(page: Int, pageSize: Int, isTrash: Bool) async throws -> [MemoEntity] {
        try await client.listMemos(page: page, pageSize: pageSize, isTrash: isTrash)
    }

    public func searchMemos(query: String, limit: Int) async throws -> [MemoEntity] {
        try await client.searchMemos(query: query, limit: limit)
    }

    public func createMemo(userID: String, text: String, imagePaths: [String], createdAt: Date, updatedAt: Date, clientID: String?) async throws -> MemoEntity {
        try await client.createMemo(
            userID: userID,
            text: text,
            imagePaths: imagePaths,
            createdAt: createdAt,
            updatedAt: updatedAt,
            clientID: clientID
        )
    }

    public func updateMemo(id: String, userID: String, text: String, expectedVersion: String, imagePaths: [String]?) async throws -> MemoEntity {
        try await client.updateMemo(
            id: id,
            userID: userID,
            text: text,
            expectedVersion: expectedVersion,
            imagePaths: imagePaths
        )
    }

    public func deleteMemo(id: String, userID: String, expectedVersion: String, deletedAt: Date) async throws -> MemoEntity? {
        try await client.deleteMemo(
            id: id,
            userID: userID,
            expectedVersion: expectedVersion,
            deletedAt: deletedAt
        )
    }

    public func restoreMemo(id: String, userID: String, expectedVersion: String, restoredAt: Date) async throws -> MemoEntity? {
        try await client.restoreMemo(
            id: id,
            userID: userID,
            expectedVersion: expectedVersion,
            restoredAt: restoredAt
        )
    }

    public func fetchMemo(id: String, userID: String) async throws -> MemoEntity? {
        try await client.fetchMemo(id: id, userID: userID)
    }
}
