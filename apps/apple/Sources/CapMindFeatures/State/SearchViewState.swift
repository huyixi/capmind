import Foundation
import CapMindCore

public struct SearchViewState: Equatable {
    public var isPresented: Bool
    public var query: String
    public var results: [MemoEntity]
    public var isSearching: Bool
    public var errorMessage: String?

    public init(
        isPresented: Bool = false,
        query: String = "",
        results: [MemoEntity] = [],
        isSearching: Bool = false,
        errorMessage: String? = nil
    ) {
        self.isPresented = isPresented
        self.query = query
        self.results = results
        self.isSearching = isSearching
        self.errorMessage = errorMessage
    }
}
