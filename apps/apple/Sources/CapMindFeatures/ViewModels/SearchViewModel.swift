import Foundation
import CapMindCore

@MainActor
public final class SearchViewModel: ObservableObject {
    @Published public private(set) var state: SearchViewState

    private let memoRepository: any MemoRepository
    private let onlineProvider: OnlineStateProviding

    public init(
        memoRepository: any MemoRepository,
        onlineProvider: OnlineStateProviding,
        initialState: SearchViewState = SearchViewState()
    ) {
        self.memoRepository = memoRepository
        self.onlineProvider = onlineProvider
        self.state = initialState
    }

    public func open() {
        state.isPresented = true
        state.errorMessage = nil
    }

    public func close() {
        state.isPresented = false
    }

    public func clear() {
        state.query = ""
        state.results = []
        state.errorMessage = nil
    }

    public func updateQuery(_ query: String, limit: Int = 50) async {
        state.query = query
        let normalized = query.trimmingCharacters(in: .whitespacesAndNewlines)

        guard !normalized.isEmpty else {
            state.results = []
            state.errorMessage = nil
            return
        }

        guard onlineProvider.isOnline else {
            state.results = []
            state.errorMessage = "Search requires an internet connection"
            return
        }

        state.isSearching = true
        state.errorMessage = nil
        defer { state.isSearching = false }

        do {
            state.results = try await memoRepository.searchMemos(query: normalized, limit: limit)
        } catch {
            state.errorMessage = "Failed to search memos"
        }
    }
}
