import Foundation
import CapMindCore

@MainActor
public final class MemoListViewModel: ObservableObject {
    @Published public private(set) var state: MemoListViewState

    private let memoRepository: any MemoRepository
    private let outboxRepository: any OutboxRepository
    private let syncEngine: any SyncEngine
    private let onlineProvider: OnlineStateProviding
    private let pageSize: Int

    private var currentPage = 0
    private var userID: String?

    public init(
        memoRepository: any MemoRepository,
        outboxRepository: any OutboxRepository,
        syncEngine: any SyncEngine,
        onlineProvider: OnlineStateProviding,
        pageSize: Int = 20,
        initialState: MemoListViewState = MemoListViewState()
    ) {
        self.memoRepository = memoRepository
        self.outboxRepository = outboxRepository
        self.syncEngine = syncEngine
        self.onlineProvider = onlineProvider
        self.pageSize = pageSize
        self.state = initialState
    }

    public func loadInitial(userID: String) async {
        self.userID = userID
        currentPage = 0
        state.isLoadingInitial = true
        state.errorMessage = nil

        defer { state.isLoadingInitial = false }

        do {
            let memos = try await memoRepository.listMemos(
                page: 0,
                pageSize: pageSize,
                isTrash: state.isTrashActive
            )
            state.memos = memos
            state.hasReachedEnd = memos.count < pageSize
        } catch {
            state.errorMessage = "Failed to load memos"
        }
    }

    public func loadMoreIfNeeded(currentMemo: MemoEntity?) async {
        guard let currentMemo else { return }
        guard !state.isLoadingMore, !state.hasReachedEnd else { return }
        guard state.memos.last?.id == currentMemo.id else { return }

        state.isLoadingMore = true
        defer { state.isLoadingMore = false }

        do {
            let nextPage = currentPage + 1
            let nextItems = try await memoRepository.listMemos(
                page: nextPage,
                pageSize: pageSize,
                isTrash: state.isTrashActive
            )
            currentPage = nextPage
            state.memos.append(contentsOf: nextItems)
            state.hasReachedEnd = nextItems.count < pageSize
        } catch {
            state.errorMessage = "Failed to load more memos"
        }
    }

    public func refresh() async {
        guard let userID else { return }
        state.isRefreshing = true
        state.errorMessage = nil

        defer { state.isRefreshing = false }

        _ = await syncNow()
        await loadInitial(userID: userID)
    }

    public func setTrashActive(_ value: Bool) async {
        guard state.isTrashActive != value else { return }
        state.isTrashActive = value
        guard let userID else { return }
        await loadInitial(userID: userID)
    }

    public func syncNow() async -> SyncResult {
        guard let userID else { return .idle }
        state.isSyncing = true
        defer { state.isSyncing = false }

        let result = await syncEngine.flushOutbox(userID: userID)
        if result.hadError {
            state.errorMessage = "Sync failed. Pending items will retry later."
        } else if result.conflictCount > 0 {
            state.errorMessage = "Sync completed with conflicts. Conflicting edits were saved as new memos."
        } else {
            state.errorMessage = nil
        }
        return result
    }

    public func syncAndReloadIfNeeded() async {
        guard let userID else { return }
        let result = await syncNow()
        if result.didSync || result.conflictCount > 0 {
            await loadInitial(userID: userID)
        }
    }

    public func upsertMemo(_ memo: MemoEntity) {
        if state.isTrashActive, memo.deletedAt == nil {
            state.memos.removeAll { $0.id == memo.id }
            return
        }

        if !state.isTrashActive, memo.deletedAt != nil {
            state.memos.removeAll { $0.id == memo.id }
            return
        }

        if let index = state.memos.firstIndex(where: { $0.id == memo.id }) {
            state.memos[index] = memo
        } else {
            state.memos.insert(memo, at: 0)
        }

        state.memos.sort { $0.createdAt > $1.createdAt }
    }

    public func removeMemo(id: String) {
        state.memos.removeAll { $0.id == id }
    }

    public func deleteMemo(_ memo: MemoEntity) async {
        guard let userID else { return }
        let now = Date()

        if !onlineProvider.isOnline {
            do {
                _ = try await outboxRepository.enqueue(
                    OutboxDraft(
                        type: .delete,
                        memoID: memo.id,
                        text: nil,
                        expectedVersion: memo.version,
                        createdAt: now,
                        updatedAt: now
                    )
                )
                removeMemo(id: memo.id)
            } catch {
                state.errorMessage = "Failed to queue delete"
            }
            return
        }

        do {
            let result = try await memoRepository.deleteMemo(
                id: memo.id,
                userID: userID,
                expectedVersion: memo.version,
                deletedAt: now
            )
            if let result {
                upsertMemo(result)
            } else {
                removeMemo(id: memo.id)
            }
        } catch {
            state.errorMessage = "Delete failed"
        }
    }

    public func restoreMemo(_ memo: MemoEntity) async {
        guard let userID else { return }
        let now = Date()

        if !onlineProvider.isOnline {
            do {
                _ = try await outboxRepository.enqueue(
                    OutboxDraft(
                        type: .restore,
                        memoID: memo.id,
                        text: nil,
                        expectedVersion: memo.version,
                        createdAt: now,
                        updatedAt: now
                    )
                )
                removeMemo(id: memo.id)
            } catch {
                state.errorMessage = "Failed to queue restore"
            }
            return
        }

        do {
            let result = try await memoRepository.restoreMemo(
                id: memo.id,
                userID: userID,
                expectedVersion: memo.version,
                restoredAt: now
            )
            if let result {
                upsertMemo(result)
            } else {
                removeMemo(id: memo.id)
            }
        } catch {
            state.errorMessage = "Restore failed"
        }
    }
}
