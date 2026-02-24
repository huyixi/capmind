import Foundation
import CapMindCore
import CapMindData

public struct CapMindDependencies {
    public let authRepository: any AuthRepository
    public let memoRepository: any MemoRepository
    public let imageRepository: any ImageRepository
    public let outboxRepository: any OutboxRepository
    public let syncEngine: any SyncEngine
    public let onlineProvider: OnlineStateProviding

    public init(
        authRepository: any AuthRepository,
        memoRepository: any MemoRepository,
        imageRepository: any ImageRepository,
        outboxRepository: any OutboxRepository,
        syncEngine: any SyncEngine,
        onlineProvider: OnlineStateProviding
    ) {
        self.authRepository = authRepository
        self.memoRepository = memoRepository
        self.imageRepository = imageRepository
        self.outboxRepository = outboxRepository
        self.syncEngine = syncEngine
        self.onlineProvider = onlineProvider
    }

    public static func inMemoryDemo() -> CapMindDependencies {
        let onlineProvider = MutableOnlineStateProvider(isOnline: true)
        let authRepository = InMemoryAuthRepository()
        let memoRepository = InMemoryMemoRepository()
        let imageRepository = InMemoryImageRepository()
        let outboxRepository = defaultOutboxRepository()
        let syncEngine = DefaultSyncEngine(
            memoRepository: memoRepository,
            outboxRepository: outboxRepository,
            imageRepository: imageRepository,
            isOnline: { onlineProvider.isOnline }
        )

        return CapMindDependencies(
            authRepository: authRepository,
            memoRepository: memoRepository,
            imageRepository: imageRepository,
            outboxRepository: outboxRepository,
            syncEngine: syncEngine,
            onlineProvider: onlineProvider
        )
    }

    public static func supabase(
        authClient: any SupabaseAuthClientProtocol,
        memoClient: any SupabaseMemoClientProtocol,
        storageClient: any SupabaseStorageClientProtocol,
        onlineProvider: OnlineStateProviding
    ) -> CapMindDependencies {
        let authRepository = SupabaseAuthRepository(client: authClient)
        let memoRepository = SupabaseMemoRepository(client: memoClient)
        let imageRepository = SupabaseImageRepository(client: storageClient)
        let outboxRepository = defaultOutboxRepository()
        let syncEngine = DefaultSyncEngine(
            memoRepository: memoRepository,
            outboxRepository: outboxRepository,
            imageRepository: imageRepository,
            isOnline: { onlineProvider.isOnline }
        )

        return CapMindDependencies(
            authRepository: authRepository,
            memoRepository: memoRepository,
            imageRepository: imageRepository,
            outboxRepository: outboxRepository,
            syncEngine: syncEngine,
            onlineProvider: onlineProvider
        )
    }

    public static func supabaseLive(
        configuration: SupabaseConfiguration,
        onlineProvider: OnlineStateProviding
    ) -> CapMindDependencies {
        let bundle = LiveSupabaseClientBundle(configuration: configuration)
        return supabase(
            authClient: bundle.auth,
            memoClient: bundle.memo,
            storageClient: bundle.storage,
            onlineProvider: onlineProvider
        )
    }

    private static func defaultOutboxRepository() -> any OutboxRepository {
        SQLiteOutboxRepository.makeDefault()
    }
}
