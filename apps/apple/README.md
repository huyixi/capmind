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
- Offline outbox queue + flush sync use case.
- Search flow (online-only).
- Version-based conflict path in sync use case.

## Supabase integration status

The package uses adapter protocols (`Supabase*ClientProtocol`) so upper layers are stable.
`UnsupportedSupabase*Client` is the default placeholder and should be replaced by concrete Supabase SDK clients.

## Run tests

```bash
cd apps/apple
swift test
```
