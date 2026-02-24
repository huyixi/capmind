import Foundation
#if canImport(Network)
import Network
#endif

public extension Notification.Name {
    static let capMindOnlineStateDidChange = Notification.Name("capmind.online-state.did-change")
}

public protocol OnlineStateProviding: AnyObject {
    var isOnline: Bool { get }
}

public final class MutableOnlineStateProvider: OnlineStateProviding {
    public var isOnline: Bool {
        didSet {
            guard oldValue != isOnline else { return }
            NotificationCenter.default.post(
                name: .capMindOnlineStateDidChange,
                object: self,
                userInfo: ["isOnline": isOnline]
            )
        }
    }

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
            let next = path.status == .satisfied
            DispatchQueue.main.async {
                guard let self else { return }
                guard self.isOnline != next else { return }
                self.isOnline = next
                NotificationCenter.default.post(
                    name: .capMindOnlineStateDidChange,
                    object: self,
                    userInfo: ["isOnline": next]
                )
            }
        }
        monitor.start(queue: queue)
    }

    deinit {
        monitor.cancel()
    }
}
#endif
