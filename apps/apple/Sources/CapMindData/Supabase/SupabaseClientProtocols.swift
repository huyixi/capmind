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

public enum MemoResolveRPCStatus: String, Codable, Sendable {
    case updated
    case deleted
    case restored
    case conflict
    case notFound = "not_found"
}

public struct MemoRPCImagePayload: Decodable, Equatable, Sendable {
    public let url: String
    public let sortOrder: Int?

    enum CodingKeys: String, CodingKey {
        case url
        case sortOrder = "sort_order"
    }
}

public struct MemoRPCPayload: Decodable, Equatable, Sendable {
    public let id: String
    public let userID: String
    public let text: String
    public let createdAt: Date
    public let updatedAt: Date
    public let version: String
    public let deletedAt: Date?
    public let memoImages: [MemoRPCImagePayload]?

    enum CodingKeys: String, CodingKey {
        case id
        case userID = "user_id"
        case text
        case createdAt = "created_at"
        case updatedAt = "updated_at"
        case version
        case deletedAt = "deleted_at"
        case memoImages = "memo_images"
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        id = try container.decode(String.self, forKey: .id)
        userID = try container.decode(String.self, forKey: .userID)
        text = try container.decode(String.self, forKey: .text)
        createdAt = try container.decode(Date.self, forKey: .createdAt)
        updatedAt = try container.decode(Date.self, forKey: .updatedAt)

        if let versionString = try? container.decode(String.self, forKey: .version) {
            version = versionString
        } else if let versionInt = try? container.decode(Int.self, forKey: .version) {
            version = String(versionInt)
        } else if let versionDouble = try? container.decode(Double.self, forKey: .version) {
            version = String(Int(versionDouble))
        } else {
            version = ""
        }

        deletedAt = try container.decodeIfPresent(Date.self, forKey: .deletedAt)
        memoImages = try container.decodeIfPresent([MemoRPCImagePayload].self, forKey: .memoImages)
    }
}

public struct MemoResolveRPCPayload: Decodable, Equatable, Sendable {
    public let status: MemoResolveRPCStatus
    public let memoID: String?
    public let memo: MemoRPCPayload?
    public let serverMemo: MemoRPCPayload?
    public let forkedMemo: MemoRPCPayload?

    enum CodingKeys: String, CodingKey {
        case status
        case memoID = "memo_id"
        case memo
        case serverMemo = "server_memo"
        case forkedMemo = "forked_memo"
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
