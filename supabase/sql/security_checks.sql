-- Security checks for memo ownership isolation.
-- Fails with an exception when any check is false.

drop table if exists _security_checks;
create temporary table _security_checks (
  check_name text primary key,
  ok boolean not null
);

insert into _security_checks (check_name, ok)
values
  (
    'memos_rls_enabled',
    coalesce((
      select c.relrowsecurity
      from pg_class c
      join pg_namespace n on n.oid = c.relnamespace
      where n.nspname = 'public' and c.relname = 'memos'
    ), false)
  ),
  (
    'memos_force_rls',
    coalesce((
      select c.relforcerowsecurity
      from pg_class c
      join pg_namespace n on n.oid = c.relnamespace
      where n.nspname = 'public' and c.relname = 'memos'
    ), false)
  ),
  (
    'memo_images_rls_enabled',
    coalesce((
      select c.relrowsecurity
      from pg_class c
      join pg_namespace n on n.oid = c.relnamespace
      where n.nspname = 'public' and c.relname = 'memo_images'
    ), false)
  ),
  (
    'memo_images_force_rls',
    coalesce((
      select c.relforcerowsecurity
      from pg_class c
      join pg_namespace n on n.oid = c.relnamespace
      where n.nspname = 'public' and c.relname = 'memo_images'
    ), false)
  ),
  (
    'memo_images_bucket_private',
    coalesce((
      select not b.public
      from storage.buckets b
      where b.id = 'memo-images'
    ), false)
  ),
  (
    'anon_no_select_memos',
    not has_table_privilege('anon', 'public.memos', 'select')
  ),
  (
    'anon_no_insert_memos',
    not has_table_privilege('anon', 'public.memos', 'insert')
  ),
  (
    'anon_no_update_memos',
    not has_table_privilege('anon', 'public.memos', 'update')
  ),
  (
    'anon_no_select_memo_images',
    not has_table_privilege('anon', 'public.memo_images', 'select')
  ),
  (
    'anon_no_insert_memo_images',
    not has_table_privilege('anon', 'public.memo_images', 'insert')
  ),
  (
    'anon_no_update_memo_images',
    not has_table_privilege('anon', 'public.memo_images', 'update')
  ),
  (
    'anon_no_delete_memo_images',
    not has_table_privilege('anon', 'public.memo_images', 'delete')
  ),
  (
    'auth_has_select_memos',
    has_table_privilege('authenticated', 'public.memos', 'select')
  ),
  (
    'auth_has_insert_memos',
    has_table_privilege('authenticated', 'public.memos', 'insert')
  ),
  (
    'auth_has_update_memos',
    has_table_privilege('authenticated', 'public.memos', 'update')
  ),
  (
    'auth_has_select_memo_images',
    has_table_privilege('authenticated', 'public.memo_images', 'select')
  ),
  (
    'auth_has_insert_memo_images',
    has_table_privilege('authenticated', 'public.memo_images', 'insert')
  ),
  (
    'auth_has_update_memo_images',
    has_table_privilege('authenticated', 'public.memo_images', 'update')
  ),
  (
    'auth_has_delete_memo_images',
    has_table_privilege('authenticated', 'public.memo_images', 'delete')
  ),
  (
    'policy_memos_select_own_exists',
    exists (
      select 1
      from pg_policies p
      where p.schemaname = 'public'
        and p.tablename = 'memos'
        and p.policyname = 'memos_select_own'
        and exists (select 1 from unnest(p.roles) r where r = 'authenticated')
        and not exists (select 1 from unnest(p.roles) r where r = 'public')
    )
  ),
  (
    'policy_memos_insert_own_exists',
    exists (
      select 1
      from pg_policies p
      where p.schemaname = 'public'
        and p.tablename = 'memos'
        and p.policyname = 'memos_insert_own'
        and exists (select 1 from unnest(p.roles) r where r = 'authenticated')
        and not exists (select 1 from unnest(p.roles) r where r = 'public')
    )
  ),
  (
    'policy_memos_update_own_exists',
    exists (
      select 1
      from pg_policies p
      where p.schemaname = 'public'
        and p.tablename = 'memos'
        and p.policyname = 'memos_update_own'
        and exists (select 1 from unnest(p.roles) r where r = 'authenticated')
        and not exists (select 1 from unnest(p.roles) r where r = 'public')
    )
  ),
  (
    'policy_memos_no_delete_policy',
    not exists (
      select 1
      from pg_policies p
      where p.schemaname = 'public'
        and p.tablename = 'memos'
        and p.cmd = 'DELETE'
    )
  ),
  (
    'policy_memo_images_select_own_exists',
    exists (
      select 1
      from pg_policies p
      where p.schemaname = 'public'
        and p.tablename = 'memo_images'
        and p.policyname = 'memo_images_select_own'
        and exists (select 1 from unnest(p.roles) r where r = 'authenticated')
        and not exists (select 1 from unnest(p.roles) r where r = 'public')
    )
  ),
  (
    'policy_memo_images_insert_own_exists',
    exists (
      select 1
      from pg_policies p
      where p.schemaname = 'public'
        and p.tablename = 'memo_images'
        and p.policyname = 'memo_images_insert_own'
        and exists (select 1 from unnest(p.roles) r where r = 'authenticated')
        and not exists (select 1 from unnest(p.roles) r where r = 'public')
    )
  ),
  (
    'policy_memo_images_update_own_exists',
    exists (
      select 1
      from pg_policies p
      where p.schemaname = 'public'
        and p.tablename = 'memo_images'
        and p.policyname = 'memo_images_update_own'
        and exists (select 1 from unnest(p.roles) r where r = 'authenticated')
        and not exists (select 1 from unnest(p.roles) r where r = 'public')
        and p.with_check is not null
    )
  ),
  (
    'policy_memo_images_delete_own_exists',
    exists (
      select 1
      from pg_policies p
      where p.schemaname = 'public'
        and p.tablename = 'memo_images'
        and p.policyname = 'memo_images_delete_own'
        and exists (select 1 from unnest(p.roles) r where r = 'authenticated')
        and not exists (select 1 from unnest(p.roles) r where r = 'public')
    )
  ),
  (
    'policy_storage_select_own_exists',
    exists (
      select 1
      from pg_policies p
      where p.schemaname = 'storage'
        and p.tablename = 'objects'
        and p.policyname = 'storage_memo_images_select_own'
        and exists (select 1 from unnest(p.roles) r where r = 'authenticated')
        and not exists (select 1 from unnest(p.roles) r where r = 'public')
    )
  ),
  (
    'policy_storage_insert_own_exists',
    exists (
      select 1
      from pg_policies p
      where p.schemaname = 'storage'
        and p.tablename = 'objects'
        and p.policyname = 'storage_memo_images_insert_own'
        and exists (select 1 from unnest(p.roles) r where r = 'authenticated')
        and not exists (select 1 from unnest(p.roles) r where r = 'public')
    )
  ),
  (
    'policy_storage_delete_own_exists',
    exists (
      select 1
      from pg_policies p
      where p.schemaname = 'storage'
        and p.tablename = 'objects'
        and p.policyname = 'storage_memo_images_delete_own'
        and exists (select 1 from unnest(p.roles) r where r = 'authenticated')
        and not exists (select 1 from unnest(p.roles) r where r = 'public')
    )
  ),
  (
    'function_increment_numeric_string_exists',
    exists (
      select 1
      from pg_proc p
      join pg_namespace n on n.oid = p.pronamespace
      where n.nspname = 'public'
        and p.proname = 'increment_numeric_string'
        and p.proargtypes = '25'::oidvector
    )
  ),
  (
    'function_memo_update_resolve_conflict_exists',
    exists (
      select 1
      from pg_proc p
      join pg_namespace n on n.oid = p.pronamespace
      where n.nspname = 'public'
        and p.proname = 'memo_update_resolve_conflict'
        and p.proargtypes = '2950 25 25 1009'::oidvector
    )
  ),
  (
    'function_memo_delete_resolve_conflict_exists',
    exists (
      select 1
      from pg_proc p
      join pg_namespace n on n.oid = p.pronamespace
      where n.nspname = 'public'
        and p.proname = 'memo_delete_resolve_conflict'
        and p.proargtypes = '2950 25 1184'::oidvector
    )
  ),
  (
    'function_memo_restore_resolve_conflict_exists',
    exists (
      select 1
      from pg_proc p
      join pg_namespace n on n.oid = p.pronamespace
      where n.nspname = 'public'
        and p.proname = 'memo_restore_resolve_conflict'
        and p.proargtypes = '2950 25 1184'::oidvector
    )
  ),
  (
    'function_memo_update_resolve_conflict_security_invoker',
    coalesce((
      select not p.prosecdef
      from pg_proc p
      join pg_namespace n on n.oid = p.pronamespace
      where n.nspname = 'public'
        and p.proname = 'memo_update_resolve_conflict'
        and p.proargtypes = '2950 25 25 1009'::oidvector
    ), false)
  ),
  (
    'function_memo_delete_resolve_conflict_security_invoker',
    coalesce((
      select not p.prosecdef
      from pg_proc p
      join pg_namespace n on n.oid = p.pronamespace
      where n.nspname = 'public'
        and p.proname = 'memo_delete_resolve_conflict'
        and p.proargtypes = '2950 25 1184'::oidvector
    ), false)
  ),
  (
    'function_memo_restore_resolve_conflict_security_invoker',
    coalesce((
      select not p.prosecdef
      from pg_proc p
      join pg_namespace n on n.oid = p.pronamespace
      where n.nspname = 'public'
        and p.proname = 'memo_restore_resolve_conflict'
        and p.proargtypes = '2950 25 1184'::oidvector
    ), false)
  ),
  (
    'anon_no_execute_memo_update_resolve_conflict',
    not has_function_privilege(
      'anon',
      'public.memo_update_resolve_conflict(uuid, text, text, text[])',
      'execute'
    )
  ),
  (
    'anon_no_execute_memo_delete_resolve_conflict',
    not has_function_privilege(
      'anon',
      'public.memo_delete_resolve_conflict(uuid, text, timestamp with time zone)',
      'execute'
    )
  ),
  (
    'anon_no_execute_memo_restore_resolve_conflict',
    not has_function_privilege(
      'anon',
      'public.memo_restore_resolve_conflict(uuid, text, timestamp with time zone)',
      'execute'
    )
  ),
  (
    'auth_has_execute_memo_update_resolve_conflict',
    has_function_privilege(
      'authenticated',
      'public.memo_update_resolve_conflict(uuid, text, text, text[])',
      'execute'
    )
  ),
  (
    'auth_has_execute_memo_delete_resolve_conflict',
    has_function_privilege(
      'authenticated',
      'public.memo_delete_resolve_conflict(uuid, text, timestamp with time zone)',
      'execute'
    )
  ),
  (
    'auth_has_execute_memo_restore_resolve_conflict',
    has_function_privilege(
      'authenticated',
      'public.memo_restore_resolve_conflict(uuid, text, timestamp with time zone)',
      'execute'
    )
  ),
  (
    'index_memos_user_id_created_at_idx_exists',
    exists (
      select 1
      from pg_indexes
      where schemaname = 'public'
        and indexname = 'memos_user_id_created_at_idx'
    )
  ),
  (
    'index_memos_user_id_deleted_at_idx_exists',
    exists (
      select 1
      from pg_indexes
      where schemaname = 'public'
        and indexname = 'memos_user_id_deleted_at_idx'
    )
  ),
  (
    'index_memo_images_memo_id_sort_order_idx_exists',
    exists (
      select 1
      from pg_indexes
      where schemaname = 'public'
        and indexname = 'memo_images_memo_id_sort_order_idx'
    )
  );

select check_name, ok
from _security_checks
order by check_name;

do $$
declare
  fail_count int;
  fail_list text;
begin
  select count(*), coalesce(string_agg(check_name, ', ' order by check_name), '')
  into fail_count, fail_list
  from _security_checks
  where not ok;

  if fail_count > 0 then
    raise exception 'DB security checks failed (%): %', fail_count, fail_list;
  end if;
end $$;
