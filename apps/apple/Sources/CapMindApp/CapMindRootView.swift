import SwiftUI
import CapMindFeatures
import CapMindUI
import CapMindData

public struct CapMindRootView: View {
    @StateObject private var authViewModel: AuthViewModel
    @StateObject private var listViewModel: MemoListViewModel
    @StateObject private var composerViewModel: ComposerViewModel
    @StateObject private var searchViewModel: SearchViewModel

    public init(dependencies: CapMindDependencies = CapMindRootView.defaultDependencies()) {
        _authViewModel = StateObject(
            wrappedValue: AuthViewModel(authRepository: dependencies.authRepository)
        )
        _listViewModel = StateObject(
            wrappedValue: MemoListViewModel(
                memoRepository: dependencies.memoRepository,
                outboxRepository: dependencies.outboxRepository,
                syncEngine: dependencies.syncEngine,
                onlineProvider: dependencies.onlineProvider
            )
        )
        _composerViewModel = StateObject(
            wrappedValue: ComposerViewModel(
                memoRepository: dependencies.memoRepository,
                imageRepository: dependencies.imageRepository,
                outboxRepository: dependencies.outboxRepository,
                onlineProvider: dependencies.onlineProvider
            )
        )
        _searchViewModel = StateObject(
            wrappedValue: SearchViewModel(
                memoRepository: dependencies.memoRepository,
                onlineProvider: dependencies.onlineProvider
            )
        )
    }

    public static func defaultDependencies() -> CapMindDependencies {
        #if canImport(Network)
        let onlineProvider: OnlineStateProviding = DefaultOnlineStateProvider()
        #else
        let onlineProvider: OnlineStateProviding = MutableOnlineStateProvider(isOnline: true)
        #endif
        if let configuration = SupabaseConfiguration.fromEnvironment() {
            return .supabaseLive(
                configuration: configuration,
                onlineProvider: onlineProvider
            )
        }
        return .inMemoryDemo()
    }

    public var body: some View {
        Group {
            if authViewModel.isAuthenticated {
                MemoHomeView(
                    authViewModel: authViewModel,
                    listViewModel: listViewModel,
                    composerViewModel: composerViewModel,
                    searchViewModel: searchViewModel
                )
            } else {
                AuthView(viewModel: authViewModel)
            }
        }
    }
}
