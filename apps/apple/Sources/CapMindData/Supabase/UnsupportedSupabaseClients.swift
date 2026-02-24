import Foundation
import CapMindCore

public struct UnsupportedSupabaseAuthClient: SupabaseAuthClientProtocol {
    public init() {}

    public func signIn(email: String, password: String) async throws -> SupabaseAuthSessionPayload {
        throw CapMindError.unsupported(message: "Supabase Auth client is not configured")
    }

    public func signUp(email: String, password: String, redirectURL: URL?) async throws -> SupabaseAuthSessionPayload? {
        throw CapMindError.unsupported(message: "Supabase Auth client is not configured")
    }

    public func restoreSession() async throws -> SupabaseAuthSessionPayload? {
        nil
    }

    public func signOut() async throws {}
}

public struct UnsupportedSupabaseMemoClient: SupabaseMemoClientProtocol {
    public init() {}

    public func listMemos(page: Int, pageSize: Int, isTrash: Bool) async throws -> [MemoEntity] {
        throw CapMindError.unsupported(message: "Supabase Memo client is not configured")
    }

    public func searchMemos(query: String, limit: Int) async throws -> [MemoEntity] {
        throw CapMindError.unsupported(message: "Supabase Memo client is not configured")
    }

    public func createMemo(userID: String, text: String, imagePaths: [String], createdAt: Date, updatedAt: Date, clientID: String?) async throws -> MemoEntity {
        throw CapMindError.unsupported(message: "Supabase Memo client is not configured")
    }

    public func updateMemo(id: String, userID: String, text: String, expectedVersion: String, imagePaths: [String]?) async throws -> MemoEntity {
        throw CapMindError.unsupported(message: "Supabase Memo client is not configured")
    }

    public func deleteMemo(id: String, userID: String, expectedVersion: String, deletedAt: Date) async throws -> MemoEntity? {
        throw CapMindError.unsupported(message: "Supabase Memo client is not configured")
    }

    public func restoreMemo(id: String, userID: String, expectedVersion: String, restoredAt: Date) async throws -> MemoEntity? {
        throw CapMindError.unsupported(message: "Supabase Memo client is not configured")
    }

    public func fetchMemo(id: String, userID: String) async throws -> MemoEntity? {
        throw CapMindError.unsupported(message: "Supabase Memo client is not configured")
    }
}

public struct UnsupportedSupabaseStorageClient: SupabaseStorageClientProtocol {
    public init() {}

    public func uploadImages(userID: String, localReferences: [String]) async throws -> [String] {
        throw CapMindError.unsupported(message: "Supabase Storage client is not configured")
    }
}
