import Foundation

public enum ConflictType: String, Codable, Sendable {
    case update
    case delete
    case restore
}

public struct ConflictServerMemoSnapshot: Codable, Equatable, Sendable {
    public var id: String
    public var text: String
    public var images: [String]
    public var updatedAt: Date
    public var version: String
    public var deletedAt: Date?

    public init(
        id: String,
        text: String,
        images: [String],
        updatedAt: Date,
        version: String,
        deletedAt: Date?
    ) {
        self.id = id
        self.text = text
        self.images = images
        self.updatedAt = updatedAt
        self.version = version
        self.deletedAt = deletedAt
    }

    public init(memo: MemoEntity) {
        self.id = memo.id
        self.text = memo.text
        self.images = memo.images
        self.updatedAt = memo.updatedAt
        self.version = memo.version
        self.deletedAt = memo.deletedAt
    }
}

public struct MemoEntity: Identifiable, Codable, Equatable, Sendable {
    public var id: String
    public var clientID: String?
    public var userID: String
    public var text: String
    public var images: [String]
    public var hasImages: Bool
    public var imageCount: Int
    public var createdAt: Date
    public var updatedAt: Date
    public var version: String
    public var deletedAt: Date?
    public var serverVersion: String?
    public var conflictType: ConflictType?
    public var conflictServerMemo: ConflictServerMemoSnapshot?

    public init(
        id: String,
        clientID: String? = nil,
        userID: String,
        text: String,
        images: [String],
        hasImages: Bool? = nil,
        imageCount: Int? = nil,
        createdAt: Date,
        updatedAt: Date,
        version: String,
        deletedAt: Date? = nil,
        serverVersion: String? = nil,
        conflictType: ConflictType? = nil,
        conflictServerMemo: ConflictServerMemoSnapshot? = nil
    ) {
        self.id = id
        self.clientID = clientID
        self.userID = userID
        self.text = text
        self.images = images
        self.hasImages = hasImages ?? !images.isEmpty
        self.imageCount = imageCount ?? images.count
        self.createdAt = createdAt
        self.updatedAt = updatedAt
        self.version = version
        self.deletedAt = deletedAt
        self.serverVersion = serverVersion
        self.conflictType = conflictType
        self.conflictServerMemo = conflictServerMemo
    }
}
