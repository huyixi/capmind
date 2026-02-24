import Foundation

public enum ConflictType: String, Codable, Sendable {
    case update
    case delete
    case restore
}

public struct MemoEntity: Identifiable, Codable, Equatable, Sendable {
    public var id: String
    public var clientID: String?
    public var userID: String
    public var text: String
    public var images: [String]
    public var createdAt: Date
    public var updatedAt: Date
    public var version: String
    public var deletedAt: Date?
    public var serverVersion: String?
    public var conflictType: ConflictType?

    public init(
        id: String,
        clientID: String? = nil,
        userID: String,
        text: String,
        images: [String],
        createdAt: Date,
        updatedAt: Date,
        version: String,
        deletedAt: Date? = nil,
        serverVersion: String? = nil,
        conflictType: ConflictType? = nil
    ) {
        self.id = id
        self.clientID = clientID
        self.userID = userID
        self.text = text
        self.images = images
        self.createdAt = createdAt
        self.updatedAt = updatedAt
        self.version = version
        self.deletedAt = deletedAt
        self.serverVersion = serverVersion
        self.conflictType = conflictType
    }
}
