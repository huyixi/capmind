# CapMind Apple (iOS/macOS)

Native Apple scaffold for CapMind with architecture aligned to the web app:

- `CapMindCore`: domain models, repository protocols, sync contracts, outbox use case.
- `CapMindData`: in-memory repositories, Supabase adapter interfaces, sync engine.
- `CapMindFeatures`: MVVM state and view models for auth, memo list, composer, and search.
- `CapMindUI`: SwiftUI screens and reusable views.
- `CapMindApp`: app entrypoint and dependency wiring.

## Web -> Native route mapping

- `/login` -> `AuthView` (sign-in mode)
- `/signup` -> `AuthView` (sign-up mode)
- `/` -> `MemoHomeView`
- `/auth/callback` -> handled by Auth repository/session restore adapter
- Trash toggle + search/composer sheets are mirrored in `MemoHomeView`

## V1 scope implemented

- Login/sign-up/session restore view model flow.
- Memo list pagination.
- Create/edit/delete/restore memo flow.
- Offline outbox queue + flush sync use case (SQLite-backed by default, with in-memory fallback).
- Search flow (online-only).
- Version-based conflict path in sync use case.

## Supabase integration status

`supabase-swift` is integrated with concrete live clients:

- `LiveSupabaseAuthClient`
- `LiveSupabaseMemoClient`
- `LiveSupabaseStorageClient`

By default, `CapMindRootView` will auto-switch to live Supabase mode when environment variables are present:

- `SUPABASE_URL` (or `NEXT_PUBLIC_SUPABASE_URL`)
- `SUPABASE_ANON_KEY` (or `NEXT_PUBLIC_SUPABASE_ANON_KEY`)

If these are missing, the app falls back to in-memory demo mode.

## Run tests

```bash
cd apps/apple
swift test
```
