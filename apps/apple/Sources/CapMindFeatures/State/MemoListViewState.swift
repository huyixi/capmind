import Foundation
import CapMindCore

public struct MemoListViewState: Equatable {
    public var memos: [MemoEntity]
    public var isLoadingInitial: Bool
    public var isLoadingMore: Bool
    public var isRefreshing: Bool
    public var isSyncing: Bool
    public var isTrashActive: Bool
    public var searchQuery: String
    public var errorMessage: String?
    public var hasReachedEnd: Bool

    public init(
        memos: [MemoEntity] = [],
        isLoadingInitial: Bool = false,
        isLoadingMore: Bool = false,
        isRefreshing: Bool = false,
        isSyncing: Bool = false,
        isTrashActive: Bool = false,
        searchQuery: String = "",
        errorMessage: String? = nil,
        hasReachedEnd: Bool = false
    ) {
        self.memos = memos
        self.isLoadingInitial = isLoadingInitial
        self.isLoadingMore = isLoadingMore
        self.isRefreshing = isRefreshing
        self.isSyncing = isSyncing
        self.isTrashActive = isTrashActive
        self.searchQuery = searchQuery
        self.errorMessage = errorMessage
        self.hasReachedEnd = hasReachedEnd
    }
}
