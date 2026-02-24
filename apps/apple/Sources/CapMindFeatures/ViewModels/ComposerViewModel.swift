import Foundation
import CapMindCore

@MainActor
public final class ComposerViewModel: ObservableObject {
    @Published public private(set) var state: ComposerViewState

    private let memoRepository: any MemoRepository
    private let imageRepository: any ImageRepository
    private let outboxRepository: any OutboxRepository
    private let onlineProvider: OnlineStateProviding

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

    public func submit(userID: String) async -> MemoEntity? {
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

    private func submitCreate(userID: String, text: String) async -> MemoEntity? {
        let now = Date()

        if !onlineProvider.isOnline {
            let localID = "local-\(UUID().uuidString.lowercased())"
            do {
                _ = try await outboxRepository.enqueue(
                    OutboxDraft(
                        type: .create,
                        clientID: localID,
                        text: text,
                        localImageReferences: state.imageReferences,
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
            let imagePaths = try await imageRepository.uploadImages(
                userID: userID,
                localReferences: state.imageReferences
            )
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
        let localRefs = refs.filter { !$0.contains("://") }
        let remoteRefs = refs.filter { $0.contains("://") }

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

            return try await memoRepository.updateMemo(
                id: editing.id,
                userID: userID,
                text: text,
                expectedVersion: editing.version,
                imagePaths: mergedPaths
            )
        } catch let error as CapMindError {
            if case .conflict(let serverMemo) = error {
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
}
