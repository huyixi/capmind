import Foundation

public enum OutboxItemType: String, Codable, Sendable {
    case create
    case update
    case delete
    case restore
}

public struct OutboxItem: Identifiable, Codable, Equatable, Sendable {
    public var id: UUID
    public var sequence: Int64
    public var type: OutboxItemType
    public var memoID: String?
    public var clientID: String?
    public var text: String?
    public var localImageReferences: [String]
    public var imagePaths: [String]
    public var expectedVersion: String
    public var createdAt: Date
    public var updatedAt: Date

    public init(
        id: UUID = UUID(),
        sequence: Int64,
        type: OutboxItemType,
        memoID: String? = nil,
        clientID: String? = nil,
        text: String? = nil,
        localImageReferences: [String] = [],
        imagePaths: [String] = [],
        expectedVersion: String = "0",
        createdAt: Date,
        updatedAt: Date
    ) {
        self.id = id
        self.sequence = sequence
        self.type = type
        self.memoID = memoID
        self.clientID = clientID
        self.text = text
        self.localImageReferences = localImageReferences
        self.imagePaths = imagePaths
        self.expectedVersion = expectedVersion
        self.createdAt = createdAt
        self.updatedAt = updatedAt
    }
}

public struct OutboxDraft: Equatable, Sendable {
    public var type: OutboxItemType
    public var memoID: String?
    public var clientID: String?
    public var text: String?
    public var localImageReferences: [String]
    public var imagePaths: [String]
    public var expectedVersion: String
    public var createdAt: Date
    public var updatedAt: Date

    public init(
        type: OutboxItemType,
        memoID: String? = nil,
        clientID: String? = nil,
        text: String? = nil,
        localImageReferences: [String] = [],
        imagePaths: [String] = [],
        expectedVersion: String = "0",
        createdAt: Date,
        updatedAt: Date
    ) {
        self.type = type
        self.memoID = memoID
        self.clientID = clientID
        self.text = text
        self.localImageReferences = localImageReferences
        self.imagePaths = imagePaths
        self.expectedVersion = expectedVersion
        self.createdAt = createdAt
        self.updatedAt = updatedAt
    }
}
