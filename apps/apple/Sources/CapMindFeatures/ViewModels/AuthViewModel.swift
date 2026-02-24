import Foundation
import CapMindCore

@MainActor
public final class AuthViewModel: ObservableObject {
    @Published public private(set) var state: AuthViewModelState

    private let authRepository: any AuthRepository

    public init(
        authRepository: any AuthRepository,
        initialState: AuthViewModelState = AuthViewModelState()
    ) {
        self.authRepository = authRepository
        self.state = initialState
    }

    public var isAuthenticated: Bool {
        state.session != nil
    }

    public func setMode(_ mode: AuthViewModelState.Mode) {
        state.mode = mode
        state.errorMessage = nil
    }

    public func bootstrap() async {
        state.isLoading = true
        defer { state.isLoading = false }

        do {
            state.session = try await authRepository.restoreSession()
        } catch {
            state.errorMessage = "Failed to restore session"
        }
    }

    public func signIn() async {
        state.errorMessage = nil
        state.isLoading = true
        defer { state.isLoading = false }

        do {
            let session = try await authRepository.signIn(
                email: state.email.trimmingCharacters(in: .whitespacesAndNewlines),
                password: state.password
            )
            state.session = session
        } catch {
            state.errorMessage = "Sign in failed"
        }
    }

    public func signUp(redirectURL: URL?) async {
        state.errorMessage = nil

        if state.password != state.confirmPassword {
            state.errorMessage = "Passwords do not match"
            return
        }

        if state.password.count < 6 {
            state.errorMessage = "Password must be at least 6 characters"
            return
        }

        state.isLoading = true
        defer { state.isLoading = false }

        do {
            let session = try await authRepository.signUp(
                email: state.email.trimmingCharacters(in: .whitespacesAndNewlines),
                password: state.password,
                redirectURL: redirectURL
            )
            state.session = session
            if session == nil {
                state.errorMessage = "Check your email to complete sign up"
            }
        } catch {
            state.errorMessage = "Sign up failed"
        }
    }

    public func signOut() async {
        state.errorMessage = nil

        do {
            try await authRepository.signOut()
            state.session = nil
            state.password = ""
            state.confirmPassword = ""
        } catch {
            state.errorMessage = "Sign out failed"
        }
    }

    public func updateEmail(_ value: String) {
        state.email = value
    }

    public func updatePassword(_ value: String) {
        state.password = value
    }

    public func updateConfirmPassword(_ value: String) {
        state.confirmPassword = value
    }
}
