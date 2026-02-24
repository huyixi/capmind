import Foundation
import CapMindCore

public actor InMemoryOutboxRepository: OutboxRepository {
    private var items: [OutboxItem] = []
    private var nextSequence: Int64 = 1

    public init() {}

    public func enqueue(_ draft: OutboxDraft) async throws -> OutboxItem {
        let item = OutboxItem(
            sequence: nextSequence,
            type: draft.type,
            memoID: draft.memoID,
            clientID: draft.clientID,
            text: draft.text,
            localImageReferences: draft.localImageReferences,
            imagePaths: draft.imagePaths,
            expectedVersion: draft.expectedVersion,
            createdAt: draft.createdAt,
            updatedAt: draft.updatedAt
        )
        nextSequence += 1
        items.append(item)
        return item
    }

    public func listOrdered() async throws -> [OutboxItem] {
        items.sorted { $0.sequence < $1.sequence }
    }

    public func remove(id: UUID) async throws {
        items.removeAll { $0.id == id }
    }

    public func removePendingCreate(clientID: String) async throws {
        items.removeAll { $0.type == .create && $0.clientID == clientID }
    }

    public func updatePendingCreate(clientID: String, text: String, updatedAt: Date) async throws -> Bool {
        guard let index = items.firstIndex(where: { $0.type == .create && $0.clientID == clientID }) else {
            return false
        }
        items[index].text = text
        items[index].updatedAt = updatedAt
        return true
    }
}
