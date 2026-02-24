import Foundation

public protocol OutboxRepository: Sendable {
    func enqueue(_ draft: OutboxDraft) async throws -> OutboxItem
    func listOrdered() async throws -> [OutboxItem]
    func remove(id: UUID) async throws
    func removePendingCreate(clientID: String) async throws
    func updatePendingCreate(clientID: String, text: String, updatedAt: Date) async throws -> Bool
}
