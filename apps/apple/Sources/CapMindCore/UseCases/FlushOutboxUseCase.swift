import Foundation

public struct FlushOutboxUseCase {
    private let memoRepository: any MemoRepository
    private let outboxRepository: any OutboxRepository
    private let imageRepository: any ImageRepository

    public init(
        memoRepository: any MemoRepository,
        outboxRepository: any OutboxRepository,
        imageRepository: any ImageRepository
    ) {
        self.memoRepository = memoRepository
        self.outboxRepository = outboxRepository
        self.imageRepository = imageRepository
    }

    public func run(userID: String) async -> SyncResult {
        let items: [OutboxItem]
        do {
            items = try await outboxRepository.listOrdered()
        } catch {
            return SyncResult(didSync: false, hadError: true, conflictCount: 0, processedCount: 0)
        }

        var didSync = false
        var hadError = false
        var conflictCount = 0
        var processedCount = 0

        for item in items {
            do {
                switch item.type {
                case .create:
                    let text = item.text?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
                    if text.isEmpty {
                        try await outboxRepository.remove(id: item.id)
                        continue
                    }

                    var imagePaths = item.imagePaths
                    if !item.localImageReferences.isEmpty {
                        let uploaded = try await imageRepository.uploadImages(
                            userID: userID,
                            localReferences: item.localImageReferences
                        )
                        imagePaths.append(contentsOf: uploaded)
                    }

                    _ = try await memoRepository.createMemo(
                        userID: userID,
                        text: text,
                        imagePaths: imagePaths,
                        createdAt: item.createdAt,
                        updatedAt: item.updatedAt,
                        clientID: item.clientID
                    )

                    try await outboxRepository.remove(id: item.id)
                    didSync = true
                    processedCount += 1

                case .update:
                    guard let memoID = item.memoID,
                          let text = item.text?.trimmingCharacters(in: .whitespacesAndNewlines),
                          !text.isEmpty
                    else {
                        try await outboxRepository.remove(id: item.id)
                        continue
                    }

                    let expectedVersion = MemoVersion.normalizeExpected(item.expectedVersion)
                    var imagePaths = item.imagePaths
                    if !item.localImageReferences.isEmpty {
                        let uploaded = try await imageRepository.uploadImages(
                            userID: userID,
                            localReferences: item.localImageReferences
                        )
                        imagePaths.append(contentsOf: uploaded)
                    }

                    do {
                        _ = try await memoRepository.updateMemo(
                            id: memoID,
                            userID: userID,
                            text: text,
                            expectedVersion: expectedVersion,
                            imagePaths: imagePaths.isEmpty ? nil : imagePaths
                        )
                    } catch let error as CapMindError {
                        guard case .conflict(let serverMemo) = error else {
                            throw error
                        }

                        _ = try await resolveServerMemoForConflict(
                            memoID: memoID,
                            userID: userID,
                            fallback: serverMemo
                        )
                        _ = try await memoRepository.createMemo(
                            userID: userID,
                            text: text,
                            imagePaths: imagePaths,
                            createdAt: item.createdAt,
                            updatedAt: item.updatedAt,
                            clientID: nil
                        )
                        conflictCount += 1
                    }

                    try await outboxRepository.remove(id: item.id)
                    didSync = true
                    processedCount += 1

                case .delete:
                    guard let memoID = item.memoID else {
                        try await outboxRepository.remove(id: item.id)
                        continue
                    }

                    let deleted = try await memoRepository.deleteMemo(
                        id: memoID,
                        userID: userID,
                        expectedVersion: MemoVersion.normalizeExpected(item.expectedVersion),
                        deletedAt: item.updatedAt
                    )

                    if deleted == nil {
                        _ = try await memoRepository.fetchMemo(id: memoID, userID: userID)
                        conflictCount += 1
                    }

                    try await outboxRepository.remove(id: item.id)
                    didSync = true
                    processedCount += 1

                case .restore:
                    guard let memoID = item.memoID else {
                        try await outboxRepository.remove(id: item.id)
                        continue
                    }

                    let restored = try await memoRepository.restoreMemo(
                        id: memoID,
                        userID: userID,
                        expectedVersion: MemoVersion.normalizeExpected(item.expectedVersion),
                        restoredAt: item.updatedAt
                    )

                    if restored == nil {
                        _ = try await memoRepository.fetchMemo(id: memoID, userID: userID)
                        conflictCount += 1
                    }

                    try await outboxRepository.remove(id: item.id)
                    didSync = true
                    processedCount += 1
                }
            } catch let error as CapMindError {
                if case .conflict = error {
                    do {
                        try await outboxRepository.remove(id: item.id)
                    } catch {
                        hadError = true
                        break
                    }
                    didSync = true
                    conflictCount += 1
                    processedCount += 1
                    continue
                }

                hadError = true
                break
            } catch {
                hadError = true
                break
            }
        }

        return SyncResult(
            didSync: didSync,
            hadError: hadError,
            conflictCount: conflictCount,
            processedCount: processedCount
        )
    }

    private func resolveServerMemoForConflict(
        memoID: String,
        userID: String,
        fallback: MemoEntity?
    ) async throws -> MemoEntity? {
        if let fallback {
            return fallback
        }
        return try await memoRepository.fetchMemo(id: memoID, userID: userID)
    }
}
