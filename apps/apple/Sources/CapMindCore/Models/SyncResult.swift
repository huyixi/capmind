import Foundation

public struct SyncResult: Equatable, Sendable {
    public var didSync: Bool
    public var hadError: Bool
    public var conflictCount: Int
    public var processedCount: Int

    public init(didSync: Bool, hadError: Bool, conflictCount: Int, processedCount: Int) {
        self.didSync = didSync
        self.hadError = hadError
        self.conflictCount = conflictCount
        self.processedCount = processedCount
    }

    public static let idle = SyncResult(
        didSync: false,
        hadError: false,
        conflictCount: 0,
        processedCount: 0
    )
}
