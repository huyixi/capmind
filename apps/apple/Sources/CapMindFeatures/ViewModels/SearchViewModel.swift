import Foundation
import CapMindCore

@MainActor
public final class SearchViewModel: ObservableObject {
    @Published public private(set) var state: SearchViewState

    private let memoRepository: any MemoRepository
    private let onlineProvider: OnlineStateProviding
    private let userDefaults: UserDefaults
    private let recentsKey = "capmind.search.recent.v1"
    private let maxRecentCount = 8

    public init(
        memoRepository: any MemoRepository,
        onlineProvider: OnlineStateProviding,
        userDefaults: UserDefaults = .standard,
        initialState: SearchViewState = SearchViewState()
    ) {
        self.memoRepository = memoRepository
        self.onlineProvider = onlineProvider
        self.userDefaults = userDefaults
        self.state = initialState
        self.state.recentQueries = loadRecentQueries()
    }

    public func open() {
        state.isPresented = true
        state.errorMessage = nil
        state.recentQueries = loadRecentQueries()
    }

    public func close() {
        state.isPresented = false
    }

    public func clear() {
        state.query = ""
        state.results = []
        state.errorMessage = nil
    }

    public func applyRecentQuery(_ query: String) async {
        await updateQuery(query)
        commitCurrentQuery()
    }

    public func removeRecentQuery(_ query: String) {
        let trimmed = query.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return }
        state.recentQueries.removeAll { $0.caseInsensitiveCompare(trimmed) == .orderedSame }
        saveRecentQueries(state.recentQueries)
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

    public func commitCurrentQuery() {
        let normalized = state.query.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !normalized.isEmpty else { return }
        pushRecentQuery(normalized)
    }

    private func pushRecentQuery(_ query: String) {
        var next = state.recentQueries
        next.removeAll { $0.caseInsensitiveCompare(query) == .orderedSame }
        next.insert(query, at: 0)
        if next.count > maxRecentCount {
            next = Array(next.prefix(maxRecentCount))
        }
        state.recentQueries = next
        saveRecentQueries(next)
    }

    private func loadRecentQueries() -> [String] {
        guard let values = userDefaults.array(forKey: recentsKey) as? [String] else {
            return []
        }
        return values
            .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
            .filter { !$0.isEmpty }
    }

    private func saveRecentQueries(_ values: [String]) {
        userDefaults.set(values, forKey: recentsKey)
    }
}
