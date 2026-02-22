import { openDB, type DBSchema, type IDBPDatabase } from "idb";

const DB_NAME = "cap-web-offline";
const DB_VERSION = 1;

export type OutboxItem =
  | {
      id?: number;
      type: "create";
      clientId: string;
      text: string;
      files: File[];
      createdAt: string;
      updatedAt: string;
    }
  | {
      id?: number;
      type: "update";
      memoId: string;
      text: string;
      updatedAt: string;
      expectedVersion: string;
    }
  | {
      id?: number;
      type: "delete";
      memoId: string;
      deletedAt: string;
      expectedVersion: string;
    }
  | {
      id?: number;
      type: "restore";
      memoId: string;
      restoredAt: string;
      expectedVersion: string;
    };

type OutboxRecord = OutboxItem & { clientId?: string | null; memoId?: string | null };

interface MemoOutboxDB extends DBSchema {
  outbox: {
    key: number;
    value: OutboxRecord;
    indexes: { "by-client-id": string };
  };
}

let dbPromise: Promise<IDBPDatabase<MemoOutboxDB>> | null = null;

const getDbPromise = () => {
  if (typeof indexedDB === "undefined") {
    return null;
  }
  if (!dbPromise) {
    dbPromise = openDB<MemoOutboxDB>(DB_NAME, DB_VERSION, {
      upgrade(db) {
        if (!db.objectStoreNames.contains("outbox")) {
          const store = db.createObjectStore("outbox", {
            keyPath: "id",
            autoIncrement: true,
          });
          store.createIndex("by-client-id", "clientId");
        }
      },
    });
  }
  return dbPromise;
};

export async function enqueueCreate(payload: {
  clientId: string;
  text: string;
  files: File[];
  createdAt: string;
  updatedAt: string;
}): Promise<number | undefined> {
  const dbPromise = getDbPromise();
  if (!dbPromise) return undefined;
  const db = await dbPromise;
  return db.add("outbox", {
    type: "create",
    clientId: payload.clientId,
    text: payload.text,
    files: payload.files,
    createdAt: payload.createdAt,
    updatedAt: payload.updatedAt,
  });
}

export async function updatePendingCreate(
  clientId: string,
  patch: { text: string; updatedAt: string },
): Promise<boolean> {
  const dbPromise = getDbPromise();
  if (!dbPromise) return false;
  const db = await dbPromise;
  const tx = db.transaction("outbox", "readwrite");
  const existing = await tx.store.index("by-client-id").get(clientId);
  if (!existing || existing.type !== "create") {
    await tx.done;
    return false;
  }
  await tx.store.put({ ...existing, ...patch });
  await tx.done;
  return true;
}

export async function removePendingCreate(clientId: string): Promise<void> {
  const dbPromise = getDbPromise();
  if (!dbPromise) return;
  const db = await dbPromise;
  const tx = db.transaction("outbox", "readwrite");
  const existing = await tx.store.index("by-client-id").get(clientId);
  if (existing?.id !== undefined) {
    await tx.store.delete(existing.id);
  }
  await tx.done;
}

export async function enqueueUpdate(payload: {
  memoId: string;
  text: string;
  updatedAt: string;
  expectedVersion: string;
}): Promise<number | undefined> {
  const dbPromise = getDbPromise();
  if (!dbPromise) return undefined;
  const db = await dbPromise;
  return db.add("outbox", {
    type: "update",
    memoId: payload.memoId,
    text: payload.text,
    updatedAt: payload.updatedAt,
    expectedVersion: payload.expectedVersion,
  });
}

export async function enqueueDelete(payload: {
  memoId: string;
  deletedAt: string;
  expectedVersion: string;
}): Promise<number | undefined> {
  const dbPromise = getDbPromise();
  if (!dbPromise) return undefined;
  const db = await dbPromise;
  return db.add("outbox", {
    type: "delete",
    memoId: payload.memoId,
    deletedAt: payload.deletedAt,
    expectedVersion: payload.expectedVersion,
  });
}

export async function enqueueRestore(payload: {
  memoId: string;
  restoredAt: string;
  expectedVersion: string;
}): Promise<number | undefined> {
  const dbPromise = getDbPromise();
  if (!dbPromise) return undefined;
  const db = await dbPromise;
  return db.add("outbox", {
    type: "restore",
    memoId: payload.memoId,
    restoredAt: payload.restoredAt,
    expectedVersion: payload.expectedVersion,
  });
}

export async function getOutboxItems(): Promise<OutboxRecord[]> {
  const dbPromise = getDbPromise();
  if (!dbPromise) return [];
  const db = await dbPromise;
  const items = await db.getAll("outbox");
  return items.sort((a, b) => (a.id ?? 0) - (b.id ?? 0));
}

export async function removeOutboxItem(id: number): Promise<void> {
  const dbPromise = getDbPromise();
  if (!dbPromise) return;
  const db = await dbPromise;
  await db.delete("outbox", id);
}
