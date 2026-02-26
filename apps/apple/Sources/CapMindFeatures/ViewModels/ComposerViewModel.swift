import Foundation
import CapMindCore

@MainActor
public final class ComposerViewModel: ObservableObject {
    @Published public private(set) var state: ComposerViewState

    private let memoRepository: any MemoRepository
    private let imageRepository: any ImageRepository
    private let outboxRepository: any OutboxRepository
    private let onlineProvider: OnlineStateProviding
    private var pendingConflictForkedMemo: MemoEntity?

    public init(
        memoRepository: any MemoRepository,
        imageRepository: any ImageRepository,
        outboxRepository: any OutboxRepository,
        onlineProvider: OnlineStateProviding,
        initialState: ComposerViewState = ComposerViewState()
    ) {
        self.memoRepository = memoRepository
        self.imageRepository = imageRepository
        self.outboxRepository = outboxRepository
        self.onlineProvider = onlineProvider
        self.state = initialState
    }

    public func openCreate(draftText: String = "") {
        pendingConflictForkedMemo = nil
        state = ComposerViewState(
            isPresented: true,
            mode: .create,
            text: draftText,
            imageReferences: [],
            editingMemo: nil,
            isSubmitting: false,
            errorMessage: nil
        )
    }

    public func openEdit(memo: MemoEntity) {
        pendingConflictForkedMemo = nil
        state = ComposerViewState(
            isPresented: true,
            mode: .edit,
            text: memo.text,
            imageReferences: memo.images,
            editingMemo: memo,
            isSubmitting: false,
            errorMessage: nil
        )
    }

    public func close() {
        pendingConflictForkedMemo = nil
        state.isPresented = false
        state.isSubmitting = false
        state.errorMessage = nil
    }

    public func updateText(_ value: String) {
        state.text = value
    }

    public func updateImageReferences(_ refs: [String]) {
        state.imageReferences = refs
    }

    public func appendImageReference(_ reference: String) {
        let normalized = reference.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !normalized.isEmpty else { return }
        if state.imageReferences.contains(normalized) {
            return
        }
        state.imageReferences.append(normalized)
    }

    public func removeImageReference(_ reference: String) {
        state.imageReferences.removeAll { $0 == reference }
    }

    public func submit(userID: String) async -> MemoEntity? {
        pendingConflictForkedMemo = nil
        state.errorMessage = nil
        let text = state.text.trimmingCharacters(in: .whitespacesAndNewlines)

        guard !text.isEmpty else {
            state.errorMessage = "Memo text cannot be empty"
            return nil
        }

        state.isSubmitting = true
        defer { state.isSubmitting = false }

        switch state.mode {
        case .create:
            return await submitCreate(userID: userID, text: text)
        case .edit:
            return await submitEdit(userID: userID, text: text)
        }
    }

    public func consumePendingForkedMemo() -> MemoEntity? {
        let value = pendingConflictForkedMemo
        pendingConflictForkedMemo = nil
        return value
    }

    private func submitCreate(userID: String, text: String) async -> MemoEntity? {
        let now = Date()
        let imageSplit = splitImageReferences(state.imageReferences)

        if !onlineProvider.isOnline {
            let localID = "local-\(UUID().uuidString.lowercased())"
            do {
                _ = try await outboxRepository.enqueue(
                    OutboxDraft(
                        type: .create,
                        clientID: localID,
                        text: text,
                        localImageReferences: imageSplit.local,
                        imagePaths: imageSplit.remote,
                        expectedVersion: "0",
                        createdAt: now,
                        updatedAt: now
                    )
                )
            } catch {
                state.errorMessage = "Failed to queue memo"
                return nil
            }

            return MemoEntity(
                id: localID,
                clientID: localID,
                userID: userID,
                text: text,
                images: state.imageReferences,
                createdAt: now,
                updatedAt: now,
                version: "1",
                deletedAt: nil,
                serverVersion: "1",
                conflictType: nil
            )
        }

        do {
            let uploaded = try await imageRepository.uploadImages(
                userID: userID,
                localReferences: imageSplit.local
            )
            let imagePaths = imageSplit.remote + uploaded
            return try await memoRepository.createMemo(
                userID: userID,
                text: text,
                imagePaths: imagePaths,
                createdAt: now,
                updatedAt: now,
                clientID: nil
            )
        } catch {
            state.errorMessage = "Failed to create memo"
            return nil
        }
    }

    private func submitEdit(userID: String, text: String) async -> MemoEntity? {
        guard let editing = state.editingMemo else {
            state.errorMessage = "No memo selected"
            return nil
        }

        let now = Date()
        let refs = state.imageReferences
        let split = splitImageReferences(refs)
        let localRefs = split.local
        let remoteRefs = split.remote

        if !onlineProvider.isOnline {
            do {
                _ = try await outboxRepository.enqueue(
                    OutboxDraft(
                        type: .update,
                        memoID: editing.id,
                        text: text,
                        localImageReferences: localRefs,
                        imagePaths: remoteRefs,
                        expectedVersion: editing.version,
                        createdAt: now,
                        updatedAt: now
                    )
                )

                var optimistic = editing
                optimistic.text = text
                optimistic.images = refs
                optimistic.updatedAt = now
                optimistic.version = MemoVersion.next(editing.version)
                optimistic.serverVersion = optimistic.version
                optimistic.conflictType = nil
                return optimistic
            } catch {
                state.errorMessage = "Failed to queue update"
                return nil
            }
        }

        do {
            let uploaded = try await imageRepository.uploadImages(
                userID: userID,
                localReferences: localRefs
            )
            let mergedPaths = remoteRefs + uploaded
            let shouldKeepExistingImages =
                mergedPaths.isEmpty &&
                editing.hasImages &&
                editing.imageCount > 0

            return try await memoRepository.updateMemo(
                id: editing.id,
                userID: userID,
                text: text,
                expectedVersion: editing.version,
                imagePaths: shouldKeepExistingImages ? nil : mergedPaths
            )
        } catch let error as CapMindError {
            if case .conflict(let serverMemo, let forkedMemo) = error {
                pendingConflictForkedMemo = forkedMemo
                state.errorMessage = "Version conflict. Please retry."
                return serverMemo
            }
            state.errorMessage = "Failed to update memo"
            return nil
        } catch {
            state.errorMessage = "Failed to update memo"
            return nil
        }
    }

    private func splitImageReferences(_ refs: [String]) -> (local: [String], remote: [String]) {
        var local: [String] = []
        var remote: [String] = []

        for ref in refs {
            let normalized = ref.trimmingCharacters(in: .whitespacesAndNewlines)
            guard !normalized.isEmpty else { continue }
            if isLocalImageReference(normalized) {
                local.append(normalized)
            } else {
                remote.append(normalized)
            }
        }

        return (local: local, remote: remote)
    }

    private func isLocalImageReference(_ reference: String) -> Bool {
        if let url = URL(string: reference), let scheme = url.scheme?.lowercased() {
            if url.isFileURL {
                return true
            }
            return scheme != "http" && scheme != "https"
        }

        return reference.hasPrefix("/") || reference.hasPrefix("~")
    }
}
