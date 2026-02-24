import Foundation
import CapMindCore

public actor InMemoryAuthRepository: AuthRepository {
    private struct UserRecord: Sendable {
        let id: String
        let email: String
        let password: String
    }

    private var usersByEmail: [String: UserRecord] = [:]
    private var currentSession: UserSession?

    public init() {}

    public func signIn(email: String, password: String) async throws -> UserSession {
        let key = email.lowercased()
        guard let user = usersByEmail[key], user.password == password else {
            throw CapMindError.unauthorized
        }

        let session = UserSession(
            userID: user.id,
            email: user.email,
            accessToken: UUID().uuidString,
            refreshToken: UUID().uuidString,
            expiresAt: Date().addingTimeInterval(60 * 60)
        )
        currentSession = session
        return session
    }

    public func signUp(email: String, password: String, redirectURL: URL?) async throws -> UserSession? {
        let key = email.lowercased()
        if usersByEmail[key] != nil {
            throw CapMindError.invalidInput(message: "Email already exists")
        }
        let record = UserRecord(id: UUID().uuidString.lowercased(), email: key, password: password)
        usersByEmail[key] = record

        let session = UserSession(
            userID: record.id,
            email: record.email,
            accessToken: UUID().uuidString,
            refreshToken: UUID().uuidString,
            expiresAt: Date().addingTimeInterval(60 * 60)
        )
        currentSession = session
        return session
    }

    public func restoreSession() async throws -> UserSession? {
        currentSession
    }

    public func signOut() async throws {
        currentSession = nil
    }
}
