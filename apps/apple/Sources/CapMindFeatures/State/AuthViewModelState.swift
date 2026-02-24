import Foundation
import CapMindCore

public struct AuthViewModelState: Equatable {
    public enum Mode: String {
        case signIn
        case signUp
    }

    public var mode: Mode
    public var email: String
    public var password: String
    public var confirmPassword: String
    public var isLoading: Bool
    public var errorMessage: String?
    public var session: UserSession?

    public init(
        mode: Mode = .signIn,
        email: String = "",
        password: String = "",
        confirmPassword: String = "",
        isLoading: Bool = false,
        errorMessage: String? = nil,
        session: UserSession? = nil
    ) {
        self.mode = mode
        self.email = email
        self.password = password
        self.confirmPassword = confirmPassword
        self.isLoading = isLoading
        self.errorMessage = errorMessage
        self.session = session
    }
}
