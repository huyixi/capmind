import Foundation

public protocol SyncEngine: Sendable {
    func flushOutbox(userID: String) async -> SyncResult
}
