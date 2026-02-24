import Foundation
import CapMindCore

public struct SupabaseAuthSessionPayload: Equatable, Sendable {
    public var userID: String
    public var email: String
    public var accessToken: String
    public var refreshToken: String?
    public var expiresAt: Date?

    public init(
        userID: String,
        email: String,
        accessToken: String,
        refreshToken: String?,
        expiresAt: Date?
    ) {
        self.userID = userID
        self.email = email
        self.accessToken = accessToken
        self.refreshToken = refreshToken
        self.expiresAt = expiresAt
    }
}

public protocol SupabaseAuthClientProtocol: Sendable {
    func signIn(email: String, password: String) async throws -> SupabaseAuthSessionPayload
    func signUp(email: String, password: String, redirectURL: URL?) async throws -> SupabaseAuthSessionPayload?
    func restoreSession() async throws -> SupabaseAuthSessionPayload?
    func signOut() async throws
}

public protocol SupabaseMemoClientProtocol: Sendable {
    func listMemos(page: Int, pageSize: Int, isTrash: Bool) async throws -> [MemoEntity]
    func searchMemos(query: String, limit: Int) async throws -> [MemoEntity]
    func createMemo(userID: String, text: String, imagePaths: [String], createdAt: Date, updatedAt: Date, clientID: String?) async throws -> MemoEntity
    func updateMemo(id: String, userID: String, text: String, expectedVersion: String, imagePaths: [String]?) async throws -> MemoEntity
    func deleteMemo(id: String, userID: String, expectedVersion: String, deletedAt: Date) async throws -> MemoEntity?
    func restoreMemo(id: String, userID: String, expectedVersion: String, restoredAt: Date) async throws -> MemoEntity?
    func fetchMemo(id: String, userID: String) async throws -> MemoEntity?
}

public protocol SupabaseStorageClientProtocol: Sendable {
    func uploadImages(userID: String, localReferences: [String]) async throws -> [String]
}
