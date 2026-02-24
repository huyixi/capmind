import Foundation
#if canImport(Network)
import Network
#endif

public protocol OnlineStateProviding: AnyObject {
    var isOnline: Bool { get }
}

public final class MutableOnlineStateProvider: OnlineStateProviding {
    public var isOnline: Bool

    public init(isOnline: Bool = true) {
        self.isOnline = isOnline
    }
}

#if canImport(Network)
public final class DefaultOnlineStateProvider: ObservableObject, OnlineStateProviding {
    @Published public private(set) var isOnline: Bool = true

    private let monitor = NWPathMonitor()
    private let queue = DispatchQueue(label: "capmind.network.monitor")

    public init() {
        monitor.pathUpdateHandler = { [weak self] path in
            self?.isOnline = path.status == .satisfied
        }
        monitor.start(queue: queue)
    }

    deinit {
        monitor.cancel()
    }
}
#endif
