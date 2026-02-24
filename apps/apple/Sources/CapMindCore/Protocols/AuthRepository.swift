import Foundation

public protocol AuthRepository: Sendable {
    func signIn(email: String, password: String) async throws -> UserSession
    func signUp(email: String, password: String, redirectURL: URL?) async throws -> UserSession?
    func restoreSession() async throws -> UserSession?
    func signOut() async throws
}
