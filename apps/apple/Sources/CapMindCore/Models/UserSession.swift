import Foundation

public struct UserSession: Codable, Equatable, Sendable {
    public var userID: String
    public var email: String
    public var accessToken: String
    public var refreshToken: String?
    public var expiresAt: Date?

    public init(
        userID: String,
        email: String,
        accessToken: String,
        refreshToken: String? = nil,
        expiresAt: Date? = nil
    ) {
        self.userID = userID
        self.email = email
        self.accessToken = accessToken
        self.refreshToken = refreshToken
        self.expiresAt = expiresAt
    }
}
