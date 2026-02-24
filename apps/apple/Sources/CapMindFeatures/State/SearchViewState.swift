import Foundation
import CapMindCore

public struct SearchViewState: Equatable {
    public var isPresented: Bool
    public var query: String
    public var results: [MemoEntity]
    public var recentQueries: [String]
    public var isSearching: Bool
    public var errorMessage: String?

    public init(
        isPresented: Bool = false,
        query: String = "",
        results: [MemoEntity] = [],
        recentQueries: [String] = [],
        isSearching: Bool = false,
        errorMessage: String? = nil
    ) {
        self.isPresented = isPresented
        self.query = query
        self.results = results
        self.recentQueries = recentQueries
        self.isSearching = isSearching
        self.errorMessage = errorMessage
    }
}
