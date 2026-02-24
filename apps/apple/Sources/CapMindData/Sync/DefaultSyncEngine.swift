import Foundation
import CapMindCore

public actor DefaultSyncEngine: SyncEngine {
    private let useCase: FlushOutboxUseCase
    private let isOnline: @Sendable () -> Bool

    public init(
        memoRepository: any MemoRepository,
        outboxRepository: any OutboxRepository,
        imageRepository: any ImageRepository,
        isOnline: @escaping @Sendable () -> Bool
    ) {
        self.useCase = FlushOutboxUseCase(
            memoRepository: memoRepository,
            outboxRepository: outboxRepository,
            imageRepository: imageRepository
        )
        self.isOnline = isOnline
    }

    public func flushOutbox(userID: String) async -> SyncResult {
        guard !userID.isEmpty else {
            return .idle
        }
        guard isOnline() else {
            return .idle
        }
        return await useCase.run(userID: userID)
    }
}
