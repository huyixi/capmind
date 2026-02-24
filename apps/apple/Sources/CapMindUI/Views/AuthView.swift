import SwiftUI
import CapMindFeatures

public struct AuthView: View {
    @ObservedObject private var viewModel: AuthViewModel

    public init(viewModel: AuthViewModel) {
        self.viewModel = viewModel
    }

    public var body: some View {
        VStack(spacing: 16) {
            Text(viewModel.state.mode == .signIn ? "Welcome back" : "Create account")
                .font(.title2.bold())

            TextField("Email", text: Binding(
                get: { viewModel.state.email },
                set: { viewModel.updateEmail($0) }
            ))
            .textFieldStyle(RoundedBorderTextFieldStyle())
#if os(iOS)
            .textInputAutocapitalization(.never)
            .autocorrectionDisabled()
#endif

            SecureField("Password", text: Binding(
                get: { viewModel.state.password },
                set: { viewModel.updatePassword($0) }
            ))
            .textFieldStyle(RoundedBorderTextFieldStyle())

            if viewModel.state.mode == .signUp {
                SecureField("Confirm password", text: Binding(
                    get: { viewModel.state.confirmPassword },
                    set: { viewModel.updateConfirmPassword($0) }
                ))
                .textFieldStyle(RoundedBorderTextFieldStyle())
            }

            if let errorMessage = viewModel.state.errorMessage {
                Text(errorMessage)
                    .font(.footnote)
                    .foregroundStyle(.red)
            }

            Button(viewModel.state.mode == .signIn ? "Sign in" : "Create account") {
                Task {
                    if viewModel.state.mode == .signIn {
                        await viewModel.signIn()
                    } else {
                        await viewModel.signUp(redirectURL: nil)
                    }
                }
            }
            .buttonStyle(.borderedProminent)
            .disabled(viewModel.state.isLoading)

            Button(viewModel.state.mode == .signIn ? "Need an account? Sign up" : "Have an account? Sign in") {
                viewModel.setMode(viewModel.state.mode == .signIn ? .signUp : .signIn)
            }
            .buttonStyle(.plain)
        }
        .padding(24)
        .frame(maxWidth: 420)
        .task {
            if viewModel.state.session == nil {
                await viewModel.bootstrap()
            }
        }
    }
}
