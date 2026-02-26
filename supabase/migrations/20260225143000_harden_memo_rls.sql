begin;

-- Ensure RLS is enabled and enforced.
alter table if exists public.memos enable row level security;
alter table if exists public.memo_images enable row level security;
alter table if exists public.memos force row level security;
alter table if exists public.memo_images force row level security;

-- Keep memo image bucket private.
update storage.buckets
set public = false
where id = 'memo-images';

-- Restrict direct table access for anon and grant explicit authenticated access.
revoke all on table public.memos from anon;
revoke all on table public.memo_images from anon;

grant select, insert, update on table public.memos to authenticated;
grant select, insert, update, delete on table public.memo_images to authenticated;

-- Recreate memos policies with explicit authenticated role.
drop policy if exists "Users can view their own memos" on public.memos;
drop policy if exists "Users can insert their own memos" on public.memos;
drop policy if exists "Users can update their own memos" on public.memos;
drop policy if exists memos_select_own on public.memos;
drop policy if exists memos_insert_own on public.memos;
drop policy if exists memos_update_own on public.memos;

create policy memos_select_own
on public.memos
for select
to authenticated
using (auth.uid() = user_id);

create policy memos_insert_own
on public.memos
for insert
to authenticated
with check (auth.uid() = user_id);

create policy memos_update_own
on public.memos
for update
to authenticated
using (auth.uid() = user_id)
with check (auth.uid() = user_id);

-- Recreate memo_images policies with explicit authenticated role.
drop policy if exists "Users can view their memo images" on public.memo_images;
drop policy if exists "Users can insert their memo images" on public.memo_images;
drop policy if exists "Users can update their memo images" on public.memo_images;
drop policy if exists "Users can delete their memo images" on public.memo_images;
drop policy if exists memo_images_select_own on public.memo_images;
drop policy if exists memo_images_insert_own on public.memo_images;
drop policy if exists memo_images_update_own on public.memo_images;
drop policy if exists memo_images_delete_own on public.memo_images;

create policy memo_images_select_own
on public.memo_images
for select
to authenticated
using (
  exists (
    select 1
    from public.memos m
    where m.id = memo_images.memo_id
      and m.user_id = auth.uid()
  )
);

create policy memo_images_insert_own
on public.memo_images
for insert
to authenticated
with check (
  exists (
    select 1
    from public.memos m
    where m.id = memo_images.memo_id
      and m.user_id = auth.uid()
  )
);

create policy memo_images_update_own
on public.memo_images
for update
to authenticated
using (
  exists (
    select 1
    from public.memos m
    where m.id = memo_images.memo_id
      and m.user_id = auth.uid()
  )
)
with check (
  exists (
    select 1
    from public.memos m
    where m.id = memo_images.memo_id
      and m.user_id = auth.uid()
  )
);

create policy memo_images_delete_own
on public.memo_images
for delete
to authenticated
using (
  exists (
    select 1
    from public.memos m
    where m.id = memo_images.memo_id
      and m.user_id = auth.uid()
  )
);

-- Recreate storage policies with explicit authenticated role.
drop policy if exists "Users can view own memo images" on storage.objects;
drop policy if exists "Users can upload memo images" on storage.objects;
drop policy if exists "Users can delete own memo images" on storage.objects;
drop policy if exists storage_memo_images_select_own on storage.objects;
drop policy if exists storage_memo_images_insert_own on storage.objects;
drop policy if exists storage_memo_images_delete_own on storage.objects;

create policy storage_memo_images_select_own
on storage.objects
for select
to authenticated
using (
  bucket_id = 'memo-images'
  and auth.uid()::text = (storage.foldername(name))[1]
);

create policy storage_memo_images_insert_own
on storage.objects
for insert
to authenticated
with check (
  bucket_id = 'memo-images'
  and auth.uid()::text = (storage.foldername(name))[1]
);

create policy storage_memo_images_delete_own
on storage.objects
for delete
to authenticated
using (
  bucket_id = 'memo-images'
  and auth.uid()::text = (storage.foldername(name))[1]
);

-- Helpful indexes for current query patterns.
create index if not exists memos_user_id_created_at_idx
  on public.memos (user_id, created_at desc);

create index if not exists memos_user_id_deleted_at_idx
  on public.memos (user_id, deleted_at);

create index if not exists memo_images_memo_id_sort_order_idx
  on public.memo_images (memo_id, sort_order);

commit;
