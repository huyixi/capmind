import Foundation
import SQLite3
import CapMindCore

private let sqliteTransient = unsafeBitCast(-1, to: sqlite3_destructor_type.self)

public actor SQLiteOutboxRepository: OutboxRepository {
    private let db: OpaquePointer
    private let encoder = JSONEncoder()
    private let decoder = JSONDecoder()

    public static func defaultDatabaseURL() throws -> URL {
        let base = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
            ?? FileManager.default.temporaryDirectory
        let directory = base.appendingPathComponent("capmind", isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        return directory.appendingPathComponent("outbox.sqlite3", isDirectory: false)
    }

    public static func makeDefault() -> any OutboxRepository {
        do {
            return try SQLiteOutboxRepository(fileURL: defaultDatabaseURL())
        } catch {
            return InMemoryOutboxRepository()
        }
    }

    public init(fileURL: URL) throws {
        let parent = fileURL.deletingLastPathComponent()
        try FileManager.default.createDirectory(at: parent, withIntermediateDirectories: true)

        var handle: OpaquePointer?
        if sqlite3_open(fileURL.path, &handle) != SQLITE_OK {
            let message = handle.flatMap { String(cString: sqlite3_errmsg($0)) } ?? "unknown"
            if let handle {
                sqlite3_close(handle)
            }
            throw CapMindError.network(message: "Failed to open outbox database: \(message)")
        }

        guard let handle else {
            throw CapMindError.network(message: "Failed to open outbox database")
        }

        db = handle

        try Self.executeOn(db: db, sql: "PRAGMA journal_mode=WAL;")
        try Self.executeOn(db: db, sql: "PRAGMA foreign_keys=ON;")
        try Self.executeOn(
            db: db,
            sql: """
            CREATE TABLE IF NOT EXISTS outbox (
              sequence INTEGER PRIMARY KEY AUTOINCREMENT,
              id TEXT NOT NULL UNIQUE,
              type TEXT NOT NULL,
              memo_id TEXT,
              client_id TEXT,
              text TEXT,
              local_image_references TEXT NOT NULL,
              image_paths TEXT NOT NULL,
              expected_version TEXT NOT NULL,
              created_at REAL NOT NULL,
              updated_at REAL NOT NULL
            );
            """
        )
        try Self.executeOn(
            db: db,
            sql: "CREATE INDEX IF NOT EXISTS idx_outbox_client_id ON outbox(client_id);"
        )
        try Self.executeOn(
            db: db,
            sql: "CREATE INDEX IF NOT EXISTS idx_outbox_type ON outbox(type);"
        )
    }

    deinit {
        sqlite3_close(db)
    }

    public func enqueue(_ draft: OutboxDraft) async throws -> OutboxItem {
        let id = UUID()
        let localImages = try encodeStringArray(draft.localImageReferences)
        let imagePaths = try encodeStringArray(draft.imagePaths)

        let sql = """
        INSERT INTO outbox (
          id, type, memo_id, client_id, text,
          local_image_references, image_paths, expected_version,
          created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?);
        """

        var statement: OpaquePointer?
        defer { sqlite3_finalize(statement) }

        try prepare(sql: sql, statement: &statement)

        bind(text: id.uuidString.lowercased(), at: 1, statement: statement)
        bind(text: draft.type.rawValue, at: 2, statement: statement)
        bind(text: draft.memoID, at: 3, statement: statement)
        bind(text: draft.clientID, at: 4, statement: statement)
        bind(text: draft.text, at: 5, statement: statement)
        bind(text: localImages, at: 6, statement: statement)
        bind(text: imagePaths, at: 7, statement: statement)
        bind(text: draft.expectedVersion, at: 8, statement: statement)
        sqlite3_bind_double(statement, 9, draft.createdAt.timeIntervalSince1970)
        sqlite3_bind_double(statement, 10, draft.updatedAt.timeIntervalSince1970)

        try stepDone(statement: statement)

        let sequence = sqlite3_last_insert_rowid(db)
        return OutboxItem(
            id: id,
            sequence: sequence,
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
    }

    public func listOrdered() async throws -> [OutboxItem] {
        let sql = """
        SELECT
          sequence, id, type, memo_id, client_id, text,
          local_image_references, image_paths, expected_version,
          created_at, updated_at
        FROM outbox
        ORDER BY sequence ASC;
        """

        var statement: OpaquePointer?
        defer { sqlite3_finalize(statement) }

        try prepare(sql: sql, statement: &statement)

        var items: [OutboxItem] = []
        while sqlite3_step(statement) == SQLITE_ROW {
            let sequence = sqlite3_column_int64(statement, 0)
            let idString = columnText(statement: statement, index: 1) ?? ""
            let typeString = columnText(statement: statement, index: 2) ?? ""
            let memoID = columnText(statement: statement, index: 3)
            let clientID = columnText(statement: statement, index: 4)
            let text = columnText(statement: statement, index: 5)
            let localImagesJSON = columnText(statement: statement, index: 6) ?? "[]"
            let imagePathsJSON = columnText(statement: statement, index: 7) ?? "[]"
            let expectedVersion = columnText(statement: statement, index: 8) ?? "0"
            let createdAt = Date(timeIntervalSince1970: sqlite3_column_double(statement, 9))
            let updatedAt = Date(timeIntervalSince1970: sqlite3_column_double(statement, 10))

            guard let id = UUID(uuidString: idString),
                  let type = OutboxItemType(rawValue: typeString)
            else {
                continue
            }

            let localImages = try decodeStringArray(localImagesJSON)
            let imagePaths = try decodeStringArray(imagePathsJSON)

            items.append(
                OutboxItem(
                    id: id,
                    sequence: sequence,
                    type: type,
                    memoID: memoID,
                    clientID: clientID,
                    text: text,
                    localImageReferences: localImages,
                    imagePaths: imagePaths,
                    expectedVersion: expectedVersion,
                    createdAt: createdAt,
                    updatedAt: updatedAt
                )
            )
        }

        if sqlite3_errcode(db) != SQLITE_OK && sqlite3_errcode(db) != SQLITE_DONE {
            throw sqliteError(prefix: "Failed to read outbox")
        }

        return items
    }

    public func remove(id: UUID) async throws {
        let sql = "DELETE FROM outbox WHERE id = ?;"
        var statement: OpaquePointer?
        defer { sqlite3_finalize(statement) }

        try prepare(sql: sql, statement: &statement)
        bind(text: id.uuidString.lowercased(), at: 1, statement: statement)
        try stepDone(statement: statement)
    }

    public func removePendingCreate(clientID: String) async throws {
        let sql = "DELETE FROM outbox WHERE type = 'create' AND client_id = ?;"
        var statement: OpaquePointer?
        defer { sqlite3_finalize(statement) }

        try prepare(sql: sql, statement: &statement)
        bind(text: clientID, at: 1, statement: statement)
        try stepDone(statement: statement)
    }

    public func updatePendingCreate(clientID: String, text: String, updatedAt: Date) async throws -> Bool {
        let sql = """
        UPDATE outbox
        SET text = ?, updated_at = ?
        WHERE type = 'create' AND client_id = ?;
        """

        var statement: OpaquePointer?
        defer { sqlite3_finalize(statement) }

        try prepare(sql: sql, statement: &statement)
        bind(text: text, at: 1, statement: statement)
        sqlite3_bind_double(statement, 2, updatedAt.timeIntervalSince1970)
        bind(text: clientID, at: 3, statement: statement)
        try stepDone(statement: statement)

        return sqlite3_changes(db) > 0
    }

    private func execute(sql: String) throws {
        var errorMessage: UnsafeMutablePointer<Int8>?
        if sqlite3_exec(db, sql, nil, nil, &errorMessage) != SQLITE_OK {
            let message = errorMessage.map { String(cString: $0) } ?? "unknown"
            sqlite3_free(errorMessage)
            throw CapMindError.network(message: "SQLite error: \(message)")
        }
    }

    private func prepare(sql: String, statement: inout OpaquePointer?) throws {
        if sqlite3_prepare_v2(db, sql, -1, &statement, nil) != SQLITE_OK {
            throw sqliteError(prefix: "Failed to prepare statement")
        }
    }

    private func stepDone(statement: OpaquePointer?) throws {
        if sqlite3_step(statement) != SQLITE_DONE {
            throw sqliteError(prefix: "Failed to execute statement")
        }
    }

    private func bind(text: String?, at index: Int32, statement: OpaquePointer?) {
        guard let text else {
            sqlite3_bind_null(statement, index)
            return
        }
        sqlite3_bind_text(statement, index, text, -1, sqliteTransient)
    }

    private func columnText(statement: OpaquePointer?, index: Int32) -> String? {
        guard let pointer = sqlite3_column_text(statement, index) else {
            return nil
        }
        return String(cString: pointer)
    }

    private func encodeStringArray(_ value: [String]) throws -> String {
        let data = try encoder.encode(value)
        guard let json = String(data: data, encoding: .utf8) else {
            throw CapMindError.network(message: "Failed to encode outbox array")
        }
        return json
    }

    private func decodeStringArray(_ value: String) throws -> [String] {
        let data = Data(value.utf8)
        return try decoder.decode([String].self, from: data)
    }

    private func sqliteError(prefix: String) -> CapMindError {
        let raw = String(cString: sqlite3_errmsg(db))
        return CapMindError.network(message: "\(prefix): \(raw)")
    }

    private static func executeOn(db: OpaquePointer, sql: String) throws {
        var errorMessage: UnsafeMutablePointer<Int8>?
        if sqlite3_exec(db, sql, nil, nil, &errorMessage) != SQLITE_OK {
            let message = errorMessage.map { String(cString: $0) } ?? "unknown"
            sqlite3_free(errorMessage)
            throw CapMindError.network(message: "SQLite error: \(message)")
        }
    }
}
