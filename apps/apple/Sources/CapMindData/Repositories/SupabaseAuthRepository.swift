import Foundation
import CapMindCore

public struct SupabaseAuthRepository: AuthRepository {
    private let client: any SupabaseAuthClientProtocol

    public init(client: any SupabaseAuthClientProtocol) {
        self.client = client
    }

    public func signIn(email: String, password: String) async throws -> UserSession {
        let payload = try await client.signIn(email: email, password: password)
        return UserSession(
            userID: payload.userID,
            email: payload.email,
            accessToken: payload.accessToken,
            refreshToken: payload.refreshToken,
            expiresAt: payload.expiresAt
        )
    }

    public func signUp(email: String, password: String, redirectURL: URL?) async throws -> UserSession? {
        guard let payload = try await client.signUp(email: email, password: password, redirectURL: redirectURL) else {
            return nil
        }
        return UserSession(
            userID: payload.userID,
            email: payload.email,
            accessToken: payload.accessToken,
            refreshToken: payload.refreshToken,
            expiresAt: payload.expiresAt
        )
    }

    public func restoreSession() async throws -> UserSession? {
        guard let payload = try await client.restoreSession() else {
            return nil
        }
        return UserSession(
            userID: payload.userID,
            email: payload.email,
            accessToken: payload.accessToken,
            refreshToken: payload.refreshToken,
            expiresAt: payload.expiresAt
        )
    }

    public func signOut() async throws {
        try await client.signOut()
    }
}
